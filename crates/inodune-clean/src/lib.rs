//! Intentionally no executor in Phase 0. A dry-run cannot mutate the filesystem.
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
