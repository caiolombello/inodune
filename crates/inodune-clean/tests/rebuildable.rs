use inodune_clean::{
    executor::{execute, ExecutionOptions},
    plan::{Action, PlanSession},
};
use inodune_core::{
    scan::{scan, ApprovedRoot, ScanControl, ScanLimits, ScopeKind},
    Risk,
};
use std::{fs, sync::Arc};

#[test]
fn recreatable_caches_use_reviewed_plans_and_trash_without_touching_manifests() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    for path in [
        "target",
        ".next/cache",
        ".mypy_cache",
        ".ruff_cache",
        ".npm/_cacache",
    ] {
        fs::create_dir_all(project.join(path)).unwrap();
        fs::write(project.join(path).join("fixture"), b"regenerable cache").unwrap();
    }
    fs::write(
        project.join("Cargo.toml"),
        b"[package]\nname=\"fixture\"\nversion=\"0.0.1\"\n",
    )
    .unwrap();
    fs::create_dir_all(project.join("other/target")).unwrap();
    fs::write(project.join("other/target/personal"), b"preserved").unwrap();
    let root = ApprovedRoot::open(&project, ScopeKind::Projects).unwrap();
    let snapshot = Arc::new(
        scan(
            &[root],
            &ScanControl::default(),
            &ScanLimits::default(),
            |_| {},
        )
        .unwrap(),
    );
    let caches: Vec<_> = snapshot
        .candidates
        .iter()
        .filter(|c| {
            [
                "rust.target",
                "next.cache",
                "python.mypy-cache",
                "python.ruff-cache",
                "npm.cache",
            ]
            .contains(&c.detector_id.as_str())
        })
        .collect();
    assert_eq!(caches.len(), 5);
    let options = ExecutionOptions {
        trash_directory: Some(temp.path().join("data/Trash")),
        history_directory: Some(temp.path().join("state/reports")),
    };
    for c in caches {
        assert_eq!(c.risk, Risk::Review, "{}", c.reason);
        let plan = PlanSession::prepare(snapshot.clone(), &[c.id.clone()], Action::Trash).unwrap();
        let hash = plan.view().hash;
        let report = execute(
            plan.consume(&hash).unwrap(),
            &options,
            &ScanControl::default(),
            |_| {},
        );
        assert!(report.complete, "{:?}", report.items);
        assert!(!std::path::Path::new(&c.display_path)
            .join("fixture")
            .exists());
    }
    assert!(project.join("Cargo.toml").is_file());
    assert!(project.join("other/target/personal").is_file());
}
