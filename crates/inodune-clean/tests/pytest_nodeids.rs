use inodune_clean::{
    executor::{execute, ExecutionOptions},
    plan::{Action, PlanSession},
};
use inodune_core::{
    scan::{scan, ApprovedRoot, ScanControl, ScanLimits, ScopeKind},
    Risk, SupportedAction,
};
use std::{fs, path::PathBuf, sync::Arc};

const README: &[u8] = b"# pytest cache directory #\n\nThis directory contains data from the pytest's cache plugin,\nwhich provides the `--lf` and `--ff` options, as well as the `cache` fixture.\n\n**Do not** commit this to version control.\nSee [the docs](https://docs.pytest.org/en/stable/how-to/cache.html) for more information.\n";
const GITIGNORE: &[u8] = b"# Created by pytest automatically.\n*\n";
const TAG: &[u8] = b"Signature: 8a477f597d28d172789f06886806bc55\n# This file is a cache directory tag created by pytest.\n# For information about cache directory tags, see:\n#\thttps://bford.info/cachedir/spec.html\n";

struct Fixture {
    dir: tempfile::TempDir,
    root: PathBuf,
    leaf: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("projects");
        let cache = root.join("project/.pytest_cache");
        let v = cache.join("v/cache");
        fs::create_dir_all(&v).unwrap();
        fs::write(cache.join("README.md"), README).unwrap();
        fs::write(cache.join(".gitignore"), GITIGNORE).unwrap();
        fs::write(cache.join("CACHEDIR.TAG"), TAG).unwrap();
        let leaf = v.join("nodeids");
        fs::write(
            &leaf,
            br#"["tests/test_alpha.py::test_one","tests/test_beta.py::test_two"]"#,
        )
        .unwrap();
        fs::write(
            v.join("lastfailed"),
            br#"{"tests/test_secret.py::test_login":true}"#,
        )
        .unwrap();
        fs::write(
            v.join("stepwise"),
            br#"["tests/test_secret.py::test_next"]"#,
        )
        .unwrap();
        fs::write(v.join("plugin-private"), b"plugin state fixture").unwrap();
        Self { dir, root, leaf }
    }
    fn snapshot(&self) -> Arc<inodune_core::scan::ScanSnapshot> {
        Arc::new(
            scan(
                &[ApprovedRoot::open(&self.root, ScopeKind::Projects).unwrap()],
                &ScanControl::default(),
                &ScanLimits::default(),
                |_| {},
            )
            .unwrap(),
        )
    }
    fn options(&self) -> ExecutionOptions {
        ExecutionOptions {
            trash_directory: Some(self.dir.path().join("data/Trash")),
            history_directory: Some(self.dir.path().join("state/reports")),
        }
    }
    fn candidate(snapshot: &inodune_core::scan::ScanSnapshot) -> String {
        let c = snapshot
            .candidates
            .iter()
            .find(|c| c.detector_id == "python.pytest-nodeids")
            .expect("verified nodeids leaf");
        assert_eq!(c.risk, Risk::Review);
        assert!(!c.selected_by_default);
        assert!(matches!(
            c.supported_action,
            SupportedAction::ReviewedCacheContents
        ));
        c.id.clone()
    }
    fn preserved(&self) {
        assert_eq!(
            fs::read(self.leaf.parent().unwrap().join("lastfailed")).unwrap(),
            br#"{"tests/test_secret.py::test_login":true}"#
        );
        assert_eq!(
            fs::read(self.leaf.parent().unwrap().join("stepwise")).unwrap(),
            br#"["tests/test_secret.py::test_next"]"#
        );
        assert_eq!(
            fs::read(self.leaf.parent().unwrap().join("plugin-private")).unwrap(),
            b"plugin state fixture"
        );
        assert!(self
            .leaf
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("README.md")
            .exists());
    }
}

#[test]
fn reviewed_execution_removes_only_the_nodeids_leaf_in_temporary_fixture() {
    let f = Fixture::new();
    let snapshot = f.snapshot();
    let id = Fixture::candidate(&snapshot);
    let plan = PlanSession::prepare(snapshot, &[id], Action::Permanent).unwrap();
    let hash = plan.view().hash;
    let report = execute(
        plan.consume(&hash).unwrap(),
        &f.options(),
        &ScanControl::default(),
        |_| {},
    );
    assert!(report.complete, "{report:?}");
    assert_eq!(report.logical_bytes_processed, 64);
    assert!(!f.leaf.exists());
    f.preserved();
}

#[test]
fn changed_nodeids_after_scan_are_skipped_without_touching_fixture_state() {
    let f = Fixture::new();
    let snapshot = f.snapshot();
    let id = Fixture::candidate(&snapshot);
    let plan = PlanSession::prepare(snapshot, &[id], Action::Permanent).unwrap();
    let hash = plan.view().hash;
    fs::write(&f.leaf, br#"["changed-after-scan"]"#).unwrap();
    let report = execute(
        plan.consume(&hash).unwrap(),
        &f.options(),
        &ScanControl::default(),
        |_| {},
    );
    assert!(!report.complete, "{report:?}");
    assert!(f.leaf.exists());
    f.preserved();
}

#[test]
fn changed_pytest_marker_after_scan_is_skipped() {
    let f = Fixture::new();
    let snapshot = f.snapshot();
    let id = Fixture::candidate(&snapshot);
    let plan = PlanSession::prepare(snapshot, &[id], Action::Permanent).unwrap();
    let hash = plan.view().hash;
    let marker = f
        .leaf
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("CACHEDIR.TAG");
    fs::write(&marker, b"changed marker fixture").unwrap();
    let report = execute(
        plan.consume(&hash).unwrap(),
        &f.options(),
        &ScanControl::default(),
        |_| {},
    );
    assert!(!report.complete, "{report:?}");
    assert!(f.leaf.exists());
    f.preserved();
}
