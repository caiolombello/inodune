//! 0.1.12 QA (B-2/B-3): "esvaziar a Lixeira". In Trash mode a Trash entry is
//! never sent to the Trash again; --safe says so when only Trash items were
//! eligible. With --permanent the entry folder and its .trashinfo both go;
//! if something protected stays, both stay. Fixtures only; fake secrets.
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

fn trash_entry(root: &Path, name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let trash = root.join(".local/share/Trash");
    let dir = trash.join("files").join(name);
    fs::create_dir_all(dir.join("sub")).unwrap();
    fs::create_dir_all(trash.join("info")).unwrap();
    fs::write(dir.join("a.txt"), "notes\n").unwrap();
    fs::write(dir.join("sub/b.txt"), "more notes\n").unwrap();
    let info = trash.join("info").join(format!("{name}.trashinfo"));
    fs::write(
        &info,
        format!(
            "[Trash Info]\nPath={}/projects/{name}\nDeletionDate=2026-11-02T06:00:00\n",
            root.display()
        ),
    )
    .unwrap();
    (dir, info)
}

fn trash_names(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root.join(".local/share/Trash/files"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn trash_mode_never_sends_a_trash_entry_back_to_the_trash() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let (dir, info) = trash_entry(root, "old-notes");
    let arg = root.to_str().unwrap();
    for args in [
        &["clean", "--root", arg, "--safe", "--dry-run"][..],
        &["clean", "--root", arg, "--safe", "--yes"][..],
    ] {
        let (code, out) = cli(root, args);
        assert_eq!(code, Some(0), "{out}");
        assert!(
            out.contains("Itens da Lixeira só saem com exclusão permanente; use --permanent."),
            "{out}"
        );
        assert!(
            out.contains("Só há itens da Lixeira elegíveis (1 item)"),
            "{out}"
        );
        assert!(dir.join("a.txt").exists() && info.exists(), "{out}");
        assert_eq!(trash_names(root), ["old-notes"], "{out}");
    }
    let (code, out) = cli(
        root,
        &["clean", "--root", arg, "--review", "trash.entry", "--yes"],
    );
    assert_eq!(code, Some(2), "{out}");
    assert!(out.contains("--safe --permanent"), "{out}");

    // Mixed: the cache goes to the Trash, the Trash entry stays as it was.
    let cache = root.join("projects/app/__pycache__");
    fs::create_dir_all(&cache).unwrap();
    fs::write(root.join("projects/app/main.py"), "print(1)\n").unwrap();
    // A real .pyc header (magic + marker), as in scan_cli.rs.
    let mut pyc = vec![0; 64];
    pyc[..4].copy_from_slice(&[0xa7, 0x0d, b'\r', b'\n']);
    pyc[16] = b'c';
    fs::write(cache.join("main.cpython-311.pyc"), pyc).unwrap();
    let (code, out) = cli(root, &["clean", "--root", arg, "--safe", "--dry-run"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(!out.contains("Trash/files/old-notes"), "{out}");
    let (code, out) = cli(root, &["clean", "--root", arg, "--safe", "--yes"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(!cache.join("main.cpython-311.pyc").exists(), "{out}");
    assert!(dir.join("a.txt").exists() && info.exists(), "{out}");
    let names = trash_names(root);
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(names.iter().any(|n| n == "old-notes"), "{names:?}");
    assert!(
        names
            .iter()
            .all(|n| n == "old-notes" || n.starts_with("inodune-")),
        "{names:?}"
    );
}

#[test]
fn permanent_plan_text_is_one_story_and_removes_the_folder_and_its_trashinfo() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let (dir, info) = trash_entry(root, "old-notes");
    let arg = root.to_str().unwrap();
    let (code, out) = cli(
        root,
        &["clean", "--root", arg, "--safe", "--dry-run", "--permanent"],
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("Ação: exclusão permanente"), "{out}");
    for contradiction in [
        "movido",
        "Mover",
        "recuperáveis pela Lixeira",
        "Ação: mover",
    ] {
        assert!(!out.contains(contradiction), "{contradiction}: {out}");
    }
    let (code, out) = cli(
        root,
        &["clean", "--root", arg, "--safe", "--yes", "--permanent"],
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(!dir.exists(), "files/<x> left behind: {out}");
    assert!(!info.exists(), "{out}");
    assert!(!out.contains("remanescente"), "{out}");
    assert!(trash_names(root).is_empty(), "{out}");
}

#[test]
fn a_protected_leftover_keeps_both_the_folder_and_its_trashinfo() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let (dir, info) = trash_entry(root, "old-notes");
    // Fake key, detected by content.
    let key = dir.join("sub/blob");
    fs::write(
        &key,
        "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAAFAKEQAFIXTURE\n-----END OPENSSH PRIVATE KEY-----\n",
    )
    .unwrap();
    let arg = root.to_str().unwrap();
    let (_, out) = cli(
        root,
        &["clean", "--root", arg, "--safe", "--yes", "--permanent"],
    );
    assert!(key.exists(), "{out}");
    assert!(dir.is_dir(), "{out}");
    assert!(info.exists(), "files/<x> without its .trashinfo: {out}");
}
