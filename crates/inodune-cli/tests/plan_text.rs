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
}
