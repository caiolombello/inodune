//! B-1 at execution time: if the .trashinfo of a planned Trash entry is gone
//! or now points into a sensitive folder, the executor refuses every file.
//! Single test in this binary: it sets XDG_DATA_HOME for the user Trash.
use inodune_clean::{
    executor::{execute, ExecutionOptions, FileStatus},
    plan::{Action, PlanSession},
};
use inodune_core::scan::{scan, ApprovedRoot, ScanControl, ScanLimits, ScopeKind};
use std::{fs, sync::Arc};

#[test]
fn executor_refuses_trash_files_without_a_valid_or_safe_origin() {
    for change in ["remove", "sensitive"] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::env::set_var("XDG_DATA_HOME", root.join(".local/share"));
        let trash = root.join(".local/share/Trash");
        let dir = trash.join("files/old-notes");
        fs::create_dir_all(&dir).unwrap();
        fs::create_dir_all(trash.join("info")).unwrap();
        fs::write(dir.join("config"), "Host example\n").unwrap();
        let info = trash.join("info/old-notes.trashinfo");
        fs::write(
            &info,
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
        let plan = PlanSession::prepare(snapshot, &[id], Action::Permanent).unwrap();
        let hash = plan.view().hash;
        match change {
            "remove" => fs::remove_file(&info).unwrap(),
            _ => fs::write(
                &info,
                format!(
                    "[Trash Info]\nPath={}/.ssh\nDeletionDate=2026-11-02T06:00:00\n",
                    root.display()
                ),
            )
            .unwrap(),
        }
        let report = execute(
            plan.consume(&hash).unwrap(),
            &ExecutionOptions {
                trash_directory: Some(trash.clone()),
                history_directory: Some(root.join("state/reports")),
            },
            &ScanControl::default(),
            |_| {},
        );
        assert!(dir.join("config").exists(), "{change}: {report:?}");
        let config = report.items[0]
            .files
            .iter()
            .find(|f| f.display_path.ends_with("config"))
            .unwrap();
        assert_eq!(config.status, FileStatus::SkippedChanged, "{change}");
    }
}
