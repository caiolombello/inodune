//! 0.1.12 QA (B-1): a Trash folder without a valid .trashinfo (the content of
//! a ~/.ssh) became a low-risk trash.entry and was deleted by --safe
//! --permanent. Such entries, and entries from sensitive folders, are
//! analysis only. Fixtures only; HOME/XDG point to a tempdir.
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

/// Trash folder entry `name` with an ssh-like content and an optional
/// .trashinfo whose raw Path= value is `path`.
fn entry(root: &Path, name: &str, path: Option<&str>) -> std::path::PathBuf {
    let trash = root.join(".local/share/Trash");
    let dir = trash.join("files").join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(trash.join("info")).unwrap();
    fs::write(dir.join("config"), "Host example\n  User fixture\n").unwrap();
    fs::write(
        dir.join("known_hosts"),
        "example.invalid ssh-ed25519 AAAAfixture\n",
    )
    .unwrap();
    if let Some(path) = path {
        fs::write(
            trash.join("info").join(format!("{name}.trashinfo")),
            format!("[Trash Info]\nPath={path}\nDeletionDate=2026-11-02T06:00:00\n"),
        )
        .unwrap();
    }
    dir
}

fn candidate(root: &Path, dir: &Path) -> serde_json::Value {
    let (_, out) = cli(root, &["scan", "--json", "--root", root.to_str().unwrap()]);
    let scan: serde_json::Value = serde_json::from_str(&out).unwrap();
    let path = dir.to_string_lossy();
    scan["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["display_path"] == path.as_ref())
        .cloned()
        .unwrap_or_else(|| panic!("no candidate for {path}: {out}"))
}

const MODES: [&[&str]; 3] = [
    &["--safe", "--yes"],
    &["--safe", "--yes", "--permanent"],
    &["--review", "trash.entry", "--yes", "--permanent"],
];

fn assert_analysis_only(root: &Path, dir: &Path, reason_part: &str) {
    let item = candidate(root, dir);
    assert_ne!(item["risk"], "low", "{item}");
    assert_eq!(item["selected_by_default"], false);
    assert_eq!(item["supported_action"], "none", "{item}");
    assert!(
        item["reason"].as_str().unwrap().contains(reason_part),
        "{item}"
    );
    for mode in MODES {
        let mut args = vec!["clean", "--root", root.to_str().unwrap()];
        args.extend_from_slice(mode);
        let (_, out) = cli(root, &args);
        assert!(dir.join("config").exists(), "{mode:?}: {out}");
        assert!(dir.join("known_hosts").exists(), "{mode:?}: {out}");
    }
}

#[test]
fn trash_entry_without_a_valid_origin_record_is_analysis_only() {
    let cases: [(&str, Option<&str>); 6] = [
        ("orphan", None),
        ("dotdot", Some("/home/user/x/../.ssh")),
        ("dot", Some("/home/user/./old")),
        ("empty", Some("")),
        ("relative", Some("home/user/old")),
        ("empty-component", Some("/home/user//old")),
    ];
    for (name, path) in cases {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let dir = entry(root, name, path);
        assert_analysis_only(root, &dir, "Sem registro de origem válido");
    }
}

#[test]
fn trash_entry_from_a_sensitive_folder_is_analysis_only() {
    for origin in [
        ".ssh",
        ".ssh/sub",
        ".config/gh/hosts.yml",
        ".gnupg/private",
        ".aws",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let original = format!("{}/{origin}", root.display());
        let dir = entry(root, "from-sensitive", Some(&original));
        assert_analysis_only(root, &dir, "pasta sensível");
    }
}

#[test]
fn legitimate_trash_entry_keeps_working_with_permanent_cleanup() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let original = format!("{}/projects/old-notes", root.display());
    let dir = entry(root, "old-notes", Some(&original));
    let item = candidate(root, &dir);
    assert_eq!(item["detector_id"], "trash.entry", "{item}");
    assert_eq!(item["risk"], "low");
    let arg = root.to_str().unwrap();
    let (code, out) = cli(
        root,
        &["clean", "--root", arg, "--safe", "--yes", "--permanent"],
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(!dir.join("config").exists(), "{out}");
}
