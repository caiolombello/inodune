//! 0.1.10 QA: `clean --permanent --dry-run` said "Ação Permanent" while the
//! item impact talked about the Trash, and sizes were raw bytes.
use std::{fs, process::Command};

#[test]
fn permanent_dry_run_text_follows_the_plan_action() {
    let temp = tempfile::tempdir().unwrap();
    let cache = temp.path().join(".npm/_cacache");
    fs::create_dir_all(cache.join("with-key")).unwrap();
    fs::create_dir_all(cache.join("safe")).unwrap();
    fs::write(
        cache.join("with-key/.netrc"),
        "machine h login u password p4ss",
    )
    .unwrap();
    fs::write(cache.join("safe/blob"), vec![b'a'; 2_700_000]).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .args([
            "clean",
            "--dry-run",
            "--permanent",
            "--review",
            "npm.cache",
            "--root",
        ])
        .arg(temp.path())
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{text}");
    assert!(text.contains("Ação: exclusão permanente"), "{text}");
    assert!(text.contains("apagados permanentemente"), "{text}");
    assert!(!text.contains("vão para a Lixeira"), "{text}");
    assert!(
        !text.contains("Permanent") && !text.contains("Review"),
        "{text}"
    );
    assert!(text.contains("2,6 MiB"), "{text}");
    assert!(!text.contains("bytes lógicos"), "{text}");
    assert!(!text.contains("/./"), "{text}");
    let json = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .args([
            "clean",
            "--dry-run",
            "--json",
            "--review",
            "npm.cache",
            "--root",
        ])
        .arg(temp.path())
        .output()
        .unwrap();
    let json = String::from_utf8_lossy(&json.stdout);
    assert!(
        json.contains("display_path") && !json.contains("/./"),
        "{json}"
    );
}

/// 0.1.10 QA: two node_modules under one root were refused ("restrinja o
/// root"); every candidate of the reviewed detector is now cleaned.
#[test]
fn review_cleans_every_candidate_of_the_detector() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let mut files = Vec::new();
    for app in ["one", "two"] {
        let project = root.join(app);
        fs::create_dir_all(project.join("node_modules/pkg")).unwrap();
        fs::write(project.join("package.json"), "{}").unwrap();
        let file = project.join("node_modules/pkg/index.js");
        fs::write(&file, "module.exports = 1;\n").unwrap();
        files.push(file);
    }
    let output = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", root)
        .env("XDG_DATA_HOME", root.join(".local/share"))
        .env("XDG_STATE_HOME", root.join(".local/state"))
        .args(["clean", "--review", "node.modules", "--yes", "--root"])
        .arg(root)
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0), "{text}");
    assert!(text.contains("2 itens"), "{text}");
    for file in files {
        assert!(!file.exists(), "{text}");
    }
}
