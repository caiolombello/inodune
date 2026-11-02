//! The desktop and CLI both prepare these plans and call this executor.
use inodune_clean::{
    executor::{execute, ExecutionOptions, FileStatus},
    plan::{Action, PlanSession},
};
use inodune_core::{
    scan::{scan, ApprovedRoot, ScanControl, ScanLimits, ScopeKind},
    Risk,
};
use std::{fs, sync::Arc};

#[test]
fn shared_gui_executor_refuses_entire_sensitive_cache_even_for_permanent_action() {
    for name in ["id_ed25519", ".netrc", "innocent"] {
        let temp = tempfile::tempdir().unwrap();
        let cache = temp.path().join("project/.npm/_cacache/x");
        fs::create_dir_all(&cache).unwrap();
        fs::write(
            cache.join(name),
            "-----BEGIN PRIVATE KEY-----\nMD4CAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==\n-----END PRIVATE KEY-----\n",
        )
        .unwrap();
        fs::write(cache.join("ordinary"), b"recreatable fixture").unwrap();
        let snapshot = Arc::new(
            scan(
                &[ApprovedRoot::open(&temp.path().join("project"), ScopeKind::Projects).unwrap()],
                &ScanControl::default(),
                &ScanLimits::default(),
                |_| {},
            )
            .unwrap(),
        );
        let candidate = snapshot
            .candidates
            .iter()
            .find(|c| c.detector_id == "npm.cache")
            .unwrap();
        assert_eq!(candidate.risk, Risk::Protected);
        for action in [Action::Trash, Action::Permanent] {
            assert!(PlanSession::prepare(
                snapshot.clone(),
                std::slice::from_ref(&candidate.id),
                action
            )
            .is_err());
        }
        assert!(cache.join(name).exists());
        assert!(cache.join("ordinary").exists());
    }
}

#[test]
fn reviewed_cache_default_action_really_trashes_and_reports_trash() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    let cache = project.join(".npm/_cacache/x");
    fs::create_dir_all(&cache).unwrap();
    fs::write(cache.join("ordinary"), b"recreatable fixture").unwrap();
    let snapshot = Arc::new(
        scan(
            &[ApprovedRoot::open(&project, ScopeKind::Projects).unwrap()],
            &ScanControl::default(),
            &ScanLimits::default(),
            |_| {},
        )
        .unwrap(),
    );
    let id = snapshot
        .candidates
        .iter()
        .find(|c| c.detector_id == "npm.cache")
        .unwrap()
        .id
        .clone();
    let plan = PlanSession::prepare(snapshot, &[id], Action::Trash).unwrap();
    let hash = plan.view().hash;
    let trash = temp.path().join("data/Trash");
    let report = execute(
        plan.consume(&hash).unwrap(),
        &ExecutionOptions {
            trash_directory: Some(trash.clone()),
            history_directory: Some(temp.path().join("state/reports")),
        },
        &ScanControl::default(),
        |_| {},
    );
    assert!(report.complete, "{report:?}");
    assert_eq!(report.action, Action::Trash);
    assert_eq!(report.items[0].files[0].status, FileStatus::Trashed);
    assert!(!cache.join("ordinary").exists());
    assert_eq!(fs::read_dir(trash.join("files")).unwrap().count(), 1);
    assert_eq!(fs::read_dir(trash.join("info")).unwrap().count(), 1);
}

#[test]
fn partial_cache_plan_excludes_key_directory_and_hashes_its_exclusions() {
    for action in [Action::Trash, Action::Permanent] {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("project");
        let cache = project.join(".npm/_cacache");
        fs::create_dir_all(cache.join("with-key")).unwrap();
        fs::create_dir_all(cache.join("safe")).unwrap();
        fs::write(
            cache.join("with-key/.netrc"),
            "machine invalid login synthetic password synthetic",
        )
        .unwrap();
        fs::write(
            cache.join("with-key/ordinary"),
            "preserve sibling in immediate directory",
        )
        .unwrap();
        fs::write(cache.join("safe/ordinary"), "rebuildable").unwrap();
        let snapshot = scan(
            &[ApprovedRoot::open(&project, ScopeKind::Projects).unwrap()],
            &ScanControl::default(),
            &ScanLimits::default(),
            |_| {},
        )
        .unwrap();
        let id = snapshot
            .candidates
            .iter()
            .find(|c| c.detector_id == "npm.cache")
            .unwrap()
            .id
            .clone();
        let target = &snapshot.targets[&id];
        assert!(target
            .files
            .iter()
            .all(|(p, _)| !p.to_string_lossy().contains("with-key")));
        let plan = PlanSession::prepare(
            Arc::new(snapshot.clone()),
            std::slice::from_ref(&id),
            action,
        )
        .unwrap();
        let hash = plan.view().hash;
        let mut changed = snapshot;
        let target = changed.targets.get_mut(&id).unwrap();
        target
            .preserved
            .push(target.relative.join("additional-preserved"));
        let altered = PlanSession::prepare(Arc::new(changed), &[id], action).unwrap();
        assert_ne!(hash, altered.view().hash);
        let trash = temp.path().join("data/Trash");
        let report = execute(
            plan.consume(&hash).unwrap(),
            &ExecutionOptions {
                trash_directory: Some(trash.clone()),
                history_directory: Some(temp.path().join("state/reports")),
            },
            &ScanControl::default(),
            |_| {},
        );
        assert!(report.complete, "{report:?}");
        assert!(cache.join("with-key/.netrc").exists());
        assert!(cache.join("with-key/ordinary").exists());
        assert!(!cache.join("safe/ordinary").exists());
        if action == Action::Trash {
            assert_eq!(fs::read_dir(trash.join("files")).unwrap().count(), 1);
            assert_eq!(fs::read_dir(trash.join("info")).unwrap().count(), 1);
        }
    }
}
