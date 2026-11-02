//! QA regression (0.1.9): credentials detected by CONTENT must stay protected in
//! every cleanup target. All secrets below are fake and live in tempdirs.
use inodune_core::{
    scan::{self, ApprovedRoot, ScanControl, ScanLimits, ScopeKind},
    Risk,
};
use std::{fs, path::Path};

pub const OPENSSH: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAAFAKEQAFIXTURE\n-----END OPENSSH PRIVATE KEY-----\n";

/// The seven QA baits, byte for byte.
fn qa_baits() -> Vec<(&'static str, &'static [u8])> {
    vec![
        ("openssh", OPENSSH.as_bytes()),
        (
            "rsa",
            b"-----BEGIN RSA PRIVATE KEY-----\nMIIFAKEQA\n-----END RSA PRIVATE KEY-----",
        ),
        (
            "pkcs8",
            b"-----BEGIN PRIVATE KEY-----\nMIIFAKEQA\n-----END PRIVATE KEY-----",
        ),
        (
            "aws-example-shaped",
            b"aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
        ),
        ("aws-fake", b"aws_secret_access_key = FAKE"),
        (
            "npm-token",
            b"//registry.npmjs.org/:_authToken=npm_FAKEFAKEFAKEFAKEFAKEFAKEFAKEFAKE1234",
        ),
        (
            "netrc",
            b"machine github.com login qa password ghp_FAKEFAKEFAKEFAKEFAKEFAKEFAKEFAKE1234",
        ),
    ]
}

#[test]
fn every_qa_bait_is_a_credential_under_any_name() {
    for name in ["blob7f3a", "d.body", "notes.txt", "blob.js", "cd", "blob"] {
        for (label, bytes) in qa_baits() {
            assert!(
                scan::credential_bytes(Path::new(name), bytes),
                "{label} in {name}"
            );
        }
    }
}

#[test]
fn header_after_the_first_4_kib_is_still_found() {
    let mut text = "cache line without secrets\n".repeat(400);
    assert!(text.len() > 4096);
    text.push_str(OPENSSH);
    assert!(scan::credential_bytes(Path::new("blob"), text.as_bytes()));
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("late"), &text).unwrap();
    let root = ApprovedRoot::open(temp.path(), ScopeKind::Projects).unwrap();
    assert!(scan::sensitive_content(&root, Path::new("late")));
}

#[test]
fn false_positives_fixed_in_0_1_9_do_not_return() {
    let pem_literal = "const KEY: &str = \"-----BEGIN PRIVATE KEY-----\\nMIIFAKE\\n-----END PRIVATE KEY-----\";\n";
    let cases: Vec<(&str, &[u8])> = vec![
        (
            "site-packages/boto3-1.34.0.dist-info/METADATA",
            b"Metadata-Version: 2.1\nName: boto3\naws_secret_access_key = YOUR_SECRET\n",
        ),
        (
            "node_modules/aws-sdk/README.md",
            b"Set aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY in your config.\n",
        ),
        (
            "node_modules/npm-registry-fetch/lib/auth.js",
            b"const token = conf[\"_authToken\"];\nconst option = \"_authToken\";\n",
        ),
        ("target/debug/deps/rustls-test-vectors.rs", pem_literal.as_bytes()),
        (
            "node_modules/netrc/docs/doc.txt",
            b"machine example.com login user password secret-from-the-docs\n",
        ),
        ("target/debug/.env.example", b"AWS_SECRET_ACCESS_KEY=YOUR_SECRET"),
        ("target/debug/server.key", b"not a key; just ordinary test code"),
        ("target/debug/test.key", b"fn main() { println!(\"test\"); }"),
        ("target/debug/credentials.rs", b"pub struct Credentials { token: String }"),
        ("target/debug/private_key.rs", b"pub fn private_key() -> Vec<u8> { vec![] }"),
    ];
    for (path, bytes) in cases {
        assert!(!scan::credential_bytes(Path::new(path), bytes), "{path}");
    }
}

/// Scanner integration: the bait and its immediate directory are excluded from
/// the manifest, the cache stays reviewable and the rest is still eligible.
#[test]
fn scanner_preserves_only_the_bait_subtree_in_each_reviewable_target() {
    let targets: [(&str, &str, &str); 6] = [
        ("python.pip-cache", ".cache/pip", "http/blob7f3a"),
        ("python.pip-cache", ".cache/pip", "http-v2/a/b/c/d.body"),
        ("npm.cache", ".npm/_cacache", "content-v2/sha512/ab/cd"),
        ("rust.target", "projects/rsproj/target", "debug/notes.txt"),
        (
            "node.modules",
            "projects/webapp/node_modules",
            "lodash/blob.js",
        ),
        ("next.cache", "projects/webapp/.next/cache", "fetch/blob"),
    ];
    for (detector, cache, bait) in targets {
        for (label, bytes) in qa_baits() {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path();
            // pip is recognised only at $XDG_CACHE_HOME/pip; this is the only test
            // in this binary that reads the variable.
            std::env::set_var("XDG_CACHE_HOME", root.join(".cache"));
            fs::create_dir_all(root.join("projects/rsproj")).unwrap();
            fs::write(root.join("projects/rsproj/Cargo.toml"), "[package]\n").unwrap();
            fs::create_dir_all(root.join("projects/webapp")).unwrap();
            fs::write(root.join("projects/webapp/package.json"), "{}").unwrap();
            let bait_path = root.join(cache).join(bait);
            fs::create_dir_all(bait_path.parent().unwrap()).unwrap();
            fs::write(&bait_path, bytes).unwrap();
            fs::create_dir_all(root.join(cache).join("zz-safe")).unwrap();
            fs::write(root.join(cache).join("zz-safe/ordinary"), "rebuildable").unwrap();
            let snapshot = scan::scan(
                &[ApprovedRoot::open(root, ScopeKind::Projects).unwrap()],
                &ScanControl::default(),
                &ScanLimits::default(),
                |_| {},
            )
            .unwrap();
            let item = snapshot
                .candidates
                .iter()
                .find(|c| c.detector_id == detector)
                .unwrap_or_else(|| panic!("{detector} not found"));
            assert_eq!(
                item.risk,
                Risk::Review,
                "{detector}/{label}: {}",
                item.reason
            );
            let target = &snapshot.targets[&item.id];
            assert!(
                target
                    .files
                    .iter()
                    .all(|(p, _)| !p.ends_with(bait) && !p.to_string_lossy().contains(bait)),
                "{detector}/{label}: bait is in the manifest"
            );
            assert!(
                !target.preserved.is_empty(),
                "{detector}/{label}: no preserved subtree"
            );
            assert!(
                target
                    .files
                    .iter()
                    .any(|(p, _)| p.ends_with("zz-safe/ordinary")),
                "{detector}/{label}: the rest of the cache must stay eligible"
            );
        }
    }
}
