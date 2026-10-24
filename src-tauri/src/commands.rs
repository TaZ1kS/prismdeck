use std::fmt::Display;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::launch::{self, LaunchOptions, LaunchRun, RunInfo, RunLogs};
use crate::prism;
use crate::prism::types::{PrismInstall, PrismSnapshot};
use crate::state::AppState;
use crate::theme::{self, ThemeCss, ThemeInfo};

#[derive(Debug, Serialize)]
pub struct AppInfo {
    name: &'static str,
    version: &'static str,
    platform: &'static str,
    arch: &'static str,
    debug: bool,
    demo_data_available: bool,
    demo_auto: bool,
    auto_launch: bool,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        name: "PrismDeck",
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        debug: cfg!(debug_assertions),
        demo_data_available: prism::demo_install().is_ok(),
        demo_auto: std::env::var_os("PRISMDECK_DEMO").is_some(),
        auto_launch: std::env::var_os("PRISMDECK_AUTOLAUNCH").is_some(),
    }
}

fn task_error(error: impl Display) -> String {
    format!("background task failed: {error}")
}

fn lock_error() -> String {
    "application state is unavailable".to_string()
}

/// Discover Prism installations without requiring one to exist. Runs on a
/// blocking thread because it may touch the filesystem and PATH.
#[tauri::command]
pub async fn detect_prism() -> Result<Vec<PrismInstall>, String> {
    tauri::async_runtime::spawn_blocking(prism::detect::detect_installations)
        .await
        .map_err(task_error)
}

/// Select an installation and scan it. The frontend sends back an object from
/// `detect_prism` (or a future manual picker), so all paths are validated by
/// `scan_install` before use.
#[tauri::command]
pub async fn set_prism_install(
    state: State<'_, AppState>,
    install: PrismInstall,
) -> Result<PrismSnapshot, String> {
    let snapshot = tauri::async_runtime::spawn_blocking(move || prism::scan_install(&install))
        .await
        .map_err(task_error)??;

    *state.snapshot.write().map_err(|_| lock_error())? = snapshot.clone();
    Ok(snapshot)
}

#[tauri::command]
pub async fn refresh_instances(state: State<'_, AppState>) -> Result<PrismSnapshot, String> {
    let install = {
        let snapshot = state.snapshot.read().map_err(|_| lock_error())?;
        snapshot
            .install
            .clone()
            .ok_or_else(|| "No active Prism installation".to_string())?
    };

    let snapshot = tauri::async_runtime::spawn_blocking(move || prism::scan_install(&install))
        .await
        .map_err(task_error)??;

    *state.snapshot.write().map_err(|_| lock_error())? = snapshot.clone();
    Ok(snapshot)
}

#[tauri::command]
pub fn list_instances(state: State<'_, AppState>) -> Result<PrismSnapshot, String> {
    state
        .snapshot
        .read()
        .map(|snapshot| snapshot.clone())
        .map_err(|_| lock_error())
}

/// Development helper: load the repository fixtures without Prism installed.
#[tauri::command]
pub async fn load_demo_data(state: State<'_, AppState>) -> Result<PrismSnapshot, String> {
    let snapshot = tauri::async_runtime::spawn_blocking(prism::demo_snapshot)
        .await
        .map_err(task_error)??;

    *state.snapshot.write().map_err(|_| lock_error())? = snapshot.clone();
    Ok(snapshot)
}

/// Read one icon lazily and return it as a data URL. This avoids exposing a
/// filesystem asset protocol and keeps the IPC payload small for large decks.
#[tauri::command]
pub fn get_instance_icon(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<Option<String>, String> {
    let snapshot = state.snapshot.read().map_err(|_| lock_error())?;
    let Some(instance) = snapshot
        .instances
        .iter()
        .find(|instance| instance.id == instance_id)
    else {
        return Ok(None);
    };
    let Some(path) = instance.icon_path.as_deref() else {
        return Ok(None);
    };

    Ok(prism::instance::icon_data_url(std::path::Path::new(path)))
}

/// Launch an instance through the Prism CLI. The wrapper command, if any, is
/// parsed as argv and prepended to the Prism command; no shell is involved.
#[tauri::command]
pub fn launch_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    options: Option<LaunchOptions>,
) -> Result<LaunchRun, String> {
    launch::launch(app, state, instance_id, options)
}

/// Ask the process tree to stop. A second call escalates to a force kill.
#[tauri::command]
pub fn stop_run(app: AppHandle, run_id: String) -> Result<RunInfo, String> {
    launch::stop(&app, &run_id)
}

#[tauri::command]
pub fn list_runs(app: AppHandle) -> Result<Vec<RunInfo>, String> {
    launch::list(&app)
}

#[tauri::command]
pub fn get_run_logs(app: AppHandle, run_id: String) -> Result<RunLogs, String> {
    launch::get_logs(&app, &run_id)
}

#[tauri::command]
pub fn list_themes() -> Result<Vec<ThemeInfo>, String> {
    theme::list_themes()
}

#[tauri::command]
pub fn get_theme_css(id: String) -> Result<ThemeCss, String> {
    theme::read_theme(&id)
}
