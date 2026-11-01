use inodune_core::scan::{scan, ApprovedRoot, ScanControl, ScanLimits, ScopeKind};
use std::{fs, os::unix::fs::symlink};

#[test]
fn scans_only_approved_tree_without_following_links_or_modifying_metadata() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("sentinel.py"), b"private").unwrap();
    fs::create_dir(root.path().join("project")).unwrap();
    fs::write(root.path().join("project/main.py"), b"pass").unwrap();
    fs::create_dir(root.path().join("project/__pycache__")).unwrap();
    fs::write(
        root.path().join("project/__pycache__/main.cpython-313.pyc"),
        b"fixture bytecode",
    )
    .unwrap();
    symlink(outside.path(), root.path().join("escape")).unwrap();
    let before = fs::metadata(root.path().join("project/__pycache__"))
        .unwrap()
        .modified()
        .unwrap();
    let approved = ApprovedRoot::open(root.path(), ScopeKind::Projects).unwrap();
    let result = scan(
        &[approved],
        &ScanControl::default(),
        &ScanLimits::default(),
        |_| {},
    )
    .unwrap();
    assert!(!result.complete); // a skipped symlink is explicitly partial coverage.
    assert!(result
        .candidates
        .iter()
        .any(|c| c.detector_id == "python.pycache"));
    assert!(result
        .candidates
        .iter()
        .all(|c| c.display_path.starts_with(root.path().to_str().unwrap())));
    assert!(result.issues.iter().any(|i| i.code == "symlink"));
    assert_eq!(
        fs::read(outside.path().join("sentinel.py")).unwrap(),
        b"private"
    );
    assert_eq!(
        before,
        fs::metadata(root.path().join("project/__pycache__"))
            .unwrap()
            .modified()
            .unwrap()
    );
}

#[test]
fn protects_terraform_agents_unknown_cache_content_and_hardlinks() {
    use inodune_core::Risk;
    let root = tempfile::tempdir().unwrap();
    for dir in [
        "app/__pycache__",
        "infra/.terraform/providers",
        "infra/.terraform/modules",
        ".codex/sessions",
        ".claude/projects/memory",
        "linked/__pycache__",
    ] {
        fs::create_dir_all(root.path().join(dir)).unwrap();
    }
    fs::write(root.path().join("app/code.py"), b"pass").unwrap();
    fs::write(
        root.path().join("app/__pycache__/personal-notes.txt"),
        b"important",
    )
    .unwrap();
    fs::write(root.path().join("infra/main.tf"), b"terraform {}").unwrap();
    fs::write(
        root.path().join("infra/terraform.tfstate"),
        b"private state",
    )
    .unwrap();
    fs::write(
        root.path().join(".codex/sessions/rollout.jsonl"),
        b"sensitive session",
    )
    .unwrap();
    fs::write(root.path().join("linked/code.py"), b"pass").unwrap();
    fs::write(
        root.path().join("linked/__pycache__/code.cpython-313.pyc"),
        b"bytecode",
    )
    .unwrap();
    fs::hard_link(
        root.path().join("linked/__pycache__/code.cpython-313.pyc"),
        root.path().join("keep-link"),
    )
    .unwrap();
    let approved = ApprovedRoot::open(root.path(), ScopeKind::Projects).unwrap();
    let result = scan(
        &[approved],
        &ScanControl::default(),
        &ScanLimits::default(),
        |_| {},
    )
    .unwrap();
    assert!(result.candidates.len() >= 5);
    assert!(result.candidates.iter().all(|c| c.risk == Risk::Protected));
    assert!(result.issues.iter().any(|i| i.code == "hardlink"));
    assert_eq!(
        fs::read(root.path().join("infra/terraform.tfstate")).unwrap(),
        b"private state"
    );
    assert_eq!(
        fs::read(root.path().join(".codex/sessions/rollout.jsonl")).unwrap(),
        b"sensitive session"
    );
}

