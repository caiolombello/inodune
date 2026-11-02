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
    assert_eq!(partial.status.code(), Some(0));
    let partial: serde_json::Value = serde_json::from_slice(&partial.stdout).unwrap();
    assert_eq!(partial["complete"], true);
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
        // Display form without `.` components (0.1.11).
        format!("{}/project/__pycache__", root.path().display())
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

#[test]
fn cli_credentials_inside_cache_are_never_removed_even_with_permanent() {
    for (relative, detector, name) in [
        (".cache/pip/http/keys", "python.pip-cache", "id_ed25519"),
        (".npm/_cacache/x", "npm.cache", ".netrc"),
        ("projects/rsproj/target/debug", "rust.target", "id_ed25519"),
        (".npm/_cacache/x", "npm.cache", "innocent"),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let cache = home.join(relative);
        fs::create_dir_all(&cache).unwrap();
        if detector == "rust.target" {
            fs::write(
                home.join("projects/rsproj/Cargo.toml"),
                b"[package]\nname=\"fixture\"\nversion=\"0.0.1\"\n",
            )
            .unwrap();
        }
        fs::write(
            cache.join(name),
            "-----BEGIN PRIVATE KEY-----\nMD4CAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==\n-----END PRIVATE KEY-----\n",
        )
        .unwrap();
        fs::write(cache.join("ordinary"), b"ordinary fixture").unwrap();
        for permanent in [false, true] {
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_inodune-cli"));
            cmd.args(["clean", "--review", detector, "--yes", "--root"])
                .arg(&home)
                .env("XDG_CACHE_HOME", home.join(".cache"))
                .env("HOME", &home)
                .env("XDG_DATA_HOME", home.join(".local/share"));
            if permanent {
                cmd.arg("--permanent");
            }
            let result = cmd.output().unwrap();
            assert!(!String::from_utf8_lossy(&result.stdout).contains("Trashed"));
            assert!(!String::from_utf8_lossy(&result.stdout).contains("Removed"));
            assert!(cache.join(name).exists());
            assert!(cache.join("ordinary").exists());
        }
    }
}

#[test]
fn safe_dry_run_excludes_review_caches() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join(".npm/_cacache")).unwrap();
    fs::write(
        temp.path().join(".npm/_cacache/blob"),
        b"recreatable fixture",
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .args(["clean", "--safe", "--dry-run", "--json", "--root"])
        .arg(temp.path())
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&result.stdout);
    assert!(result.status.success(), "{text}");
    let view: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(view["plan"].is_null(), "{text}");
    assert_eq!(view["eligible_items"], 0);
    assert!(temp.path().join(".npm/_cacache/blob").exists());
}

#[test]
fn cli_reviewed_cache_uses_xdg_trash_by_default() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let cache = home.join(".npm/_cacache/x");
    fs::create_dir_all(&cache).unwrap();
    fs::write(cache.join("ordinary"), b"ordinary fixture").unwrap();
    let data = home.join(".local/share");
    let result = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .args([
            "clean",
            "--review",
            "npm.cache",
            "--yes",
            "--json",
            "--root",
        ])
        .arg(&home)
        .env("HOME", &home)
        .env("XDG_DATA_HOME", &data)
        .env("XDG_STATE_HOME", home.join(".local/state"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["action"], "trash");
    assert_eq!(report["items"][0]["files"][0]["status"], "trashed");
    assert!(!cache.join("ordinary").exists());
    assert_eq!(fs::read_dir(data.join("Trash/info")).unwrap().count(), 1);
}
