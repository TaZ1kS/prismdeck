use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
pub struct ThemeInfo {
    pub id: String,
    pub name: String,
    pub accent: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThemeCss {
    pub id: String,
    pub css: String,
}

#[derive(Debug, Clone, Serialize)]
struct ThemeChanged {
    id: String,
}

/// Resolve the themes directory. Development builds read `ui/themes` next to
/// the crate; `PRISMDECK_THEMES_DIR` overrides for tests and packaging.
pub fn themes_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("PRISMDECK_THEMES_DIR") {
        return Some(PathBuf::from(dir));
    }

    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ui/themes");
    if dev.is_dir() {
        return Some(dev);
    }

    None
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('.')
        && !id.contains('/')
        && !id.contains('\\')
        && id != ".."
        && id != "."
}

fn title_case(id: &str) -> String {
    let mut chars = id.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut out = first.to_uppercase().collect::<String>();
            out.extend(chars.map(|ch| {
                if ch == '-' || ch == '_' {
                    ' '
                } else {
                    ch
                }
            }));
            out
        }
    }
}

pub fn list_themes() -> Result<Vec<ThemeInfo>, String> {
    let dir = themes_dir().ok_or_else(|| "Theme folder was not found".to_string())?;
    let entries =
        std::fs::read_dir(&dir).map_err(|error| format!("Cannot read themes: {error}"))?;

    let mut themes = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        if !valid_id(&id) {
            continue;
        }
        let css_path = path.join("theme.css");
        if !css_path.is_file() {
            continue;
        }
        let accent = std::fs::read_to_string(&css_path)
            .ok()
            .and_then(|css| accent_from_css(&css));
        themes.push(ThemeInfo {
            name: title_case(&id),
            id,
            accent,
        });
    }

    themes.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(themes)
}

fn accent_from_css(css: &str) -> Option<String> {
    let line = css
        .lines()
        .find(|line| line.trim_start().starts_with("--pd-accent:"))?;
    let value = line.split_once(':')?.1.trim().trim_end_matches(';').trim();
    (!value.is_empty()).then(|| value.to_string())
}

pub fn read_theme(id: &str) -> Result<ThemeCss, String> {
    if !valid_id(id) {
        return Err(format!("Invalid theme id: {id}"));
    }
    let dir = themes_dir().ok_or_else(|| "Theme folder was not found".to_string())?;
    let path = dir.join(id).join("theme.css");
    let css = std::fs::read_to_string(&path)
        .map_err(|error| format!("Cannot read theme {id}: {error}"))?;
    Ok(ThemeCss {
        id: id.to_string(),
        css,
    })
}

/// Watch the themes directory and emit a `theme-changed` event (debounced)
/// whenever a `theme.css` file changes. The frontend re-fetches and re-applies
/// the active theme on receipt.
pub fn watch_themes(app: AppHandle) {
    let Some(dir) = themes_dir() else {
        log::warn!("no theme directory; live reload disabled");
        return;
    };

    let root = dir.clone();
    thread::spawn(move || {
        use notify::Watcher;

        let (sender, receiver) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();

        let mut watcher = match notify::recommended_watcher(move |event| {
            let _ = sender.send(event);
        }) {
            Ok(watcher) => watcher,
            Err(error) => {
                log::warn!("theme watcher failed to start: {error}");
                return;
            }
        };

        if watcher
            .watch(&root, notify::RecursiveMode::Recursive)
            .is_err()
        {
            log::warn!("theme watcher failed to watch {}", root.display());
            return;
        }

        let mut changed = HashSet::new();
        loop {
            match receiver.recv_timeout(Duration::from_millis(120)) {
                Ok(Ok(event)) => {
                    for path in event.paths {
                        if let Some(id) = theme_id_from_path(&root, &path) {
                            changed.insert(id);
                        }
                    }
                }
                Ok(Err(_)) => {}
                Err(_) => {
                    for id in changed.drain() {
                        log::info!("theme changed on disk: {id}");
                        let _ = app.emit("theme-changed", ThemeChanged { id });
                    }
                }
            }
        }
    });
}

fn theme_id_from_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let id = relative.components().next()?.as_os_str().to_string_lossy().to_string();
    valid_id(&id).then_some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_ids() {
        assert!(valid_id("neon"));
        assert!(!valid_id(".."));
        assert!(!valid_id("."));
        assert!(!valid_id(""));
        assert!(!valid_id("a/b"));
        assert!(!valid_id("..\\windows"));
    }

    #[test]
    fn title_cases_ids() {
        assert_eq!(title_case("neon"), "Neon");
        assert_eq!(title_case("midnight-sun"), "Midnight sun");
    }

    #[test]
    fn lists_bundled_themes() {
        let themes = list_themes().expect("repo ships a ui/themes directory");
        assert!(themes.iter().any(|theme| theme.id == "default"));
        assert!(themes.iter().any(|theme| theme.id == "neo"));

        let neon = themes.iter().find(|theme| theme.id == "neon").unwrap();
        assert_eq!(neon.accent.as_deref(), Some("#22d3ee"));

        let css = read_theme("neo").unwrap();
        assert!(css.css.contains("grid-template-areas"));
    }
}
