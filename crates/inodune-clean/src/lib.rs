//! Shared simulation compatibility and Linux backend-authorized cleanup.
pub mod diagnostics;
pub mod executor;
pub mod history;
pub mod plan;
pub mod storage;
pub mod tools;
use inodune_core::{prepare_simulation, Candidate, Plan};
pub fn dry_run(items: &[Candidate], ids: &[String]) -> Result<Plan, &'static str> {
    prepare_simulation(items, ids)
}
#[cfg(test)]
mod tests {
    #[test]
    fn dry_run_keeps_physical_gain_unknown() {
        let inv = inodune_core::demo_inventory().unwrap();
        let plan = super::dry_run(&inv.candidates, &["py".into()]).unwrap();
        assert!(plan.measured_freed_bytes.is_none());
    }
}
