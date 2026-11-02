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
        let plan =
            PlanSession::prepare(snapshot.clone(), std::slice::from_ref(&c.id), Action::Trash)
                .unwrap();
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

/// QA (node-gyp): hardlinks whose every link is inside the item are cleaned in
/// both modes; a link outside the item keeps the file preserved.
#[test]
fn internal_hardlinks_are_cleaned_and_external_links_preserved() {
    use inodune_clean::{
        executor::{execute, ExecutionOptions, FileStatus},
        plan::{Action, PlanSession},
    };
    use inodune_core::scan::{scan, ApprovedRoot, ScanControl, ScanLimits, ScopeKind};
    use std::{fs, sync::Arc};
    for action in [Action::Trash, Action::Permanent] {
        for external in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let project = temp.path().join("project");
            let release = project.join("node_modules/cpu-features/build/Release");
            fs::create_dir_all(release.join("obj.target")).unwrap();
            fs::write(project.join("package.json"), "{}").unwrap();
            fs::write(release.join("cpufeatures.node"), b"native addon fixture").unwrap();
            fs::hard_link(
                release.join("cpufeatures.node"),
                release.join("obj.target/cpufeatures.node"),
            )
            .unwrap();
            fs::write(release.join("cpu_features.a"), b"static lib fixture").unwrap();
            fs::hard_link(
                release.join("cpu_features.a"),
                release.join("obj.target/cpu_features.a"),
            )
            .unwrap();
            if external {
                fs::hard_link(release.join("cpu_features.a"), project.join("outside.a")).unwrap();
            }
            let snapshot = Arc::new(
                scan(
                    &[ApprovedRoot::open(&project, ScopeKind::Projects).unwrap()],
                    &ScanControl::default(),
                    &ScanLimits::default(),
                    |_| {},
                )
                .unwrap(),
            );
            let candidate = snapshot
                .candidates
                .iter()
                .find(|c| c.detector_id == "node.modules")
                .unwrap();
            let Ok(plan) = PlanSession::prepare(
                snapshot.clone(),
                std::slice::from_ref(&candidate.id),
                action,
            ) else {
                // An external link may already make the scan withhold the item.
                assert!(external, "{action:?}: {}", candidate.reason);
                assert!(release.join("cpu_features.a").exists());
                continue;
            };
            let hash = plan.view().hash;
            let report = execute(
                plan.consume(&hash).unwrap(),
                &ExecutionOptions {
                    trash_directory: Some(temp.path().join("data/Trash")),
                    history_directory: Some(temp.path().join("state/reports")),
                },
                &ScanControl::default(),
                |_| {},
            );
            let files = &report.items[0].files;
            let status = |name: &str| {
                files
                    .iter()
                    .filter(|f| f.display_path.ends_with(name))
                    .map(|f| f.status.clone())
                    .collect::<Vec<_>>()
            };
            let done = if action == Action::Trash {
                FileStatus::Trashed
            } else {
                FileStatus::Removed
            };
            assert_eq!(
                status("cpufeatures.node"),
                vec![done.clone(), done.clone()],
                "{action:?}"
            );
            assert!(!release.join("cpufeatures.node").exists());
            assert!(!release.join("obj.target/cpufeatures.node").exists());
            if external {
                assert!(
                    status("cpu_features.a")
                        .iter()
                        .all(|s| *s == FileStatus::SkippedChanged),
                    "{action:?}: {files:?}"
                );
                assert!(project.join("outside.a").exists());
                assert!(release.join("cpu_features.a").exists());
            } else {
                assert_eq!(
                    status("cpu_features.a"),
                    vec![done.clone(), done.clone()],
                    "{action:?}"
                );
                assert!(report.complete, "{action:?}: {files:?}");
            }
        }
    }
}
