//! 0.1.11 QA: after cleaning pip, only empty folders were left and the new
//! analysis said "Cobertura incompleta: protegido". It is "nothing to clean"
//! and stays non-cleanable.
use std::{fs, path::Path, process::Command};

fn cli(root: &Path, args: &[&str]) -> (Option<i32>, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", root)
        .env("XDG_CACHE_HOME", root.join(".cache"))
        .env("XDG_DATA_HOME", root.join(".local/share"))
        .env("XDG_STATE_HOME", root.join(".local/state"))
        .args(args)
        .output()
        .unwrap();
    (
        output.status.code(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

fn pip_item(root: &Path) -> serde_json::Value {
    let (_, out) = cli(root, &["scan", "--json", "--root", root.to_str().unwrap()]);
    let scan: serde_json::Value = serde_json::from_str(&out).unwrap();
    scan["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["detector_id"] == "python.pip-cache")
        .cloned()
        .unwrap()
}

#[test]
fn cleaned_pip_cache_with_only_empty_folders_says_nothing_to_clean() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let body = root.join(".cache/pip/http-v2/a/b/c/d.body");
    fs::create_dir_all(body.parent().unwrap()).unwrap();
    fs::write(&body, "cached").unwrap();
    let arg = root.to_str().unwrap();
    let (code, out) = cli(
        root,
        &[
            "clean",
            "--root",
            arg,
            "--review",
            "python.pip-cache",
            "--yes",
        ],
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(!body.exists());
    assert!(
        root.join(".cache/pip/http-v2/a/b").is_dir(),
        "empty folders are kept"
    );

    let item = pip_item(root);
    let reason = item["reason"].as_str().unwrap();
    assert!(reason.contains("Nada a limpar"), "{reason}");
    assert!(!reason.contains("Cobertura incompleta"), "{reason}");
    assert_eq!(item["logical_bytes"], 0);
    assert!(item["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e == "nothing-to-clean"));
    // Still not cleanable.
    assert_eq!(item["risk"], "protected");
    assert_eq!(item["supported_action"], "none");
    assert!(item["coverage_key"].is_null());
    let (code, out) = cli(
        root,
        &[
            "clean",
            "--root",
            arg,
            "--review",
            "python.pip-cache",
            "--yes",
        ],
    );
    assert_ne!(code, Some(0), "{out}");
    assert!(out.contains("Nenhum candidato elegível"), "{out}");
    // The protection reason of each candidate is repeated.
    assert!(
        out.contains("Motivos:") && out.contains("Nada a limpar"),
        "{out}"
    );
    assert!(out.contains(".cache/pip (protegido)"), "{out}");
}

#[test]
fn an_unsafe_entry_in_an_otherwise_empty_cache_still_reports_incomplete_coverage() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    // An unreadable folder is a real coverage gap, unlike plain empty folders.
    use std::os::unix::fs::PermissionsExt;
    let locked = root.join(".cache/pip/http-v2/locked");
    fs::create_dir_all(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let item = pip_item(root);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
    let reason = item["reason"].as_str().unwrap();
    assert!(!reason.contains("Nada a limpar"), "{reason}");
    assert!(reason.contains("Cobertura incompleta"), "{reason}");
    assert_eq!(item["risk"], "protected");
}
