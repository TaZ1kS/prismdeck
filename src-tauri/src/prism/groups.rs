use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use serde::Deserialize;

use super::fsutil::read_text_limited;
use super::types::{GroupSummary, Instance};

const MAX_GROUPS_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Default, Clone)]
pub struct GroupIndex {
    pub membership: HashMap<String, String>,
    pub hidden: HashSet<String>,
    pub ungrouped_hidden: bool,
}

#[derive(Debug, Deserialize)]
struct RawGroupsFile {
    #[serde(default)]
    groups: HashMap<String, RawGroup>,
    #[serde(default)]
    ungrouped: Option<RawUngrouped>,
}

#[derive(Debug, Deserialize)]
struct RawGroup {
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    instances: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawUngrouped {
    #[serde(default)]
    hidden: bool,
}

/// Parse `<data>/instgroups.json`. Missing or corrupt files are treated as
/// "everything is ungrouped" — never as a fatal error.
pub fn read_groups(data_dir: &Path) -> GroupIndex {
    let Some(text) = read_text_limited(&data_dir.join("instgroups.json"), MAX_GROUPS_BYTES) else {
        return GroupIndex::default();
    };
    parse_groups(&text)
}

pub fn parse_groups(text: &str) -> GroupIndex {
    let Ok(raw) = serde_json::from_str::<RawGroupsFile>(text) else {
        return GroupIndex::default();
    };

    let mut index = GroupIndex::default();
    for (name, group) in raw.groups {
        if group.hidden {
            index.hidden.insert(name.clone());
        }
        for instance_id in group.instances {
            index.membership.insert(instance_id, name.clone());
        }
    }
    if let Some(ungrouped) = raw.ungrouped {
        index.ungrouped_hidden = ungrouped.hidden;
    }
    index
}

/// Build the sidebar summary. Only groups that currently contain an instance
/// are listed; the empty-name entry represents ungrouped instances.
pub fn summarize(instances: &[Instance], index: &GroupIndex) -> Vec<GroupSummary> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut ungrouped = 0usize;

    for instance in instances {
        if instance.group.is_empty() {
            ungrouped += 1;
        } else {
            *counts.entry(instance.group.clone()).or_default() += 1;
        }
    }

    let mut groups: Vec<GroupSummary> = counts
        .into_iter()
        .map(|(name, count)| GroupSummary {
            hidden: index.hidden.contains(&name),
            name,
            count,
        })
        .collect();

    if ungrouped > 0 {
        groups.push(GroupSummary {
            name: String::new(),
            count: ungrouped,
            hidden: index.ungrouped_hidden,
        });
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_groups_and_ungrouped() {
        let raw = r#"{
            "formatVersion": 1,
            "groups": {
                "Modded": { "hidden": false, "instances": ["atm10", "fabric"] },
                "Secret": { "hidden": true, "instances": ["hidden-one"] }
            },
            "ungrouped": { "hidden": true }
        }"#;
        let index = parse_groups(raw);
        assert_eq!(index.membership.get("atm10").map(String::as_str), Some("Modded"));
        assert_eq!(index.membership.get("hidden-one").map(String::as_str), Some("Secret"));
        assert!(index.hidden.contains("Secret"));
        assert!(index.ungrouped_hidden);
    }

    #[test]
    fn corrupt_groups_are_not_fatal() {
        let index = parse_groups("{not json");
        assert!(index.membership.is_empty());
        assert!(index.hidden.is_empty());
    }
}
