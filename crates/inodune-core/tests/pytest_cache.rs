use inodune_core::{
    scan::{scan, ApprovedRoot, ScanControl, ScanLimits, ScopeKind},
    Risk, SupportedAction,
};
use std::fs;

const CACHEDIR_TAG: &[u8] = b"Signature: 8a477f597d28d172789f06886806bc55\n# This file is a cache directory tag created by pytest.\n# For information about cache directory tags, see:\n#\thttps://bford.info/cachedir/spec.html\n";

fn create_pytest_cache(root: &std::path::Path, nodeids: &[u8]) {
    let cache = root.join("project/.pytest_cache");
    fs::create_dir_all(cache.join("v/cache")).unwrap();
    fs::write(
        cache.join("README.md"),
        b"# pytest cache directory #\n\nThis directory contains data from the pytest's cache plugin,\nwhich provides the `--lf` and `--ff` options, as well as the `cache` fixture.\n\n**Do not** commit this to version control.\nSee [the docs](https://docs.pytest.org/en/stable/how-to/cache.html) for more information.\n",
    )
    .unwrap();
    fs::write(
        cache.join(".gitignore"),
        b"# Created by pytest automatically.\n*\n",
    )
    .unwrap();
    fs::write(cache.join("CACHEDIR.TAG"), CACHEDIR_TAG).unwrap();
    fs::write(cache.join("v/cache/nodeids"), nodeids).unwrap();
}

fn scan_project(root: &std::path::Path) -> inodune_core::scan::ScanSnapshot {
    let approved = ApprovedRoot::open(root, ScopeKind::Projects).unwrap();
    scan(
        &[approved],
        &ScanControl::default(),
        &ScanLimits::default(),
        |_| {},
    )
    .unwrap()
}

#[test]
fn offers_only_verified_pytest_nodeids_as_an_individual_review_item() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("project")).unwrap();
    fs::write(
        root.path().join("project/pyproject.toml"),
        b"[tool.pytest.ini_options]\ntestpaths = ['tests']\n",
    )
    .unwrap();

    // A synthetic large suite makes the impact measurable while keeping all I/O
    // inside this test's TempDir. IDs are sorted and unique, as pytest writes them.
    let ids: Vec<String> = (0..12_000)
        .map(|i| {
            format!("tests/test_component_{i:05}.py::TestSubsystem::test_case_{i:05}[linux-x86_64]")
        })
        .collect();
    let bytes = serde_json::to_vec(&ids).unwrap();
    assert!(bytes.len() > 512 * 1024);
    create_pytest_cache(root.path(), &bytes);

    let snapshot = scan_project(root.path());
    let candidates: Vec<_> = snapshot
        .candidates
        .iter()
        .filter(|candidate| candidate.detector_id == "python.pytest-nodeids")
        .collect();
    let cache_container = snapshot
        .candidates
        .iter()
        .find(|candidate| candidate.detector_id == "python.toolcache")
        .unwrap();
    let target = snapshot.targets.get(&cache_container.id).unwrap();
    let decision = inodune_core::cache_rules::classify_pytest_nodeids(
        &target.root,
        &target.relative.join("v/cache/nodeids"),
        &target.files,
    );

    assert_eq!(
        candidates.len(),
        1,
        "candidatos observados: {:?}; decision={decision:?}; files={:?}",
        snapshot
            .candidates
            .iter()
            .map(|candidate| (
                &candidate.detector_id,
                &candidate.display_path,
                &candidate.reason
            ))
            .collect::<Vec<_>>(),
        target
            .files
            .iter()
            .map(|(path, _)| path)
            .collect::<Vec<_>>()
    );
    let candidate = candidates[0];
    assert_eq!(candidate.risk, Risk::Review);
    assert!(matches!(
        candidate.supported_action,
        SupportedAction::ReviewedCacheContents
    ));
    assert!(!candidate.selected_by_default);
    assert_eq!(candidate.logical_bytes, bytes.len() as u64);
    assert!(candidate.reason.contains("--new-first"));
    assert!(candidate.preserved.contains("lastfailed"));

    let parent = snapshot
        .candidates
        .iter()
        .find(|candidate| {
            candidate.detector_id == "python.toolcache"
                && candidate.display_path.ends_with("/.pytest_cache")
        })
        .unwrap();
    assert_eq!(parent.risk, Risk::Protected);
    assert!(matches!(parent.supported_action, SupportedAction::None));
    assert!(parent.logical_bytes < bytes.len() as u64 + 1024);
}

