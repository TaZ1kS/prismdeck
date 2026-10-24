pub mod detect;
pub mod fsutil;
pub mod groups;
pub mod instance;
pub mod types;

use std::path::PathBuf;

use types::{GroupSummary, PrismInstall, PrismKind, PrismSnapshot};

const MAX_INSTANCES: usize = 10_000;

/// Scan a Prism data directory. Read-only: PrismDeck never writes to Prism's
/// files. A broken instance is reported with fallback values instead of
/// aborting the scan.
pub fn scan_install(install: &PrismInstall) -> Result<PrismSnapshot, String> {
    let data_dir = PathBuf::from(&install.data_dir);
    if !data_dir.is_dir() {
        return Err(format!(
            "Prism data folder was not found: {}",
            data_dir.display()
        ));
    }

    let paths = instance::resolve_paths(&data_dir);
    let group_index = groups::read_groups(&data_dir);
    let icons = instance::collect_icons(&paths.icons_dir);

    let mut warnings = Vec::new();
    if install.exe.is_none() {
        warnings.push(
            "No Prism executable is configured. Browsing works, launching is disabled."
                .to_string(),
        );
    }
    if !paths.icons_dir.is_dir() {
        warnings.push(format!(
            "Icons folder not found: {}",
            paths.icons_dir.display()
        ));
    }

    let mut instances = Vec::new();
    if !paths.instances_dir.is_dir() {
        warnings.push(format!(
            "Instances folder not found: {}",
            paths.instances_dir.display()
        ));
    } else {
        for entry in fsutil::sorted_dir_entries(&paths.instances_dir) {
            if instances.len() >= MAX_INSTANCES {
                warnings.push(format!(
                    "Instance limit reached ({MAX_INSTANCES}); remaining folders were skipped."
                ));
                break;
            }

            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }

            let id = entry.file_name().to_string_lossy().to_string();
            if id.starts_with('.') {
                continue;
            }

            let group = group_index
                .membership
                .get(&id)
                .cloned()
                .unwrap_or_default();
            instances.push(instance::parse_instance(
                &id,
                &entry.path(),
                &icons,
                group,
            ));
        }
    }

    instances.sort_by(|left, right| {
        right
            .last_launch_ms
            .unwrap_or(0)
            .cmp(&left.last_launch_ms.unwrap_or(0))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    if instances.is_empty() && paths.instances_dir.is_dir() {
        warnings.push(format!(
            "No instances found in {}",
            paths.instances_dir.display()
        ));
    }

    let group_summaries: Vec<GroupSummary> = groups::summarize(&instances, &group_index);

    Ok(PrismSnapshot {
        install: Some(install.clone()),
        instances,
        groups: group_summaries,
        warnings,
    })
}

/// Deterministic demo install used by development builds and screenshots.
/// The fixtures ship with the repository, so Prism does not need to be
/// installed to exercise discovery-independent parsing.
pub fn demo_install() -> Result<PrismInstall, String> {
    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/prism");
    let data_dir = fixture_root
        .canonicalize()
        .unwrap_or_else(|_| fixture_root.clone());
    if !data_dir.is_dir() {
        return Err(format!(
            "Demo fixtures are missing: {}",
            data_dir.display()
        ));
    }

    Ok(PrismInstall {
        id: format!("demo|{}|", data_dir.display()),
        kind: PrismKind::Custom,
        label: "Demo fixtures".to_string(),
        exe: None,
        app_id: None,
        data_dir: data_dir.to_string_lossy().to_string(),
        source: "bundled fixtures".to_string(),
        valid: true,
        version: Some("fixture".to_string()),
    })
}

pub fn demo_snapshot() -> Result<PrismSnapshot, String> {
    let install = demo_install()?;
    scan_install(&install)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_tree_scans_without_prism() {
        let snapshot = demo_snapshot().expect("demo fixtures should scan");

        assert_eq!(snapshot.instances.len(), 6);
        assert_eq!(snapshot.install.as_ref().map(|i| i.kind), Some(PrismKind::Custom));

        let fabric = snapshot
            .instances
            .iter()
            .find(|instance| instance.id == "fabric-1.20.1")
            .expect("fabric fixture");
        assert_eq!(fabric.name, "Fabric Lab");
        assert_eq!(fabric.mc_version.as_deref(), Some("1.20.1"));
        assert_eq!(fabric.loader.as_deref(), Some("Fabric"));
        assert_eq!(fabric.group, "Modded");
        assert!(fabric.icon_path.is_some());

        let atm = snapshot
            .instances
            .iter()
            .find(|instance| instance.id == "atm10")
            .expect("atm10 fixture");
        assert_eq!(atm.mc_version.as_deref(), Some("1.21.1"));
        assert_eq!(atm.loader.as_deref(), Some("NeoForge"));
        let pack = atm.managed_pack.as_ref().expect("managed pack");
        assert_eq!(pack.kind, "curseforge");
        assert_eq!(pack.name, "All the Mods 10");

        let broken = snapshot
            .instances
            .iter()
            .find(|instance| instance.id == "broken-instance")
            .expect("broken fixture is still listed");
        assert_eq!(broken.name, "broken-instance");
        assert_eq!(broken.mc_version, None);
        assert_eq!(broken.group, "");

        let modded = snapshot
            .groups
            .iter()
            .find(|group| group.name == "Modded")
            .expect("Modded group");
        assert_eq!(modded.count, 4);
    }
}
