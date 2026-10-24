use serde::{Deserialize, Serialize};

/// How Prism was found. The kind drives the launch command shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrismKind {
    Native,
    Flatpak,
    Portable,
    AppImage,
    Custom,
    DataOnly,
}

impl PrismKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Native => "Prism Launcher (native)",
            Self::Flatpak => "Prism Launcher (Flatpak)",
            Self::Portable => "Prism Launcher (portable)",
            Self::AppImage => "Prism Launcher (AppImage)",
            Self::Custom => "Custom Prism data",
            Self::DataOnly => "Prism data folder",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Flatpak => "flatpak",
            Self::Portable => "portable",
            Self::AppImage => "appimage",
            Self::Custom => "custom",
            Self::DataOnly => "data",
        }
    }

    pub fn rank(self) -> u8 {
        match self {
            Self::Native => 0,
            Self::Flatpak => 1,
            Self::Portable => 2,
            Self::AppImage => 3,
            Self::Custom => 4,
            Self::DataOnly => 5,
        }
    }
}

/// A discovered or manually selected Prism installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrismInstall {
    pub id: String,
    pub kind: PrismKind,
    pub label: String,
    /// Executable path. `None` for a data-only folder that can be browsed
    /// but not launched until the user points PrismDeck at an executable.
    pub exe: Option<String>,
    /// Flatpak application id when `kind == Flatpak`.
    pub app_id: Option<String>,
    pub data_dir: String,
    pub source: String,
    /// The data directory exists on disk.
    pub valid: bool,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedPack {
    /// `curseforge`, `modrinth`, `ftb`, `technic`, `atlauncher`, ...
    pub kind: String,
    pub id: String,
    pub name: String,
    pub version: String,
}

/// Instance metadata assembled from `instance.cfg`, `mmc-pack.json`,
/// `instgroups.json` and the icon folder. Everything is read-only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instance {
    /// Folder name inside Prism's instances directory. This is Prism's
    /// launch id — display names may contain spaces, ids should not.
    pub id: String,
    pub name: String,
    pub mc_version: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub managed_pack: Option<ManagedPack>,
    pub icon_key: String,
    /// Empty string means "ungrouped".
    pub group: String,
    pub notes: String,
    /// Unix milliseconds, as written by Prism.
    pub last_launch_ms: Option<i64>,
    /// Seconds, as written by Prism.
    pub total_played_s: i64,
    /// Seconds, as written by Prism.
    pub last_played_s: i64,
    /// Internal absolute path, never serialized to the webview.
    #[serde(skip)]
    pub instance_dir: String,
    /// Internal icon path, never serialized to the webview.
    #[serde(skip)]
    pub icon_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GroupSummary {
    /// Empty string is the synthetic "Ungrouped" entry.
    pub name: String,
    pub count: usize,
    pub hidden: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PrismSnapshot {
    pub install: Option<PrismInstall>,
    pub instances: Vec<Instance>,
    pub groups: Vec<GroupSummary>,
    pub warnings: Vec<String>,
}
