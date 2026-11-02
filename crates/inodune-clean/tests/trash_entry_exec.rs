//! 0.1.12 QA (B-2/B-3): emptying a Trash entry. A Trash plan refuses it (it
//! would only be renamed inside the Trash); a permanent plan removes the
//! files, then the empty entry folder and only then its .trashinfo. If
//! anything is left, the folder and the .trashinfo both stay.
//! Single test in this binary: it sets XDG_DATA_HOME for the user Trash.
use inodune_clean::{
    executor::{execute, ExecutionOptions, FileStatus},
    plan::{Action, PlanSession, TRASH_NEEDS_PERMANENT},
};
use inodune_core::scan::{scan, ApprovedRoot, ScanControl, ScanLimits, ScanSnapshot, ScopeKind};
use std::{fs, path::Path, sync::Arc};

fn fixture(root: &Path) -> (Arc<ScanSnapshot>, String) {
    std::env::set_var("XDG_DATA_HOME", root.join(".local/share"));
    let trash = root.join(".local/share/Trash");
    let dir = trash.join("files/old-notes");
    fs::create_dir_all(dir.join("drafts/2026")).unwrap();
    fs::create_dir_all(trash.join("info")).unwrap();
    fs::write(dir.join("a.txt"), "notes\n").unwrap();
    fs::write(dir.join("drafts/2026/b.txt"), "draft\n").unwrap();
    fs::write(
        trash.join("info/old-notes.trashinfo"),
        format!(
            "[Trash Info]\nPath={}/projects/old-notes\nDeletionDate=2026-11-02T06:00:00\n",
            root.display()
        ),
    )
    .unwrap();
    let snapshot = Arc::new(
        scan(
            &[ApprovedRoot::open(root, ScopeKind::Projects).unwrap()],
            &ScanControl::default(),
            &ScanLimits::default(),
            |_| {},
        )
        .unwrap(),
    );
    let id = snapshot
        .candidates
        .iter()
        .find(|c| c.detector_id == "trash.entry")
        .unwrap()
        .id
        .clone();
    (snapshot, id)
}

#[test]
fn trash_entries_need_a_permanent_plan_and_never_lose_their_trashinfo_first() {
    let options = |root: &Path| ExecutionOptions {
        trash_directory: Some(root.join(".local/share/Trash")),
        history_directory: Some(root.join("state/reports")),
    };
    // Trash plan: refused with the shared wording.
    let temp = tempfile::tempdir().unwrap();
    let (snapshot, id) = fixture(temp.path());
    let error = PlanSession::prepare(snapshot, &[id], Action::Trash)
        .err()
        .expect("a Trash plan must refuse a Trash entry");
    assert_eq!(error, TRASH_NEEDS_PERMANENT);

    // Permanent plan: no files/<x> and no .trashinfo left, no warning.
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let (snapshot, id) = fixture(root);
    let plan = PlanSession::prepare(snapshot, &[id], Action::Permanent).unwrap();
    let hash = plan.view().hash;
    let report = execute(
        plan.consume(&hash).unwrap(),
        &options(root),
        &ScanControl::default(),
        |_| {},
    );
    let trash = root.join(".local/share/Trash");
    assert!(!trash.join("files/old-notes").exists(), "{report:?}");
    assert!(
        !trash.join("info/old-notes.trashinfo").exists(),
        "{report:?}"
    );
    assert!(report.items[0].complete, "{report:?}");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);

    // Something new inside the entry: the folder and the .trashinfo stay.
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let (snapshot, id) = fixture(root);
    let plan = PlanSession::prepare(snapshot, &[id], Action::Permanent).unwrap();
    let hash = plan.view().hash;
    let trash = root.join(".local/share/Trash");
    let late = trash.join("files/old-notes/drafts/late.txt");
    fs::write(&late, "written after the analysis\n").unwrap();
    let report = execute(
        plan.consume(&hash).unwrap(),
        &options(root),
        &ScanControl::default(),
        |_| {},
    );
    assert!(late.exists(), "{report:?}");
    assert!(!trash.join("files/old-notes/a.txt").exists(), "{report:?}");
    let info = trash.join("info/old-notes.trashinfo");
    assert!(info.exists(), "the .trashinfo must stay: {report:?}");
    assert!(!report.items[0].complete);
    let kept = report.items[0]
        .files
        .iter()
        .find(|f| f.display_path.ends_with(".trashinfo"))
        .unwrap();
    assert_eq!(kept.status, FileStatus::SkippedChanged);
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("Lixeira foi mantida")),
        "{:?}",
        report.warnings
    );
}
