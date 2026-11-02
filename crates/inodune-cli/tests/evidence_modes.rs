//! 0.1.10 QA: Ubuntu's umask 002 leaves Cargo.toml at 664 with the user's
//! private group (UPG). That evidence is accepted; o+w is always refused.
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

fn clean(root: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", root)
        .env("XDG_CACHE_HOME", root.join(".cache"))
        .env("XDG_DATA_HOME", root.join(".local/share"))
        .env("XDG_STATE_HOME", root.join(".local/state"))
        .args(["clean", "--root"])
        .arg(root)
        .args(["--review", "rust.target", "--yes"])
        .output()
        .unwrap()
}

fn fixture(mode: u32) -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("projects/rsproj");
    fs::create_dir_all(project.join("target/debug")).unwrap();
    fs::write(project.join("Cargo.toml"), "[package]\n").unwrap();
    fs::set_permissions(project.join("Cargo.toml"), fs::Permissions::from_mode(mode)).unwrap();
    let artifact = project.join("target/debug/artifact");
    fs::write(&artifact, b"rebuildable").unwrap();
    (temp, artifact)
}

#[test]
fn cargo_toml_664_in_the_users_private_group_is_accepted() {
    let (temp, artifact) = fixture(0o664);
    let manifest = temp.path().join("projects/rsproj/Cargo.toml");
    let meta = fs::metadata(&manifest).unwrap();
    use std::os::unix::fs::MetadataExt;
    if inodune_core::access::evidence_mode_allowed(
        meta.uid(),
        meta.gid(),
        meta.mode(),
        &inodune_core::access::SystemGroups,
    )
    .is_err()
    {
        eprintln!(
            "SKIP: this environment is not UPG (gid {}); 664 evidence is refused here by design",
            meta.gid()
        );
        return;
    }
    let output = clean(temp.path());
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0), "{text}");
    assert!(!artifact.exists(), "{text}");
    assert!(manifest.exists());
}

#[test]
fn world_writable_cargo_toml_is_still_refused() {
    let (temp, artifact) = fixture(0o666);
    let output = clean(temp.path());
    assert_ne!(output.status.code(), Some(0));
    assert!(artifact.exists());
    // The refusal names a file (not "Pasta") and its path.
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("Arquivo gravável por qualquer usuário"),
        "{text}"
    );
    assert!(text.contains("projects/rsproj/Cargo.toml"), "{text}");
}
