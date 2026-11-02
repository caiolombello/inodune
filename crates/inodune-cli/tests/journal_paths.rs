//! 0.1.12 QA (B-8): a root given as `<dir>/./proj` left `/./` in the journal.
//! Paths are written without `.` components; same file, same identity.
use std::{fs, process::Command};

#[test]
fn journal_paths_have_no_dot_components() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let cache = root.join("proj/__pycache__");
    fs::create_dir_all(&cache).unwrap();
    fs::write(root.join("proj/m.py"), "x = 1\n").unwrap();
    let mut pyc = vec![0; 64];
    pyc[..4].copy_from_slice(&[0xa7, 0x0d, b'\r', b'\n']);
    pyc[16] = b'c';
    fs::write(cache.join("m.cpython-311.pyc"), pyc).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", root)
        .env("XDG_DATA_HOME", root.join(".local/share"))
        .env("XDG_STATE_HOME", root.join(".local/state"))
        .args(["clean", "--safe", "--yes", "--root"])
        .arg(format!("{}/./proj", root.display()))
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{out}");
    assert!(!cache.join("m.cpython-311.pyc").exists());
    let reports = root.join(".local/state/inodune/reports");
    let journals: Vec<_> = fs::read_dir(&reports)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .collect();
    assert_eq!(journals.len(), 1);
    let text = fs::read_to_string(&journals[0]).unwrap();
    assert!(text.contains("\"phase\":\"intent\""), "{text}");
    let intent = text
        .lines()
        .find(|line| line.contains("\"phase\":\"intent\""))
        .unwrap();
    assert!(!intent.contains("/./"), "{intent}");
    let capture = text
        .lines()
        .find(|line| line.contains("\"phase\":\"capture_intent\""))
        .unwrap();
    assert!(!capture.contains("/./"), "{capture}");
}
