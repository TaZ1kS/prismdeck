use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::prism::types::PrismInstall;

/// Persistent user settings, stored as JSON in the app config directory.
/// All fields are optional/simple so a missing or partial file is fine.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// The selected Prism installation. `None` until the user picks one.
    pub install: Option<PrismInstall>,
    pub theme: Option<String>,
    pub wrapper: Option<String>,
    pub offline: bool,
    pub show_window: bool,
    pub server: Option<String>,
    pub world: Option<String>,
    /// Skin scaffold (no real behaviour yet).
    pub skin: Option<String>,
    /// Surface transparency, 0.3..=1.0 (1.0 = opaque). None = default.
    pub transparency: Option<f64>,
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|error| format!("cannot resolve config dir: {error}"))
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(config_dir(app)?.join("settings.json"))
}

pub fn load(app: &AppHandle) -> Result<Settings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(Settings::default());
    }
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read settings: {error}"))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("settings file is invalid: {error}"))
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let dir = config_dir(app)?;
    fs::create_dir_all(&dir).map_err(|error| format!("cannot create config dir: {error}"))?;
    let path = settings_path(app)?;
    let text = serde_json::to_string_pretty(settings)
        .map_err(|error| format!("cannot serialize settings: {error}"))?;
    fs::write(&path, text).map_err(|error| format!("cannot write settings: {error}"))
}

/// Persist only the selected install, keeping every other field intact.
pub fn set_install(app: &AppHandle, install: PrismInstall) -> Result<(), String> {
    let mut settings = load(app)?;
    settings.install = Some(install);
    save(app, &settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip() {
        let mut settings = Settings::default();
        settings.theme = Some("neon".to_string());
        settings.wrapper = Some("gamescope -f --".to_string());
        settings.offline = true;

        let json = serde_json::to_string(&settings).unwrap();
        let decoded: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.theme.as_deref(), Some("neon"));
        assert_eq!(decoded.wrapper.as_deref(), Some("gamescope -f --"));
        assert!(decoded.offline);
    }

    #[test]
    fn missing_file_loads_defaults() {
        let json = "{}";
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert!(settings.install.is_none());
        assert!(!settings.offline);
    }
}
