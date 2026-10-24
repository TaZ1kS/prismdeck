use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::prism::types::{PrismInstall, PrismKind};
use crate::state::AppState;

const MAX_LOG_LINES: usize = 2_000;
const MAX_LINE_CHARS: usize = 4_096;
const MAX_RUN_HISTORY: usize = 20;
const BATCH_SIZE: usize = 40;
const BATCH_WINDOW: Duration = Duration::from_millis(60);

static NEXT_RUN: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Stopping,
    Stopped,
    Exited,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunInfo {
    pub run_id: String,
    pub instance_id: String,
    pub instance_name: String,
    pub pid: u32,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub exit_code: Option<i32>,
    pub status: RunStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub stream: String,
    pub text: String,
    pub seq: u64,
}

#[derive(Debug, Clone, Serialize)]
struct LogBatch {
    run_id: String,
    lines: Vec<LogLine>,
}

#[derive(Debug, Clone, Serialize)]
struct LaunchExit {
    run_id: String,
    status: RunStatus,
    exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunLogs {
    pub info: RunInfo,
    pub lines: Vec<LogLine>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LaunchRun {
    pub run_id: String,
    pub instance_id: String,
    pub pid: u32,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LaunchOptions {
    /// Optional wrapper argv, parsed with POSIX shell quoting. It is prepended
    /// to the Prism command; no shell is ever invoked.
    pub wrapper: Option<String>,
    pub show_window: bool,
    pub offline: bool,
    pub server: Option<String>,
    pub world: Option<String>,
}

#[derive(Debug)]
pub struct RunRecord {
    pub info: RunInfo,
    pub logs: VecDeque<LogLine>,
}

#[derive(Debug, Clone)]
struct RawLine {
    stream: &'static str,
    text: String,
}

pub fn launch(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    options: Option<LaunchOptions>,
) -> Result<LaunchRun, String> {
    let options = options.unwrap_or_default();

    let (install, instance_name) = {
        let snapshot = state
            .snapshot
            .read()
            .map_err(|_| "application state is unavailable".to_string())?;
        let install = snapshot
            .install
            .clone()
            .ok_or_else(|| "No active Prism installation".to_string())?;
        let instance = snapshot
            .instances
            .iter()
            .find(|instance| instance.id == instance_id)
            .ok_or_else(|| format!("Instance not found: {instance_id}"))?;
        (install, instance.name.clone())
    };

    let (program, args) = build_command(&install, &instance_id, &options)?;
    let run_id = next_run_id();

    let mut command = Command::new(&program);
    command
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process(&mut command);

    let mut child = command
        .spawn()
        .map_err(|error| format!("Failed to launch {program}: {error}"))?;
    let pid = child.id();
    let info = RunInfo {
        run_id: run_id.clone(),
        instance_id: instance_id.clone(),
        instance_name,
        pid,
        started_at_ms: now_ms(),
        ended_at_ms: None,
        exit_code: None,
        status: RunStatus::Running,
    };

    {
        let mut runs = state
            .runs
            .lock()
            .map_err(|_| "run registry is unavailable".to_string())?;
        prune_runs(&mut runs);
        runs.insert(
            run_id.clone(),
            RunRecord {
                info: info.clone(),
                logs: VecDeque::new(),
            },
        );
    }

    let (tx, rx) = mpsc::channel::<RawLine>();
    let mut reader_handles = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        reader_handles.push(pump_reader(stdout, "stdout", tx.clone()));
    }
    if let Some(stderr) = child.stderr.take() {
        reader_handles.push(pump_reader(stderr, "stderr", tx.clone()));
    }
    drop(tx);

    let writer_handle = spawn_writer(app.clone(), run_id.clone(), rx);
    spawn_waiter(
        child,
        app.clone(),
        run_id.clone(),
        reader_handles,
        writer_handle,
    );

    let _ = app.emit("launch-started", info.clone());
    log::info!(
        "launched instance {instance_id} as run {run_id} (pid {pid}) via {program}"
    );

    Ok(LaunchRun {
        run_id,
        instance_id,
        pid,
    })
}

fn build_command(
    install: &PrismInstall,
    instance_id: &str,
    options: &LaunchOptions,
) -> Result<(String, Vec<String>), String> {
    let (program, mut args): (String, Vec<String>) = match install.kind {
        PrismKind::Flatpak => {
            let flatpak = install
                .exe
                .clone()
                .ok_or_else(|| "Flatpak executable not found".to_string())?;
            let app_id = install
                .app_id
                .clone()
                .ok_or_else(|| "Flatpak application id is missing".to_string())?;
            (
                flatpak,
                vec![
                    "run".to_string(),
                    "--command=prismlauncher".to_string(),
                    app_id,
                ],
            )
        }
        _ => {
            let exe = install.exe.clone().ok_or_else(|| {
                "No Prism executable is configured. This source is browse-only."
                    .to_string()
            })?;
            (
                exe,
                vec!["--dir".to_string(), install.data_dir.clone()],
            )
        }
    };

    args.push("--launch".to_string());
    args.push(instance_id.to_string());

    if options.show_window {
        args.push("--show-window".to_string());
    }
    if options.offline {
        args.push("--offline".to_string());
    }
    if let Some(server) = clean_option(&options.server) {
        args.push("--server".to_string());
        args.push(server);
    }
    if let Some(world) = clean_option(&options.world) {
        args.push("--world".to_string());
        args.push(world);
    }

    if let Some(wrapper) = clean_option(&options.wrapper) {
        let mut parts = shlex::split(&wrapper).ok_or_else(|| {
            format!("Invalid wrapper command (unbalanced quotes): {wrapper}")
        })?;
        if parts.is_empty() || parts[0].is_empty() {
            return Err("Wrapper command is empty".to_string());
        }
        parts.push(program);
        let program = parts.remove(0);
        parts.extend(args);
        Ok((program, parts))
    } else {
        Ok((program, args))
    }
}

fn clean_option(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn next_run_id() -> String {
    let sequence = NEXT_RUN.fetch_add(1, Ordering::Relaxed);
    format!("run-{}-{sequence}", now_ms())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn prune_runs(runs: &mut HashMap<String, RunRecord>) {
    if runs.len() < MAX_RUN_HISTORY {
        return;
    }

    let mut finished: Vec<(String, i64)> = runs
        .iter()
        .filter(|(_, record)| {
            !matches!(
                record.info.status,
                RunStatus::Running | RunStatus::Stopping
            )
        })
        .map(|(run_id, record)| (run_id.clone(), record.info.started_at_ms))
        .collect();
    finished.sort_by_key(|(_, started_at)| *started_at);

    let remove_count = runs.len().saturating_sub(MAX_RUN_HISTORY) + 1;
    for (run_id, _) in finished.into_iter().take(remove_count) {
        runs.remove(&run_id);
    }
}

fn pump_reader<R: Read + Send + 'static>(
    reader: R,
    stream: &'static str,
    sender: Sender<RawLine>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut buffer = Vec::new();

        loop {
            buffer.clear();
            match reader.read_until(b'\n', &mut buffer) {
                Ok(0) => break,
                Ok(_) => {
                    while let Some(byte) = buffer.last() {
                        if *byte == b'\n' || *byte == b'\r' {
                            buffer.pop();
                        } else {
                            break;
                        }
                    }
                    if buffer.is_empty() {
                        continue;
                    }
                    let text = truncate_line(String::from_utf8_lossy(&buffer).into_owned());
                    if sender.send(RawLine { stream, text }).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    })
}

fn truncate_line(text: String) -> String {
    if text.chars().count() <= MAX_LINE_CHARS {
        return text;
    }
    let mut truncated: String = text.chars().take(MAX_LINE_CHARS).collect();
    truncated.push('…');
    truncated
}

fn spawn_writer(
    app: AppHandle,
    run_id: String,
    receiver: Receiver<RawLine>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut batch = Vec::new();
        let mut sequence = 0_u64;
        let mut last_flush = Instant::now();

        loop {
            let timeout = if batch.is_empty() {
                Duration::from_millis(200)
            } else {
                BATCH_WINDOW
            };

            match receiver.recv_timeout(timeout) {
                Ok(line) => {
                    batch.push(line);
                    if batch.len() >= BATCH_SIZE {
                        flush_batch(&app, &run_id, &mut batch, &mut sequence);
                        last_flush = Instant::now();
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    if !batch.is_empty() {
                        flush_batch(&app, &run_id, &mut batch, &mut sequence);
                        last_flush = Instant::now();
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    flush_batch(&app, &run_id, &mut batch, &mut sequence);
                    break;
                }
            }

            if !batch.is_empty() && last_flush.elapsed() >= BATCH_WINDOW {
                flush_batch(&app, &run_id, &mut batch, &mut sequence);
                last_flush = Instant::now();
            }
        }
    })
}

fn flush_batch(
    app: &AppHandle,
    run_id: &str,
    batch: &mut Vec<RawLine>,
    sequence: &mut u64,
) {
    if batch.is_empty() {
        return;
    }

    let mut lines = Vec::with_capacity(batch.len());
    for raw in batch.drain(..) {
        *sequence += 1;
        lines.push(LogLine {
            stream: raw.stream.to_string(),
            text: raw.text,
            seq: *sequence,
        });
    }

    if let Ok(mut runs) = app.state::<AppState>().runs.lock() {
        if let Some(record) = runs.get_mut(run_id) {
            for line in &lines {
                record.logs.push_back(line.clone());
            }
            while record.logs.len() > MAX_LOG_LINES {
                record.logs.pop_front();
            }
        }
    }

    let _ = app.emit(
        "launch-log",
        LogBatch {
            run_id: run_id.to_string(),
            lines,
        },
    );
}

fn spawn_waiter(
    mut child: Child,
    app: AppHandle,
    run_id: String,
    readers: Vec<JoinHandle<()>>,
    writer: JoinHandle<()>,
) {
    thread::spawn(move || {
        let wait_result = child.wait();

        for reader in readers {
            let _ = reader.join();
        }
        let _ = writer.join();

        let exit_code = wait_result.as_ref().ok().and_then(|status| status.code());
        let mut status = if wait_result.is_ok() && exit_code == Some(0) {
            RunStatus::Exited
        } else {
            RunStatus::Failed
        };

        if let Ok(mut runs) = app.state::<AppState>().runs.lock() {
            if let Some(record) = runs.get_mut(&run_id) {
                if matches!(
                    record.info.status,
                    RunStatus::Stopping | RunStatus::Stopped
                ) {
                    status = RunStatus::Stopped;
                }
                record.info.status = status;
                record.info.exit_code = exit_code;
                record.info.ended_at_ms = Some(now_ms());
            }
        }

        let _ = app.emit(
            "launch-exit",
            LaunchExit {
                run_id,
                status,
                exit_code,
            },
        );
        log::info!("run finished with status {status:?} (exit {exit_code:?})");
    });
}

pub fn stop(app: &AppHandle, run_id: &str) -> Result<RunInfo, String> {
    let (pid, force) = {
        let state = app.state::<AppState>();
        let mut runs = state
            .runs
            .lock()
            .map_err(|_| "run registry is unavailable".to_string())?;
        let record = runs
            .get_mut(run_id)
            .ok_or_else(|| format!("Run not found: {run_id}"))?;

        match record.info.status {
            RunStatus::Running => {
                record.info.status = RunStatus::Stopping;
                (record.info.pid, false)
            }
            RunStatus::Stopping => (record.info.pid, true),
            _ => return Ok(record.info.clone()),
        }
    };

    if let Err(error) = terminate_tree(pid, force) {
        if !force {
            if let Ok(mut runs) = app.state::<AppState>().runs.lock() {
                if let Some(record) = runs.get_mut(run_id) {
                    if record.info.status == RunStatus::Stopping {
                        record.info.status = RunStatus::Running;
                    }
                }
            }
        }
        return Err(error);
    }

    let state = app.state::<AppState>();
    let runs = state
        .runs
        .lock()
        .map_err(|_| "run registry is unavailable".to_string())?;
    runs.get(run_id)
        .map(|record| record.info.clone())
        .ok_or_else(|| format!("Run not found: {run_id}"))
}

pub fn list(app: &AppHandle) -> Result<Vec<RunInfo>, String> {
    let state = app.state::<AppState>();
    let runs = state
        .runs
        .lock()
        .map_err(|_| "run registry is unavailable".to_string())?;
    let mut infos: Vec<RunInfo> = runs
        .values()
        .map(|record| record.info.clone())
        .collect();
    infos.sort_by(|left, right| right.started_at_ms.cmp(&left.started_at_ms));
    Ok(infos)
}

pub fn get_logs(app: &AppHandle, run_id: &str) -> Result<RunLogs, String> {
    let state = app.state::<AppState>();
    let runs = state
        .runs
        .lock()
        .map_err(|_| "run registry is unavailable".to_string())?;
    let record = runs
        .get(run_id)
        .ok_or_else(|| format!("Run not found: {run_id}"))?;

    Ok(RunLogs {
        info: record.info.clone(),
        lines: record.logs.iter().cloned().collect(),
    })
}

#[cfg(unix)]
fn configure_process(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(windows)]
fn configure_process(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(any(unix, windows)))]
fn configure_process(_command: &mut Command) {}

#[cfg(unix)]
fn terminate_tree(pid: u32, force: bool) -> Result<(), String> {
    let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
    let result = unsafe { libc::kill(-(pid as i32), signal) };
    if result == 0 {
        return Ok(());
    }

    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(format!("Failed to stop process tree {pid}: {error}"))
    }
}

#[cfg(windows)]
fn terminate_tree(pid: u32, force: bool) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let mut command = Command::new("taskkill");
    command.args(["/PID", &pid.to_string(), "/T"]);
    if force {
        command.arg("/F");
    }
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command
        .output()
        .map_err(|error| format!("Failed to call taskkill: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            format!("taskkill failed for process tree {pid}")
        } else {
            format!("taskkill failed for process tree {pid}: {stderr}")
        })
    }
}

