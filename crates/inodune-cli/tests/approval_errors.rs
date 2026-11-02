//! 0.1.12 QA (B-4/G-2): approving a folder showed raw errors such as
//! "Permission denied (os error 13)". The reasons are now plain sentences.
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

fn scan_error(root: &std::path::Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .args(["scan", "--json", "--root"])
        .arg(root)
        .output()
        .unwrap();
    assert!(!output.status.success());
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn folder_approval_errors_are_plain_sentences() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path();
    let locked = base.join("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let file = base.join("notes.txt");
    fs::write(&file, "text").unwrap();
    let real = base.join("real");
    fs::create_dir(&real).unwrap();
    std::os::unix::fs::symlink(&real, base.join("link")).unwrap();
    let cases = [
        (
            locked.clone(),
            "Pasta do sistema ou de outro usuário; escolha uma pasta sua.",
        ),
        (base.join("missing"), "A pasta não existe."),
        (file, "É um arquivo, não uma pasta."),
        (
            base.join("link"),
            "O caminho passa por um link; escolha a pasta real, sem links.",
        ),
    ];
    let results: Vec<String> = cases.iter().map(|(path, _)| scan_error(path)).collect();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
    for ((path, expected), error) in cases.iter().zip(results) {
        assert!(error.contains(expected), "{}: {error}", path.display());
        assert!(!error.contains("os error"), "{error}");
        assert!(!error.contains("denied"), "{error}");
    }
}
