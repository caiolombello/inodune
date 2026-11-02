//! QA regression (0.1.9): `clean --review <detector> --yes` and
//! `--permanent --yes` must leave every content-detected credential in place
//! and still clean the rest of the cache. Fake secrets in tempdirs only; HOME
//! and XDG directories point inside the tempdir, so the Trash is a fixture.
use std::{fs, path::Path, process::Command};

const OPENSSH: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAAFAKEQAFIXTURE\n-----END OPENSSH PRIVATE KEY-----\n";

fn baits() -> Vec<(&'static str, Vec<u8>)> {
    let mut late = "cache line without secrets\n".repeat(400).into_bytes();
    late.extend_from_slice(OPENSSH.as_bytes());
    vec![
        ("openssh", OPENSSH.as_bytes().to_vec()),
        (
            "rsa",
            b"-----BEGIN RSA PRIVATE KEY-----\nMIIFAKEQA\n-----END RSA PRIVATE KEY-----".to_vec(),
        ),
        (
            "pkcs8",
            b"-----BEGIN PRIVATE KEY-----\nMIIFAKEQA\n-----END PRIVATE KEY-----".to_vec(),
        ),
        (
            "aws-example-shaped",
            b"aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_vec(),
        ),
        ("aws-fake", b"aws_secret_access_key = FAKE".to_vec()),
        (
            "npm-token",
            b"//registry.npmjs.org/:_authToken=npm_FAKEFAKEFAKEFAKEFAKEFAKEFAKEFAKE1234".to_vec(),
        ),
        (
            "netrc",
            b"machine github.com login qa password ghp_FAKEFAKEFAKEFAKEFAKEFAKEFAKEFAKE1234"
                .to_vec(),
        ),
        ("after-4-kib", late),
    ]
}

/// (detector, cache directory, bait path inside the cache)
const TARGETS: [(&str, &str, &str); 7] = [
    ("python.pip-cache", ".cache/pip", "http/blob7f3a"),
    ("python.pip-cache", ".cache/pip", "http-v2/a/b/c/d.body"),
    ("npm.cache", ".npm/_cacache", "content-v2/sha512/ab/cd"),
    ("rust.target", "projects/rsproj/target", "debug/notes.txt"),
    ("rust.target", "projects/rsproj/target", "debug/blob"),
    (
        "node.modules",
        "projects/webapp/node_modules",
        "lodash/blob.js",
    ),
    ("next.cache", "projects/webapp/.next/cache", "fetch/blob"),
];

fn layout(root: &Path, cache: &str) {
    if cache.contains("rsproj") {
        fs::create_dir_all(root.join("projects/rsproj")).unwrap();
        fs::write(root.join("projects/rsproj/Cargo.toml"), "[package]\n").unwrap();
    }
    if cache.contains("webapp") {
        fs::create_dir_all(root.join("projects/webapp")).unwrap();
        fs::write(root.join("projects/webapp/package.json"), "{}").unwrap();
    }
    fs::create_dir_all(root.join(cache).join("zz-safe")).unwrap();
    fs::write(root.join(cache).join("zz-safe/ordinary"), "rebuildable").unwrap();
}

fn clean(root: &Path, detector: &str, permanent: bool) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_inodune-cli"));
    command
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", root)
        .env("XDG_CACHE_HOME", root.join(".cache"))
        .env("XDG_DATA_HOME", root.join(".local/share"))
        .env("XDG_STATE_HOME", root.join(".local/state"))
        .args(["clean", "--root"])
        .arg(root)
        .args(["--review", detector, "--yes"]);
    if permanent {
        command.arg("--permanent");
    }
    command.output().unwrap()
}