#[test]
fn protects_pytest_session_state_plugins_and_unrecognized_formats() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("project")).unwrap();
    let small_valid = serde_json::to_vec(&["tests/test_a.py::test_a"]).unwrap();
    create_pytest_cache(root.path(), &small_valid);

    // These are pytest's resume/failure state or arbitrary plugin state. They stay
    // in the parent inventory but never receive their own cleanup authority.
    let cache = root.path().join("project/.pytest_cache");
    fs::write(cache.join("v/cache/lastfailed"), b"{\"test_a\":true}").unwrap();
    fs::write(cache.join("v/cache/stepwise"), b"[\"test_b\"]").unwrap();
    fs::write(cache.join("v/cache/plugin-private"), b"private plugin data").unwrap();

    let snapshot = scan_project(root.path());
    let nodeids = snapshot
        .candidates
        .iter()
        .find(|candidate| candidate.detector_id == "python.pytest-nodeids")
        .unwrap();
    assert_eq!(nodeids.risk, Risk::Review);
    let entire_cache = snapshot
        .candidates
        .iter()
        .find(|candidate| {
            candidate.detector_id == "python.toolcache"
                && candidate.display_path.ends_with("/.pytest_cache")
        })
        .unwrap();
    assert_eq!(entire_cache.risk, Risk::Protected);
    assert!(entire_cache.logical_bytes > 0);

    fs::write(
        cache.join("v/cache/nodeids"),
        b"{\"not\":\"an array of nodeids\"}",
    )
    .unwrap();
    let snapshot = scan_project(root.path());
    assert!(snapshot
        .candidates
        .iter()
        .all(|candidate| candidate.detector_id != "python.pytest-nodeids"));
}

#[test]
fn protects_symlink_and_hardlinked_nodeids_leaves() {
    use std::os::unix::fs::symlink;

    for hardlink in [false, true] {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("project")).unwrap();
        create_pytest_cache(root.path(), br#"["tests/test_a.py::test_a"]"#);
        let leaf = root.path().join("project/.pytest_cache/v/cache/nodeids");
        if hardlink {
            let alias = root.path().join("nodeids-alias");
            fs::hard_link(&leaf, alias).unwrap();
        } else {
            let target = root.path().join("nodeids-target");
            fs::rename(&leaf, &target).unwrap();
            symlink(&target, &leaf).unwrap();
        }
        let snapshot = scan_project(root.path());
        assert!(snapshot
            .candidates
            .iter()
            .all(|candidate| candidate.detector_id != "python.pytest-nodeids"));
    }
}

#[test]
fn progress_reports_only_the_latest_four_real_candidates_including_final_state() {
    let root = tempfile::tempdir().unwrap();
    for i in 0..6 {
        let project = root.path().join(format!("project-{i}"));
        fs::create_dir_all(&project).unwrap();
        let cache = project.join(".pytest_cache");
        let v = cache.join("v/cache");
        fs::create_dir_all(&v).unwrap();
        fs::write(cache.join("README.md"), b"# pytest cache directory #\n\nThis directory contains data from the pytest's cache plugin,\nwhich provides the `--lf` and `--ff` options, as well as the `cache` fixture.\n\n**Do not** commit this to version control.\nSee [the docs](https://docs.pytest.org/en/stable/how-to/cache.html) for more information.\n").unwrap();
        fs::write(
            cache.join(".gitignore"),
            b"# Created by pytest automatically.\n*\n",
        )
        .unwrap();
        fs::write(cache.join("CACHEDIR.TAG"), CACHEDIR_TAG).unwrap();
        fs::write(
            v.join("nodeids"),
            serde_json::to_vec(&[format!("tests/test_{i}.py::test_case")]).unwrap(),
        )
        .unwrap();
    }
    let approved = ApprovedRoot::open(root.path(), ScopeKind::Projects).unwrap();
    let mut events = Vec::new();
    let snapshot = scan(
        &[approved],
        &ScanControl::default(),
        &ScanLimits::default(),
        |event| events.push(event),
    )
    .unwrap();
    let final_event = events.last().unwrap();
    assert_eq!(final_event.stage, "Concluído");
    assert_eq!(final_event.candidates, snapshot.candidates.len());
    assert_eq!(final_event.recent_findings.len(), 4);
    let expected = &snapshot.candidates[snapshot.candidates.len() - 4..];
    for (finding, candidate) in final_event.recent_findings.iter().zip(expected) {
        assert_eq!(finding.id, candidate.id);
        assert_eq!(finding.title, candidate.title);
        assert_eq!(finding.category, candidate.category);
        assert_eq!(finding.risk, candidate.risk);
        assert_eq!(finding.logical_bytes, candidate.logical_bytes);
    }
    let payload = serde_json::to_string(final_event).unwrap();
    assert!(!payload.contains(root.path().to_str().unwrap()));
    let legacy: inodune_core::scan::ScanProgress =
        serde_json::from_str(r#"{"visited":1,"candidates":0,"stage":"Scanning"}"#).unwrap();
    assert!(legacy.recent_findings.is_empty());
}