#[test]
fn rejects_unsafe_roots_overlaps_symlink_roots_and_respects_limits_and_cancel() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("nested")).unwrap();
    symlink(root.path().join("nested"), root.path().join("link")).unwrap();
    assert!(ApprovedRoot::open(&root.path().join("link"), ScopeKind::Projects).is_err());
    assert!(ApprovedRoot::open(std::path::Path::new("/"), ScopeKind::Projects).is_err());
    assert!(ApprovedRoot::open(std::path::Path::new("../relative"), ScopeKind::Projects).is_err());
    let a = ApprovedRoot::open(root.path(), ScopeKind::Projects).unwrap();
    let b = ApprovedRoot::open(&root.path().join("nested"), ScopeKind::Projects).unwrap();
    assert!(scan(
        &[a.clone(), b],
        &ScanControl::default(),
        &ScanLimits::default(),
        |_| {}
    )
    .is_err());
    let limits = ScanLimits {
        max_entries: 1,
        ..ScanLimits::default()
    };
    assert!(
        !scan(
            std::slice::from_ref(&a),
            &ScanControl::default(),
            &limits,
            |_| {}
        )
        .unwrap()
        .complete
    );
    let control = ScanControl::default();
    control.pause();
    control.cancel();
    let cancelled = scan(&[a], &control, &ScanLimits::default(), |_| {}).unwrap();
    assert!(cancelled.cancelled);
    assert!(!cancelled.complete);
}

#[test]
fn python_cache_without_matching_source_is_not_suggested() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("__pycache__")).unwrap();
    fs::write(root.path().join("other.py"), b"pass").unwrap();
    fs::write(
        root.path().join("__pycache__/missing.cpython-313.pyc"),
        b"bytecode",
    )
    .unwrap();
    let result = scan(
        &[ApprovedRoot::open(root.path(), ScopeKind::Projects).unwrap()],
        &ScanControl::default(),
        &ScanLimits::default(),
        |_| {},
    )
    .unwrap();
    assert_eq!(result.candidates[0].risk, inodune_core::Risk::Protected);
}

#[test]
fn malformed_bytecode_is_protected_but_real_compiler_output_can_be_suggested() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("__pycache__")).unwrap();
    fs::write(root.path().join("source.py"), b"answer = 42\n").unwrap();
    fs::write(
        root.path().join("__pycache__/source.cpython-313.pyc"),
        b"personal text, not Python bytecode",
    )
    .unwrap();
    let scan_once = || {
        scan(
            &[ApprovedRoot::open(root.path(), ScopeKind::Projects).unwrap()],
            &ScanControl::default(),
            &ScanLimits::default(),
            |_| {},
        )
        .unwrap()
    };
    assert_eq!(
        scan_once().candidates[0].risk,
        inodune_core::Risk::Protected
    );
    fs::remove_file(root.path().join("__pycache__/source.cpython-313.pyc")).unwrap();
    let compiled = std::process::Command::new("/usr/bin/python3")
        .args([
            "-I",
            "-S",
            "-c",
            "import py_compile,sys; py_compile.compile(sys.argv[1], doraise=True)",
        ])
        .arg(root.path().join("source.py"))
        .status()
        .unwrap();
    assert!(compiled.success());
    assert_eq!(scan_once().candidates[0].risk, inodune_core::Risk::Low);
}

#[test]
fn rejects_root_replaced_after_approval() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("chosen")).unwrap();
    let approved = ApprovedRoot::open(&root.path().join("chosen"), ScopeKind::Projects).unwrap();
    fs::rename(root.path().join("chosen"), root.path().join("moved")).unwrap();
    fs::create_dir(root.path().join("chosen")).unwrap();
    assert!(scan(
        &[approved],
        &ScanControl::default(),
        &ScanLimits::default(),
        |_| {}
    )
    .is_err());
}

