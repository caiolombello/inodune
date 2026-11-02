use inodune_clean::{
    executor::{execute, ExecutionOptions},
    plan::{Action, PlanSession},
};
use inodune_core::scan::{scan, ApprovedRoot, ScanControl, ScanLimits, ScopeKind};
use std::{fs, sync::Arc};

#[test]
fn cache_env_overrides_do_not_redirect_deletion() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(project.join(".npm/_cacache")).unwrap();
    fs::write(project.join(".npm/_cacache/fixture"), b"scanned-cache").unwrap();
    fs::create_dir_all(project.join("target")).unwrap();
    fs::write(project.join("target/fixture"), b"scanned-target").unwrap();
    fs::write(
        project.join("Cargo.toml"),
        b"[package]\nname=\"fixture\"\nversion=\"0.0.1\"\n",
    )
    .unwrap();
    let npm_decoy = temp.path().join("other-npm-cache");
    let cargo_decoy = temp.path().join("other-cargo-target");
    fs::create_dir_all(&npm_decoy).unwrap();
    fs::create_dir_all(cargo_decoy.join("debug")).unwrap();
    fs::write(npm_decoy.join("keep"), b"leave-npm").unwrap();
    fs::write(cargo_decoy.join("debug/keep"), b"leave-cargo").unwrap();

    struct Restore;
    impl Drop for Restore {
        fn drop(&mut self) {
            // SAFETY: these variables are test-only and restored when the test ends.
            // Cleanup must ignore them; the decoy directories prove the scanned path was used.
            unsafe {
                std::env::remove_var("CARGO_TARGET_DIR");
                std::env::remove_var("npm_config_cache");
                std::env::remove_var("PIP_CACHE_DIR");
                std::env::remove_var("UV_CACHE_DIR");
            }
        }
    }
    let _restore = Restore;
    unsafe {
        std::env::set_var("CARGO_TARGET_DIR", &cargo_decoy);
        std::env::set_var("npm_config_cache", &npm_decoy);
        std::env::set_var("PIP_CACHE_DIR", &npm_decoy);
        std::env::set_var("UV_CACHE_DIR", &npm_decoy);
    }

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
    let ids = snapshot
        .candidates
        .iter()
        .filter(|candidate| matches!(candidate.detector_id.as_str(), "npm.cache" | "rust.target"))
        .map(|candidate| candidate.id.clone())
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 2, "{:?}", snapshot.candidates);
    let plan = PlanSession::prepare(snapshot, &ids, Action::Permanent).unwrap();
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
    assert!(report.complete, "{report:?}");
    assert!(!project.join(".npm/_cacache/fixture").exists());
    assert!(!project.join("target/fixture").exists());
    assert_eq!(fs::read(npm_decoy.join("keep")).unwrap(), b"leave-npm");
    assert_eq!(
        fs::read(cargo_decoy.join("debug/keep")).unwrap(),
        b"leave-cargo"
    );
}
