use std::collections::HashMap;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use serde::Deserialize;

use super::fsutil::{read_text_limited, sorted_dir_entries};
use super::types::{Instance, ManagedPack};

const MAX_CONFIG_BYTES: u64 = 2 * 1024 * 1024;
const MAX_ICON_BYTES: u64 = 2 * 1024 * 1024;
const ICON_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg", "ico"];

#[derive(Debug, Clone)]
pub struct PrismPaths {
    pub instances_dir: PathBuf,
    pub icons_dir: PathBuf,
}

/// Resolve Prism's instance and icon folders, honoring `prismlauncher.cfg`.
/// Defaults are `<data>/instances` and `<data>/icons`.
pub fn resolve_paths(data_dir: &Path) -> PrismPaths {
    let general = read_text_limited(&data_dir.join("prismlauncher.cfg"), MAX_CONFIG_BYTES)
        .map(|text| parse_general_ini(&text))
        .unwrap_or_default();

    let instances_dir = resolve_configured_path(&general, "InstancesDir", data_dir, "instances");
    let icons_dir = resolve_configured_path(&general, "IconsDir", data_dir, "icons");

    PrismPaths {
        instances_dir,
        icons_dir,
    }
}

fn resolve_configured_path(
    general: &HashMap<String, String>,
    key: &str,
    data_dir: &Path,
    fallback: &str,
) -> PathBuf {
    let configured = general
        .get(key)
        .map(|value| value.trim().trim_matches('"').trim())
        .filter(|value| !value.is_empty());

    match configured {
        Some(value) => {
            let path = PathBuf::from(value);
            if path.is_absolute() {
                path
            } else {
                data_dir.join(path)
            }
        }
        None => data_dir.join(fallback),
    }
}

/// Small tolerant parser for the subset of Qt QSettings we need.
/// Only the `[General]` section is returned; unknown sections are ignored.
pub fn parse_general_ini(text: &str) -> HashMap<String, String> {
    let mut in_general = false;
    let mut values = HashMap::new();

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') {
            let section = line
                .trim_start_matches('[')
                .split(']')
                .next()
                .unwrap_or("")
                .trim();
            in_general = section.eq_ignore_ascii_case("General");
            continue;
        }

        if in_general {
            if let Some((key, value)) = line.split_once('=') {
                values.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }

    values
}

/// Parse one instance folder. Every missing or malformed file degrades to a
/// sensible fallback; one bad instance must never abort the whole scan.
pub fn parse_instance(
    id: &str,
    instance_dir: &Path,
    icons: &HashMap<String, PathBuf>,
    group: String,
) -> Instance {
    let general = read_text_limited(&instance_dir.join("instance.cfg"), MAX_CONFIG_BYTES)
        .map(|text| parse_general_ini(&text))
        .unwrap_or_default();

    let pack = read_mmc_pack(instance_dir);
    let name = general
        .get("name")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| id.to_string());

    let mc_version = pack
        .as_ref()
        .and_then(|pack| pack.mc_version.clone())
        .or_else(|| get_non_empty(&general, &["IntendedVersion", "MinecraftVersion"]));

    let icon_key = general
        .get("iconKey")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("default")
        .to_string();

    let icon_path = resolve_icon(&icon_key, instance_dir, icons);

    Instance {
        id: id.to_string(),
        name,
        mc_version,
        loader: pack.as_ref().and_then(|pack| pack.loader.clone()),
        loader_version: pack.as_ref().and_then(|pack| pack.loader_version.clone()),
        managed_pack: read_managed_pack(&general),
        icon_key,
        group,
        notes: general.get("notes").cloned().unwrap_or_default(),
        last_launch_ms: general
            .get("lastLaunchTime")
            .and_then(|value| value.trim().parse::<i64>().ok())
            .filter(|value| *value > 0),
        total_played_s: parse_seconds(&general, "totalTimePlayed"),
        last_played_s: parse_seconds(&general, "lastTimePlayed"),
        instance_dir: instance_dir.to_string_lossy().to_string(),
        icon_path: icon_path.map(|path| path.to_string_lossy().to_string()),
    }
}

fn get_non_empty(general: &HashMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        general
            .get(*key)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn parse_seconds(general: &HashMap<String, String>, key: &str) -> i64 {
    general
        .get(key)
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(0)
        .max(0)
}

fn read_managed_pack(general: &HashMap<String, String>) -> Option<ManagedPack> {
    if !general
        .get("ManagedPack")
        .map(|value| parse_bool(value))
        .unwrap_or(false)
    {
        return None;
    }

    let kind = general
        .get("ManagedPackType")
        .cloned()
        .unwrap_or_default()
        .trim()
        .to_string();
    let id = general
        .get("ManagedPackID")
        .cloned()
        .unwrap_or_default()
        .trim()
        .to_string();
    let name = get_non_empty(general, &["ManagedPackName"])
        .or_else(|| if id.is_empty() { None } else { Some(id.clone()) })
        .unwrap_or_else(|| "Managed pack".to_string());
    let version = get_non_empty(general, &["ManagedPackVersionName", "ManagedPackVersionID"])
        .unwrap_or_else(|| "unknown".to_string());

    Some(ManagedPack {
        kind,
        id,
        name,
        version,
    })
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "on"
    )
}