#[test]
#[ignore = "requires unprivileged user+mount namespaces; run explicitly in an isolated namespace"]
fn refuses_bind_mounts_in_private_user_namespace() {
    if std::env::var_os("INODUNE_MOUNT_TEST_CHILD").is_none() {
        let result = std::process::Command::new("unshare")
            .args([
                "--user",
                "--map-root-user",
                "--mount",
                "--propagation",
                "private",
            ])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "refuses_bind_mounts_in_private_user_namespace",
                "--ignored",
                "--nocapture",
            ])
            .env("INODUNE_MOUNT_TEST_CHILD", "1")
            .status()
            .unwrap();
        assert!(
            result.success(),
            "user/mount namespace test unavailable or failed; do not claim mount coverage"
        );
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("external")).unwrap();
    fs::write(outside.path().join("sentinel.py"), b"must not be inspected").unwrap();
    assert!(std::process::Command::new("mount")
        .arg("--bind")
        .arg(outside.path())
        .arg(root.path().join("external"))
        .status()
        .unwrap()
        .success());
    let result = scan(
        &[ApprovedRoot::open(root.path(), ScopeKind::Projects).unwrap()],
        &ScanControl::default(),
        &ScanLimits::default(),
        |_| {},
    )
    .unwrap();
    let unmounted = std::process::Command::new("umount")
        .arg("--")
        .arg(root.path().join("external"))
        .status()
        .unwrap();
    assert!(unmounted.success());
    assert!(!result.complete);
    assert!(result.issues.iter().any(|i| i.code == "mount"));
    assert_eq!(
        fs::read(outside.path().join("sentinel.py")).unwrap(),
        b"must not be inspected"
    );
}

#[test]
fn thumbnail_format_is_checked_independently_of_python_cache_format() {
    let Some(cache_root) = std::env::var_os("INODUNE_CACHE_FIXTURE_ROOT") else {
        let root = tempfile::tempdir().unwrap();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "thumbnail_format_is_checked_independently_of_python_cache_format",
                "--nocapture",
            ])
            .env("XDG_CACHE_HOME", root.path())
            .env("INODUNE_CACHE_FIXTURE_ROOT", root.path())
            .status()
            .unwrap();
        assert!(result.success());
        return;
    };
    let thumbs = std::path::PathBuf::from(cache_root).join("thumbnails");
    fs::create_dir_all(thumbs.join("normal")).unwrap();
    fs::create_dir_all(thumbs.join("large")).unwrap();
    let mut header = vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82];
    header.resize(33, 0);
    fs::write(
        thumbs.join("normal/0123456789abcdef0123456789abcdef.png"),
        header,
    )
    .unwrap();
    fs::write(
        thumbs.join("large/0123456789abcdef0123456789abcdef.png"),
        b"personal text",
    )
    .unwrap();
    let result = scan(
        &[ApprovedRoot::open(&thumbs, ScopeKind::Thumbnails).unwrap()],
        &ScanControl::default(),
        &ScanLimits::default(),
        |_| {},
    )
    .unwrap();
    assert_eq!(
        result
            .candidates
            .iter()
            .find(|c| c.display_path.ends_with("normal"))
            .unwrap()
            .risk,
        inodune_core::Risk::Low
    );
    assert_eq!(
        result
            .candidates
            .iter()
            .find(|c| c.display_path.ends_with("large"))
            .unwrap()
            .risk,
        inodune_core::Risk::Protected
    );
}

#[test]
fn ambiguous_control_character_paths_are_inventory_only() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("misleading\nproject");
    fs::create_dir_all(project.join("__pycache__")).unwrap();
    fs::write(project.join("module.py"), b"pass").unwrap();
    let mut pyc = vec![0u8; 64];
    pyc[..4].copy_from_slice(&[0xa7, 0x0d, 13, 10]);
    pyc[16] = b'c';
    fs::write(project.join("__pycache__/module.cpython-311.pyc"), pyc).unwrap();
    let root = ApprovedRoot::open(dir.path(), ScopeKind::Projects).unwrap();
    let snapshot = scan(
        &[root],
        &ScanControl::default(),
        &ScanLimits::default(),
        |_| {},
    )
    .unwrap();
    assert_eq!(snapshot.candidates[0].risk, inodune_core::Risk::Protected);
    assert!(!snapshot.candidates[0].selected_by_default);
}

