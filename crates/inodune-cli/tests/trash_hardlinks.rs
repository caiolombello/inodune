//! 0.1.10 QA: hardlinks moved together to the user's Trash must not make
//! later analyses incomplete; real external links still block and every
//! refusal lists its reasons. Fixtures only; HOME/XDG point to a tempdir.
use std::{fs, path::Path, process::Command};

fn cli(root: &Path, args: &[&str]) -> (Option<i32>, String, String) {
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
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn node_gyp(root: &Path) {
    let release = root.join("projects/webapp/node_modules/cpu-features/build/Release");
    fs::create_dir_all(release.join("obj.target")).unwrap();
    fs::write(root.join("projects/webapp/package.json"), "{}").unwrap();
    fs::write(release.join("cpufeatures.node"), b"native addon fixture").unwrap();
    fs::hard_link(
        release.join("cpufeatures.node"),
        release.join("obj.target/cpufeatures.node"),
    )
    .unwrap();
}

fn npm_cache(root: &Path) -> std::path::PathBuf {
    let file = root.join(".npm/_cacache/content-v2/ordinary");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, b"rebuildable").unwrap();
    file
}

fn scan_json(root: &Path) -> serde_json::Value {
    let root_arg = root.to_str().unwrap();
    let (code, out, err) = cli(root, &["scan", "--json", "--root", root_arg]);
    // 0 = complete, 3 = partial coverage (documented CLI exit codes).
    assert!(matches!(code, Some(0 | 3)), "{err}");
    serde_json::from_str(&out).unwrap()
}

#[test]
fn hardlinks_trashed_together_do_not_block_later_cleanups() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let root_arg = root.to_str().unwrap();
    node_gyp(root);
    let (code, out, err) = cli(
        root,
        &[
            "clean",
            "--root",
            root_arg,
            "--review",
            "node.modules",
            "--yes",
        ],
    );
    assert_eq!(code, Some(0), "{out}{err}");
    let trashed = fs::read_dir(root.join(".local/share/Trash/files"))
        .unwrap()
        .count();
    assert_eq!(trashed, 2, "both links in the Trash");

    let scan = scan_json(root);
    assert_eq!(scan["complete"], true, "{}", scan["issues"]);
    assert!(scan["issues"]
        .as_array()
        .unwrap()
        .iter()
        .all(|issue| issue["code"] != "hardlink"));

    let ordinary = npm_cache(root);
    let (code, out, err) = cli(
        root,
        &[
            "clean",
            "--root",
            root_arg,
            "--review",
            "npm.cache",
            "--yes",
        ],
    );
    assert_eq!(code, Some(0), "{out}{err}");
    assert!(!ordinary.exists());
    // The hardlinked pair is preserved, not emptied.
    assert_eq!(
        fs::read_dir(root.join(".local/share/Trash/files"))
            .unwrap()
            .filter(|e| fs::read(e.as_ref().unwrap().path()).ok().as_deref()
                == Some(b"native addon fixture".as_slice()))
            .count(),
        2
    );
}

#[test]
fn a_real_external_link_still_blocks_and_the_refusal_explains_why() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let root_arg = root.to_str().unwrap();
    let files = root.join(".local/share/Trash/files");
    fs::create_dir_all(&files).unwrap();
    fs::create_dir_all(root.join(".local/share/Trash/info")).unwrap();
    fs::write(root.join("outside.bin"), b"kept outside").unwrap();
    fs::hard_link(root.join("outside.bin"), files.join("linked.bin")).unwrap();
    let ordinary = npm_cache(root);

    let scan = scan_json(root);
    assert_eq!(scan["complete"], false);
    assert!(scan["issues"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| issue["code"] == "hardlink"));

    // A general cleanup refuses and lists the reason with an example path.
    let (code, out, err) = cli(root, &["clean", "--root", root_arg, "--safe", "--yes"]);
    assert_eq!(code, Some(3), "{out}{err}");
    assert!(
        out.contains("Motivos:") && out.contains("hardlink: 1"),
        "{out}"
    );
    assert!(out.contains("linked.bin"), "{out}");

    // The issue does not touch the npm cache, so its review proceeds.
    let (code, out, err) = cli(
        root,
        &[
            "clean",
            "--root",
            root_arg,
            "--review",
            "npm.cache",
            "--yes",
        ],
    );
    assert_eq!(code, Some(0), "{out}{err}");
    assert!(!ordinary.exists());
    assert!(err.contains("Análise parcial"), "{err}");
    assert_eq!(fs::read(files.join("linked.bin")).unwrap(), b"kept outside");
    assert_eq!(fs::read(root.join("outside.bin")).unwrap(), b"kept outside");
}

#[test]
fn an_issue_inside_the_reviewed_target_still_blocks() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let root_arg = root.to_str().unwrap();
    let ordinary = npm_cache(root);
    let cache = root.join(".npm/_cacache");
    std::os::unix::fs::symlink("/nonexistent-fixture", cache.join("dangling")).unwrap();
    fs::create_dir_all(cache.join("locked")).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(cache.join("locked"), fs::Permissions::from_mode(0o000)).unwrap();
    let (code, out, err) = cli(
        root,
        &[
            "clean",
            "--root",
            root_arg,
            "--review",
            "npm.cache",
            "--yes",
        ],
    );
    fs::set_permissions(cache.join("locked"), fs::Permissions::from_mode(0o700)).unwrap();
    assert_ne!(code, Some(0), "{out}{err}");
    assert!(ordinary.exists(), "{out}{err}");
}