#[derive(Debug, Default)]
struct PackInfo {
    mc_version: Option<String>,
    loader: Option<String>,
    loader_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawMmcPack {
    #[serde(default)]
    components: Vec<RawComponent>,
}

#[derive(Debug, Deserialize)]
struct RawComponent {
    #[serde(default)]
    uid: String,
    #[serde(default)]
    version: String,
    #[serde(default, rename = "cachedName")]
    cached_name: String,
}

fn read_mmc_pack(instance_dir: &Path) -> Option<PackInfo> {
    let text = read_text_limited(&instance_dir.join("mmc-pack.json"), MAX_CONFIG_BYTES)?;
    let raw: RawMmcPack = serde_json::from_str(&text).ok()?;

    let mut info = PackInfo::default();
    for component in raw.components {
        if component.uid == "net.minecraft" {
            info.mc_version = non_empty(component.version);
            continue;
        }

        if info.loader.is_none() {
            if let Some(label) = loader_label(&component.uid) {
                info.loader = Some(label.to_string());
                info.loader_version = non_empty(component.version).or_else(|| {
                    if component.cached_name.trim().is_empty() {
                        None
                    } else {
                        Some(component.cached_name.trim().to_string())
                    }
                });
            }
        }
    }

    Some(info)
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn loader_label(uid: &str) -> Option<&'static str> {
    match uid {
        "net.neoforged" => Some("NeoForge"),
        "net.minecraftforge" => Some("Forge"),
        "net.fabricmc.fabric-loader" => Some("Fabric"),
        "org.quiltmc.quilt-loader" => Some("Quilt"),
        "net.legacyfabric.legacy-fabric-loader" => Some("Legacy Fabric"),
        "com.mumfrey.liteloader" => Some("LiteLoader"),
        "org.multimc.legacywrapper" => Some("Legacy"),
        "optifine" | "net.optifine" => Some("OptiFine"),
        _ if uid.ends_with(".fabric-loader") => Some("Fabric"),
        _ => None,
    }
}

/// Recursively index `<data>/icons/<key>.<ext>`. Nested icon folders are
/// supported; the first deterministic match wins.
pub fn collect_icons(icons_dir: &Path) -> HashMap<String, PathBuf> {
    let mut files = Vec::new();
    walk_icons(icons_dir, 0, &mut files);
    files.sort();

    let mut icons = HashMap::new();
    for path in files {
        if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
            icons.entry(stem.to_string()).or_insert(path);
        }
    }
    icons
}

fn walk_icons(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 4 {
        return;
    }
    for entry in sorted_dir_entries(dir) {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if file_type.is_dir() {
            walk_icons(&path, depth + 1, out);
        } else if file_type.is_file() && is_icon_path(&path) {
            out.push(path);
        }
    }
}

fn is_icon_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ICON_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn resolve_icon(
    icon_key: &str,
    instance_dir: &Path,
    icons: &HashMap<String, PathBuf>,
) -> Option<PathBuf> {
    if !icon_key.is_empty() {
        if let Some(path) = icons.get(icon_key) {
            return Some(path.clone());
        }
    }

    for name in [
        "icon.png",
        "icon.jpg",
        "icon.jpeg",
        "icon.webp",
        "icon.svg",
        "icon.gif",
        "icon.ico",
    ] {
        let path = instance_dir.join(name);
        if path.is_file() {
            return Some(path);
        }
    }

    let path = instance_dir.join(".minecraft").join("icon.png");
    path.is_file().then_some(path)
}

/// Convert a local icon to a data URL. The webview never receives arbitrary
/// file paths; only image bytes that the backend chose to expose.
pub fn icon_data_url(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_ICON_BYTES {
        return None;
    }
    let mime = mime_for_path(path)?;
    let bytes = std::fs::read(path).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:{mime};base64,{encoded}"))
}

fn mime_for_path(path: &Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => Some("image/png"),
        Some("jpg" | "jpeg") => Some("image/jpeg"),
        Some("gif") => Some("image/gif"),
        Some("webp") => Some("image/webp"),
        Some("svg") => Some("image/svg+xml"),
        Some("ico") => Some("image/x-icon"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_general_section_only() {
        let text = "\n[General]\nname=My Pack\niconKey=fabric\nlastLaunchTime=1760000000000\n\n[UI]\nname=ignored\n";
        let values = parse_general_ini(text);
        assert_eq!(values.get("name").map(String::as_str), Some("My Pack"));
        assert_eq!(values.get("iconKey").map(String::as_str), Some("fabric"));
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn parses_managed_pack() {
        let values = parse_general_ini(
            "[General]\nManagedPack=true\nManagedPackType=curseforge\nManagedPackID=123\nManagedPackName=All the Mods 10\nManagedPackVersionName=2.4.0\n",
        );
        let pack = read_managed_pack(&values).expect("managed pack");
        assert_eq!(pack.kind, "curseforge");
        assert_eq!(pack.name, "All the Mods 10");
        assert_eq!(pack.version, "2.4.0");
    }

    #[test]
    fn maps_mime_types() {
        assert_eq!(mime_for_path(Path::new("a.PNG")), Some("image/png"));
        assert_eq!(mime_for_path(Path::new("a.svg")), Some("image/svg+xml"));
        assert_eq!(mime_for_path(Path::new("a.txt")), None);
    }
}
