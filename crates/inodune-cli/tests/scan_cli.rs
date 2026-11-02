use std::{fs, process::Command};
#[test]
fn json_scan_is_real_read_only_and_partial_exit_is_not_hidden() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join(".codex/sessions")).unwrap();
    fs::write(
        root.path().join(".codex/sessions/rollout.jsonl"),
        b"keep this fixture",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .args(["scan", "--json", "--root"])
        .arg(root.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["mode"], "read_only");
    assert_eq!(result["candidates"][0]["risk"], "protected");
    assert_eq!(
        fs::read(root.path().join(".codex/sessions/rollout.jsonl")).unwrap(),
        b"keep this fixture"
    );
    std::os::unix::fs::symlink(root.path().join(".codex"), root.path().join("link")).unwrap();
    let partial = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .args(["scan", "--json", "--root"])
        .arg(root.path())
        .output()
        .unwrap();
    assert_eq!(partial.status.code(), Some(3));
    let partial: serde_json::Value = serde_json::from_slice(&partial.stdout).unwrap();
    assert_eq!(partial["complete"], false);
}

#[test]
fn clean_dry_run_is_explicit_and_noninteractive_apply_does_not_touch_the_fixture() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("project");
    fs::create_dir(&project).unwrap();
    let cache = project.join("__pycache__");
    fs::create_dir(&cache).unwrap();
    fs::write(project.join("module.py"), b"source fixture\n").unwrap();
    let mut pyc = vec![0; 64];
    pyc[..4].copy_from_slice(&[0xa7, 0x0d, b'\r', b'\n']);
    pyc[16] = b'c';
    let pyc_path = cache.join("module.cpython-311.pyc");
    fs::write(&pyc_path, pyc).unwrap();
    let before = fs::read(&pyc_path).unwrap();

    let scan = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .args(["scan", "--json", "--root"])
        .arg(root.path())
        .output()
        .unwrap();
    assert!(scan.status.success());
    let scan: serde_json::Value = serde_json::from_slice(&scan.stdout).unwrap();
    assert_eq!(scan["candidates"][0]["risk"], "low", "{scan}");

    let output = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .args(["clean", "--dry-run", "--json", "--root"])
        .arg(root.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let view: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(view["mode"], "dry_run");
    assert_eq!(view["action"], "trash");
    assert_eq!(view["eligible_items"], 1, "{view}");
    assert_eq!(
        view["plan"]["items"][0]["display_path"],
        format!("{}/./project/__pycache__", root.path().display())
    );
    assert_eq!(fs::read(&pyc_path).unwrap(), before);

    let noninteractive_apply = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .args(["clean", "--apply", "--root"])
        .arg(root.path())
        .output()
        .unwrap();
    assert_eq!(noninteractive_apply.status.code(), Some(2));
    assert_eq!(fs::read(&pyc_path).unwrap(), before);
}

#[test]
fn protected_scope_and_unknown_clean_flags_cannot_select_files() {
    let root = tempfile::tempdir().unwrap();
    let secret = root.path().join("credentials.json");
    fs::write(&secret, b"protected fixture").unwrap();

    let protected = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .args(["clean", "--json", "--scope", "protected", "--root"])
        .arg(root.path())
        .output()
        .unwrap();
    assert!(protected.status.success());
    let view: serde_json::Value = serde_json::from_slice(&protected.stdout).unwrap();
    assert_eq!(view["eligible_items"], 0);
    assert_eq!(view["protected_or_review_items"], 1);
    assert!(view["plan"].is_null());
    assert_eq!(fs::read(&secret).unwrap(), b"protected fixture");

    let unknown = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .args(["clean", "--unknown", "--root"])
        .arg(root.path())
        .output()
        .unwrap();
    assert_eq!(unknown.status.code(), Some(2));
    assert_eq!(fs::read(&secret).unwrap(), b"protected fixture");
}
