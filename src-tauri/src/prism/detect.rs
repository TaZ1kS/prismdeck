use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::fsutil::sorted_dir_entries;
use super::types::{PrismInstall, PrismKind};

#[cfg(target_os = "linux")]
pub fn detect_installations() -> Vec<PrismInstall> {
    finalize(linux::detect())
}

#[cfg(target_os = "windows")]
pub fn detect_installations() -> Vec<PrismInstall> {
    finalize(windows::detect())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn detect_installations() -> Vec<PrismInstall> {
    Vec::new()
}

fn finalize(mut installs: Vec<PrismInstall>) -> Vec<PrismInstall> {
    installs.sort_by(|left, right| {
        right
            .valid
            .cmp(&left.valid)
            .then_with(|| right.exe.is_some().cmp(&left.exe.is_some()))
            .then_with(|| left.kind.rank().cmp(&right.kind.rank()))
            .then_with(|| left.label.cmp(&right.label))
            .then_with(|| left.data_dir.cmp(&right.data_dir))
    });
    installs
}

fn make_install(
    kind: PrismKind,
    exe: Option<PathBuf>,
    app_id: Option<String>,
    data_dir: PathBuf,
    source: &str,
) -> PrismInstall {
    let exe = exe.map(|path| path.to_string_lossy().to_string());
    let data_dir = data_dir.to_string_lossy().to_string();
    let id = format!(
        "{}|{}|{}",
        kind.slug(),
        data_dir,
        exe.as_deref().unwrap_or("")
    );

    PrismInstall {
        id,
        kind,
        label: kind.label().to_string(),
        exe,
        app_id,
        valid: Path::new(&data_dir).is_dir(),
        data_dir,
        source: source.to_string(),
        version: None,
    }
}

fn push_unique(out: &mut Vec<PrismInstall>, candidate: PrismInstall) {
    let existing = out.iter_mut().find(|existing| {
        existing.kind == candidate.kind
            && existing.data_dir == candidate.data_dir
            && existing.exe == candidate.exe
    });

    match existing {
        Some(existing) if candidate.valid && !existing.valid => *existing = candidate,
        Some(_) => {}
        None => out.push(candidate),
    }
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn find_in_path(names: &[&str]) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        for name in names {
            let candidate = directory.join(name);
            if is_executable(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn dedupe_exe_candidates(candidates: Vec<(PathBuf, &'static str)>) -> Vec<(PathBuf, &'static str)> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    for (path, source) in candidates {
        let key = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if seen.insert(key) {
            result.push((path, source));
        }
    }
    result
}

fn portable_data_for(exe: &Path) -> Option<PathBuf> {
    let directory = exe.parent()?;
    let user_data = directory.join("UserData");
    if user_data.is_dir() {
        return Some(user_data);
    }
    if directory.join("portable.txt").is_file() {
        return Some(directory.to_path_buf());
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;

    pub fn detect() -> Vec<PrismInstall> {
        let mut installs = Vec::new();

        let home = env_path("HOME");
        let data_home = env_path("XDG_DATA_HOME")
            .or_else(|| home.as_ref().map(|home| home.join(".local/share")));
        let default_data = data_home
            .map(|base| base.join("PrismLauncher"))
            .unwrap_or_else(|| PathBuf::from("PrismLauncher"));
        let default_data_string = default_data.to_string_lossy().to_string();

        let mut exes: Vec<(PathBuf, &'static str)> = Vec::new();

        if let Some(path) = find_in_path(&["prismlauncher"]) {
            exes.push((path, "PATH"));
        }

        for path in [
            "/usr/bin/prismlauncher",
            "/usr/local/bin/prismlauncher",
            "/opt/prismlauncher/prismlauncher",
            "/opt/PrismLauncher/prismlauncher",
            "/snap/bin/prismlauncher",
        ] {
            let path = PathBuf::from(path);
            if is_executable(&path) {
                exes.push((path, "system path"));
            }
        }

        if let Some(home) = &home {
            let local_bin = home.join(".local/bin/prismlauncher");
            if is_executable(&local_bin) {
                exes.push((local_bin, "user path"));
            }

            for folder in ["Applications", "AppImages"] {
                for entry in sorted_dir_entries(&home.join(folder)) {
                    let path = entry.path();
                    if entry.file_type().map(|kind| kind.is_file()).unwrap_or(false)
                        && is_appimage(&path)
                        && looks_like_prism(&path)
                        && is_executable(&path)
                    {
                        exes.push((path, "AppImage folder"));
                    }
                }
            }
        }

        for (exe, source) in dedupe_exe_candidates(exes) {
            let portable = portable_data_for(&exe);
            let kind = if portable.is_some() {
                PrismKind::Portable
            } else if is_appimage(&exe) {
                PrismKind::AppImage
            } else {
                PrismKind::Native
            };
            let data_dir = portable.unwrap_or_else(|| default_data.clone());
            push_unique(
                &mut installs,
                make_install(kind, Some(exe), None, data_dir, source),
            );
        }

        if let Some(data_dir) = env_path("PRISMLAUNCHER_DATA_DIR") {
            let exe = installs
                .iter()
                .find(|install| install.exe.is_some() && install.kind == PrismKind::Native)
                .and_then(|install| install.exe.as_ref())
                .map(PathBuf::from);
            let kind = if exe.is_some() {
                PrismKind::Native
            } else {
                PrismKind::DataOnly
            };
            push_unique(
                &mut installs,
                make_install(kind, exe, None, data_dir, "PRISMLAUNCHER_DATA_DIR"),
            );
        }

        if let Some(home) = &home {
            let flatpak_data = home
                .join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher");
            if flatpak_data.is_dir() {
                let flatpak = find_in_path(&["flatpak"]);
                push_unique(
                    &mut installs,
                    make_install(
                        PrismKind::Flatpak,
                        flatpak,
                        Some("org.prismlauncher.PrismLauncher".to_string()),
                        flatpak_data,
                        "Flatpak data dir",
                    ),
                );
            }
        }

        if default_data.is_dir()
            && installs
                .iter()
                .all(|install| install.data_dir != default_data_string)
        {
            push_unique(
                &mut installs,
                make_install(
                    PrismKind::DataOnly,
                    None,
                    None,
                    default_data,
                    "default data dir",
                ),
            );
        }

        installs
    }

    fn is_appimage(path: &Path) -> bool {
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("AppImage"))
            .unwrap_or(false)
    }

    fn looks_like_prism(path: &Path) -> bool {
        path.file_name()
            .map(|name| name.to_string_lossy().to_ascii_lowercase().contains("prism"))
            .unwrap_or(false)
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;

    pub fn detect() -> Vec<PrismInstall> {
        let mut installs = Vec::new();

        let home = env_path("USERPROFILE");
        let appdata = env_path("APPDATA");
        let default_data = appdata
            .as_ref()
            .map(|base| base.join("PrismLauncher"))
            .unwrap_or_else(|| PathBuf::from("PrismLauncher"));
        let default_data_string = default_data.to_string_lossy().to_string();

        let mut exes: Vec<(PathBuf, &'static str)> = Vec::new();

        if let Some(path) = find_in_path(&["prismlauncher.exe", "prismlauncher"]) {
            exes.push((path, "PATH"));
        }

        if let Some(local) = env_path("LOCALAPPDATA") {
            let path = local.join("Programs/PrismLauncher/prismlauncher.exe");
            if is_executable(&path) {
                exes.push((path, "LOCALAPPDATA"));
            }
        }

        for key in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(base) = env_path(key) {
                let path = base.join("PrismLauncher/prismlauncher.exe");
                if is_executable(&path) {
                    exes.push((path, "Program Files"));
                }
            }
        }

        if let Some(home) = &home {
            let path = home.join("scoop/apps/prismlauncher/current/prismlauncher.exe");
            if is_executable(&path) {
                exes.push((path, "Scoop"));
            }
        }

        for path in registry_exe_paths() {
            if is_executable(&path) {
                exes.push((path, "registry"));
            }
        }

        for (exe, source) in dedupe_exe_candidates(exes) {
            let portable = portable_data_for(&exe);
            let kind = if portable.is_some() {
                PrismKind::Portable
            } else {
                PrismKind::Native
            };
            let data_dir = portable.unwrap_or_else(|| default_data.clone());
            push_unique(
                &mut installs,
                make_install(kind, Some(exe), None, data_dir, source),
            );
        }

        if let (Some(home), Some(scoop_exe)) = (
            &home,
            is_executable(
                &home.join("scoop/apps/prismlauncher/current/prismlauncher.exe"),
            )
            .then(|| home.join("scoop/apps/prismlauncher/current/prismlauncher.exe")),
        ) {
            let scoop_data = home.join("scoop/persist/prismlauncher");
            if scoop_data.is_dir() {
                push_unique(
                    &mut installs,
                    make_install(
                        PrismKind::Portable,
                        Some(scoop_exe),
                        None,
                        scoop_data,
                        "Scoop persist",
                    ),
                );
            }
        }

        if let Some(data_dir) = env_path("PRISMLAUNCHER_DATA_DIR") {
            let exe = installs
                .iter()
                .find(|install| install.exe.is_some() && install.kind == PrismKind::Native)
                .and_then(|install| install.exe.as_ref())
                .map(PathBuf::from);
            let kind = if exe.is_some() {
                PrismKind::Native
            } else {
                PrismKind::DataOnly
            };
            push_unique(
                &mut installs,
                make_install(kind, exe, None, data_dir, "PRISMLAUNCHER_DATA_DIR"),
            );
        }

        if default_data.is_dir()
            && installs
                .iter()
                .all(|install| install.data_dir != default_data_string)
        {
            push_unique(
                &mut installs,
                make_install(
                    PrismKind::DataOnly,
                    None,
                    None,
                    default_data,
                    "default data dir",
                ),
            );
        }

        installs
    }

    fn registry_exe_paths() -> Vec<PathBuf> {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
        use winreg::RegKey;

        let mut paths = Vec::new();
        for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
            for subkey in [
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
                r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
            ] {
                let Ok(root) = RegKey::predef(hive).open_subkey_with_flags(subkey, KEY_READ)
                else {
                    continue;
                };
                for name in root.enum_keys().flatten() {
                    let Ok(entry) = root.open_subkey_with_flags(&name, KEY_READ) else {
                        continue;
                    };
                    let display_name: String = entry.get_value("DisplayName").unwrap_or_default();
                    if !display_name.to_ascii_lowercase().contains("prism") {
                        continue;
                    }

                    if let Ok(location) = entry.get_value::<String, _>("InstallLocation") {
                        let exe = PathBuf::from(location).join("prismlauncher.exe");
                        if exe.is_file() {
                            paths.push(exe);
                        }
                    }

                    if let Ok(icon) = entry.get_value::<String, _>("DisplayIcon") {
                        let raw = icon.split(',').next().unwrap_or("").trim().trim_matches('"');
                        let icon_path = PathBuf::from(raw);
                        if icon_path.is_file() {
                            let exe = if icon_path
                                .extension()
                                .map(|ext| ext.eq_ignore_ascii_case("exe"))
                                .unwrap_or(false)
                            {
                                icon_path
                            } else if let Some(parent) = icon_path.parent() {
                                parent.join("prismlauncher.exe")
                            } else {
                                continue;
                            };
                            if exe.is_file() {
                                paths.push(exe);
                            }
                        }
                    }
                }
            }
        }
        paths
    }
}