#[cfg(not(any(unix, windows)))]
fn terminate_tree(_pid: u32, _force: bool) -> Result<(), String> {
    Err("Stopping processes is not supported on this platform".to_string())
}

/// Keep this import used on all platforms; `Mutex` is referenced in the type
/// signature indirectly via AppState.
#[allow(dead_code)]
fn _assert_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Mutex<HashMap<String, RunRecord>>>();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install(kind: PrismKind, exe: Option<&str>, app_id: Option<&str>) -> PrismInstall {
        PrismInstall {
            id: "test".to_string(),
            kind,
            label: "test".to_string(),
            exe: exe.map(ToOwned::to_owned),
            app_id: app_id.map(ToOwned::to_owned),
            data_dir: "/data/PrismLauncher".to_string(),
            source: "test".to_string(),
            valid: true,
            version: None,
        }
    }

    fn string_args(args: Vec<String>) -> Vec<String> {
        args
    }

    #[test]
    fn native_command_uses_dir_and_launch() {
        let install = install(PrismKind::Native, Some("/usr/bin/prismlauncher"), None);
        let (program, args) =
            build_command(&install, "fabric-1.20.1", &LaunchOptions::default()).unwrap();
        assert_eq!(program, "/usr/bin/prismlauncher");
        assert_eq!(
            string_args(args),
            vec![
                "--dir",
                "/data/PrismLauncher",
                "--launch",
                "fabric-1.20.1"
            ]
        );
    }

    #[test]
    fn wrapper_is_prepended_without_a_shell() {
        let install = install(PrismKind::Native, Some("/usr/bin/prismlauncher"), None);
        let options = LaunchOptions {
            wrapper: Some("gamescope -f --".to_string()),
            ..LaunchOptions::default()
        };
        let (program, args) = build_command(&install, "atm10", &options).unwrap();
        assert_eq!(program, "gamescope");
        assert_eq!(
            string_args(args),
            vec![
                "-f",
                "--",
                "/usr/bin/prismlauncher",
                "--dir",
                "/data/PrismLauncher",
                "--launch",
                "atm10"
            ]
        );
    }

    #[test]
    fn flatpak_command_uses_host_flatpak() {
        let install = install(
            PrismKind::Flatpak,
            Some("/usr/bin/flatpak"),
            Some("org.prismlauncher.PrismLauncher"),
        );
        let (program, args) =
            build_command(&install, "vanilla-1.21.4", &LaunchOptions::default()).unwrap();
        assert_eq!(program, "/usr/bin/flatpak");
        assert_eq!(
            string_args(args),
            vec![
                "run",
                "--command=prismlauncher",
                "org.prismlauncher.PrismLauncher",
                "--launch",
                "vanilla-1.21.4"
            ]
        );
    }

    #[test]
    fn browse_only_source_refuses_to_launch() {
        let install = install(PrismKind::DataOnly, None, None);
        let error = build_command(&install, "x", &LaunchOptions::default()).unwrap_err();
        assert!(error.contains("browse-only"));
    }

    #[test]
    fn invalid_wrapper_is_rejected() {
        let install = install(PrismKind::Native, Some("/usr/bin/prismlauncher"), None);
        let options = LaunchOptions {
            wrapper: Some("gamescope 'unclosed".to_string()),
            ..LaunchOptions::default()
        };
        assert!(build_command(&install, "x", &options).is_err());
    }
}