#[test]
fn opaque_next_and_package_manager_formats_are_protected_inventory() {
    let root = tempfile::tempdir().unwrap();
    for dir in ["web/.next/cache", "tools/.npm/_cacache"] {
        fs::create_dir_all(root.path().join(dir)).unwrap();
        fs::write(
            root.path().join(dir).join("unknown.bin"),
            b"fixture cache or personal content",
        )
        .unwrap();
    }
    fs::write(root.path().join("web/package.json"), b"{}").unwrap();
    let result = scan(
        &[ApprovedRoot::open(root.path(), ScopeKind::Projects).unwrap()],
        &ScanControl::default(),
        &ScanLimits::default(),
        |_| {},
    )
    .unwrap();
    for detector in ["next.cache", "npm.cache"] {
        let c = result
            .candidates
            .iter()
            .find(|c| c.detector_id == detector)
            .unwrap();
        assert_eq!(c.risk, inodune_core::Risk::Protected);
        assert!(!c.selected_by_default);
        assert!(matches!(
            c.supported_action,
            inodune_core::SupportedAction::None
        ));
    }
}

#[test]
fn directly_approved_agent_descendant_never_exposes_cache_actions() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join(".codex/sessions/project");
    fs::create_dir_all(root.join("__pycache__")).unwrap();
    fs::write(root.join("memory.py"), b"private fixture source").unwrap();
    let mut bytes = vec![0u8; 64];
    bytes[..4].copy_from_slice(&[0xa7, 0x0d, 13, 10]);
    bytes[16] = b'c';
    fs::write(root.join("__pycache__/memory.cpython-311.pyc"), bytes).unwrap();
    let snapshot = scan(
        &[ApprovedRoot::open(&root, ScopeKind::Projects).unwrap()],
        &ScanControl::default(),
        &ScanLimits::default(),
        |_| {},
    )
    .unwrap();
    assert!(snapshot
        .candidates
        .iter()
        .all(|c| c.risk == inodune_core::Risk::Protected
            && matches!(c.supported_action, inodune_core::SupportedAction::None)));
    assert!(snapshot
        .candidates
        .iter()
        .any(|c| c.detector_id == "ai.history"));
}

#[test]
fn scanner_integrates_sandboxed_git_evidence_without_writing_index() {
    use std::process::Command;
    if !std::path::Path::new("/usr/bin/bwrap").exists() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    fs::create_dir(&repo).unwrap();
    let git = |args: &[&str]| {
        Command::new("/usr/bin/git")
            .current_dir(&repo)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .args(args)
            .output()
            .unwrap()
            .status
            .success()
    };
    assert!(git(&["init", "-q", "-b", "main"]));
    fs::write(repo.join("source.txt"), b"fixture source").unwrap();
    assert!(git(&["add", "source.txt"]));
    assert!(git(&[
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit",
        "-q",
        "-m",
        "fixture"
    ]));
    let index = fs::read(repo.join(".git/index")).unwrap();
    let modified = fs::metadata(repo.join(".git/index"))
        .unwrap()
        .modified()
        .unwrap();
    for base in [dir.path(), repo.as_path()] {
        let snapshot = scan(
            &[ApprovedRoot::open(base, ScopeKind::Projects).unwrap()],
            &ScanControl::default(),
            &ScanLimits::default(),
            |_| {},
        )
        .unwrap();
        let c = snapshot
            .candidates
            .iter()
            .find(|c| c.detector_id == "git.worktree")
            .unwrap();
        assert_eq!(c.risk, inodune_core::Risk::Protected);
        assert!(c.evidence.iter().any(|e| e.contains("main=true")), "{c:?}");
        assert!(c.coverage_key.is_none());
    }
    assert_eq!(fs::read(repo.join(".git/index")).unwrap(), index);
    assert_eq!(
        fs::metadata(repo.join(".git/index"))
            .unwrap()
            .modified()
            .unwrap(),
        modified
    );
}
