//! Shared contracts, read-only discovery and conservative planning.
#[cfg(target_os = "linux")]
pub mod scan;
#[cfg(target_os = "linux")]
pub mod git;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    Low,
    Review,
    Protected,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    pub dev: u64,
    pub ino: u64,
    pub mtime_unix: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Detector {
    pub id: String,
    pub version: String,
    pub documentation: Vec<String>,
    pub risk: Risk,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApprovedScope {
    pub id: String,
    pub label: String,
    pub display_path: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub title: String,
    pub display_path: String,
    pub detector_id: String,
    pub detector_version: String,
    pub scope_id: String,
    pub category: String,
    pub project: String,
    pub risk: Risk,
    pub logical_bytes: u64,
    pub allocated_bytes_estimate: Option<u64>,
    pub identity: Option<Identity>,
    pub coverage_key: Option<String>,
    pub selected_by_default: bool,
    pub reason: String,
    pub impact: String,
    pub rebuild_hint: String,
    pub preserved: String,
    pub evidence: Vec<String>,
    pub activity: String,
    pub supported_action: SupportedAction,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportedAction {
    None,
    CacheContents,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Volume {
    pub label: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inventory {
    pub schema_version: u32,
    pub mode: Mode,
    pub scopes: Vec<ApprovedScope>,
    pub volume: Volume,
    pub candidates: Vec<Candidate>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Simulation,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub mode: Mode,
    pub candidate_ids: Vec<String>,
    pub logical_bytes_estimate: u64,
    pub measured_freed_bytes: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Simulated,
    SkippedChanged,
    Failed,
    Cancelled,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemOutcome {
    pub candidate_id: String,
    pub outcome: Outcome,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub mode: Mode,
    pub items: Vec<ItemOutcome>,
    pub measured_freed_bytes: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScanEvent {
    Started,
    Candidate {
        candidate_id: String,
        emitted: usize,
    },
    Paused,
    Cancelled,
    Completed,
    Error {
        message: String,
    },
}

pub fn demo_inventory() -> Result<Inventory, serde_json::Error> {
    serde_json::from_str(include_str!("../../../tests/fixtures/demo.json"))
}

/// This is a simulation plan, never a filesystem authorization token.
pub fn prepare_simulation(items: &[Candidate], ids: &[String]) -> Result<Plan, &'static str> {
    let mut inventory_ids = HashSet::new();
    if items.iter().any(|c| !inventory_ids.insert(&c.id)) {
        return Err("duplicate inventory ID");
    }
    let mut seen = HashSet::new();
    let mut coverage = HashSet::new();
    let mut total = 0u64;
    for id in ids {
        if !seen.insert(id) {
            continue;
        }
        let item = items
            .iter()
            .find(|c| &c.id == id)
            .ok_or("unknown candidate")?;
        if item.risk == Risk::Protected {
            return Err("protected candidate");
        }
        let key = item.coverage_key.as_ref().ok_or("coverage unknown")?;
        if !coverage.insert(key) {
            return Err("overlapping coverage");
        }
        total = total
            .checked_add(item.logical_bytes)
            .ok_or("byte overflow")?;
    }
    if seen.is_empty() {
        return Err("empty selection");
    }
    Ok(Plan {
        mode: Mode::Simulation,
        candidate_ids: ids.iter().filter(|id| seen.remove(id)).cloned().collect(),
        logical_bytes_estimate: total,
        measured_freed_bytes: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corpus_is_explicit_and_conservative() {
        let inv = demo_inventory().unwrap();
        assert_eq!(inv.schema_version, 1);
        for c in inv.candidates {
            assert!(c.display_path.starts_with("/demo/"));
            assert!(c.identity.is_none());
            if c.risk != Risk::Low {
                assert!(!c.selected_by_default);
            }
            if c.detector_id.starts_with("ai.")
                || c.detector_id.starts_with("terraform.")
                || c.detector_id == "git.worktree"
            {
                assert_eq!(c.risk, Risk::Protected);
            }
        }
    }
    #[test]
    fn rejects_protected_unknown_and_overlapping() {
        let mut inv = demo_inventory().unwrap();
        for id in ["ai", "git", "tfp", "state", "unknown"] {
            assert!(prepare_simulation(&inv.candidates, &[id.into()]).is_err());
        }
        inv.candidates[1].coverage_key = inv.candidates[0].coverage_key.clone();
        assert!(prepare_simulation(&inv.candidates, &["thumbs".into(), "py".into()]).is_err());
    }
    #[test]
    fn deduplicates_and_never_claims_reclaimed_bytes() {
        let inv = demo_inventory().unwrap();
        let plan =
            prepare_simulation(&inv.candidates, &["thumbs".into(), "thumbs".into()]).unwrap();
        assert_eq!(plan.candidate_ids.len(), 1);
        assert_eq!(plan.logical_bytes_estimate, inv.candidates[0].logical_bytes);
        assert_eq!(plan.measured_freed_bytes, None);
    }
}
