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