#[test]
fn content_credentials_survive_review_cleanup_in_every_target_and_mode() {
    for (detector, cache, bait) in TARGETS {
        for (label, bytes) in baits() {
            for permanent in [false, true] {
                let temp = tempfile::tempdir().unwrap();
                let root = temp.path();
                layout(root, cache);
                let bait_path = root.join(cache).join(bait);
                fs::create_dir_all(bait_path.parent().unwrap()).unwrap();
                fs::write(&bait_path, &bytes).unwrap();
                let output = clean(root, detector, permanent);
                let context = format!(
                    "{detector} {bait} {label} permanent={permanent}\nstdout: {}\nstderr: {}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(
                    fs::read(&bait_path).ok().as_deref(),
                    Some(bytes.as_slice()),
                    "credential moved or deleted: {context}"
                );
                assert!(
                    !root.join(cache).join("zz-safe/ordinary").exists(),
                    "the rest of the cache was not cleaned: {context}"
                );
                let trashed: Vec<_> = fs::read_dir(root.join(".local/share/Trash/files"))
                    .map(|dir| dir.map(|e| fs::read(e.unwrap().path()).unwrap()).collect())
                    .unwrap_or_default();
                for info in fs::read_dir(root.join(".local/share/Trash/info"))
                    .into_iter()
                    .flatten()
                {
                    let text = fs::read_to_string(info.unwrap().path()).unwrap();
                    assert!(!text.contains("/./"), "trashinfo Path has /./: {text}");
                }
                let stdout = String::from_utf8_lossy(&output.stdout);
                assert!(!stdout.contains("/./"), "CLI shows /./: {context}");
                assert!(!stdout.contains("1 itens"), "plural: {context}");
                if permanent {
                    assert!(trashed.is_empty(), "permanent used the Trash: {context}");
                } else {
                    assert_eq!(
                        trashed,
                        vec![b"rebuildable".to_vec()],
                        "only the safe file goes to the fixture Trash: {context}"
                    );
                }
            }
        }
    }
}

#[test]
fn name_protected_key_survives_and_cache_root_is_never_removed_wholesale() {
    for permanent in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        layout(root, ".cache/pip");
        let key = root.join(".cache/pip/http/id_ed25519");
        fs::create_dir_all(key.parent().unwrap()).unwrap();
        fs::write(&key, OPENSSH).unwrap();
        let output = clean(root, "python.pip-cache", permanent);
        assert!(key.exists(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(root.join(".cache/pip").is_dir());
        assert!(!root.join(".cache/pip/zz-safe/ordinary").exists());
    }
}

#[test]
fn package_docs_and_library_code_do_not_poison_the_cache() {
    let pem_literal = "const KEY: &str = \"-----BEGIN PRIVATE KEY-----\\nMIIFAKE\\n-----END PRIVATE KEY-----\";\n";
    let harmless: [(&str, &str); 6] = [
        (
            "npm-registry-fetch/lib/auth.js",
            "const token = conf[\"_authToken\"];\n",
        ),
        (
            "netrc/docs/doc.txt",
            "machine example.com login user password secret-from-the-docs\n",
        ),
        (
            "aws-sdk/README.md",
            "aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY\n",
        ),
        ("rustls/tests/rustls-test-vectors.rs", pem_literal),
        ("uuid/dist/id_generator.js", "module.exports = () => 1;\n"),
        ("dotenv/.env.example", "AWS_SECRET_ACCESS_KEY=YOUR_SECRET\n"),
    ];
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    layout(root, "projects/webapp/node_modules");
    for (path, content) in harmless {
        let file = root.join("projects/webapp/node_modules").join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, content).unwrap();
    }
    let output = clean(root, "node.modules", false);
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    for (path, _) in harmless {
        assert!(
            !root
                .join("projects/webapp/node_modules")
                .join(path)
                .exists(),
            "{path} was treated as a credential"
        );
    }
}

#[test]
fn review_accepts_scan_ids_and_aliases_and_lists_accepted_ids() {
    for alias in ["node.modules", "node_modules", "node-modules"] {
        let temp = tempfile::tempdir().unwrap();
        layout(temp.path(), "projects/webapp/node_modules");
        let output = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
            .args(["clean", "--dry-run", "--root"])
            .arg(temp.path())
            .args(["--review", alias])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{alias}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .args(["clean", "--dry-run", "--root", "/tmp", "--review", "bogus"])
        .output()
        .unwrap();
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(error.contains("node.modules"), "{error}");
    assert!(error.contains("python.pip-cache"), "{error}");
}
