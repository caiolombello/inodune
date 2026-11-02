use base64::{engine::general_purpose::STANDARD, Engine};
use inodune_core::{
    scan::{self, ApprovedRoot, ScanControl, ScanLimits, ScopeKind},
    Risk,
};
use std::{fs, path::Path};
fn key() -> String {
    let mut der = vec![0x30, 62];
    der.extend([0x02, 0x01, 0x00]);
    der.resize(64, 0);
    format!(
        "{}{}\n{}\n{}{}\n",
        "-----BEGIN ",
        "PRIVATE KEY-----",
        STANDARD.encode(der),
        "-----END ",
        "PRIVATE KEY-----"
    )
}
fn value() -> String {
    (0..40)
        .map(|i| b"aB3dE5gH7jK9mN2pQ4sT6vW8xY0zA1cD"[i % 32] as char)
        .collect()
}
#[test]
fn qa_baits_and_botocore_metadata_do_not_poison_the_cache() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/credential-cache-baits.json"
    ))
    .unwrap();
    assert_eq!(cases.as_array().unwrap().len(), 29);
    for case in cases.as_array().unwrap() {
        let temp = tempfile::tempdir().unwrap();
        let cache = temp.path().join("node_modules");
        let path = cache.join(case["path"].as_str().unwrap());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::create_dir_all(cache.join("safe-other")).unwrap();
        fs::write(&path, case["content"].as_str().unwrap()).unwrap();
        fs::write(cache.join("safe-other/ordinary"), "rebuildable cache").unwrap();
        let snapshot = scan::scan(
            &[ApprovedRoot::open(temp.path(), ScopeKind::Projects).unwrap()],
            &ScanControl::default(),
            &ScanLimits::default(),
            |_| {},
        )
        .unwrap();
        let item = snapshot
            .candidates
            .iter()
            .find(|c| c.detector_id == "node.modules")
            .unwrap();
        assert_eq!(
            item.risk,
            Risk::Review,
            "{}: {}",
            path.display(),
            item.reason
        );
        let target = &snapshot.targets[&item.id];
        let relative = Path::new("./node_modules").join(case["path"].as_str().unwrap());
        assert_eq!(
            target.files.iter().any(|(p, _)| p == &relative),
            !case["protected"].as_bool().unwrap(),
            "{}",
            path.display()
        );
        assert!(target
            .files
            .iter()
            .any(|(p, _)| p.ends_with("safe-other/ordinary")));
    }
    let metadata = b"Metadata-Version: 2.1\nName: botocore\nExample configuration:\naws_secret_access_key = YOUR_SECRET\n";
    assert!(!scan::credential_bytes(Path::new("METADATA"), metadata));
    let embedded = format!("const TEST_KEY: &str = r#\"{}\"#;", key());
    assert!(!scan::credential_bytes(
        Path::new("rustls-test-vectors.rs"),
        embedded.as_bytes()
    ));
    assert!(!scan::credential_bytes(
        Path::new("server.key"),
        b"not a key; just ordinary test code"
    ));
}
#[test]
fn actual_shapes_in_generic_names_and_nonplaceholder_cloud_values_stay_protected() {
    assert!(scan::credential_bytes(
        Path::new("notes.txt"),
        key().as_bytes()
    ));
    assert!(scan::credential_bytes(
        Path::new("notes.txt"),
        format!("aws_secret_access_key = {}", value()).as_bytes()
    ));
    assert!(scan::credential_bytes(
        Path::new("notes.txt"),
        format!("{{\"access_token\":\"{}\"}}", value()).as_bytes()
    ));
    assert!(scan::credential_bytes(
        Path::new(".npmrc"),
        format!("//registry.invalid/:_authToken={}", value()).as_bytes()
    ));
    for placeholder in [
        "YOUR_SECRET",
        "EXAMPLE_AWS_SECRET_ACCESS_KEY",
        "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        "<put-your-token-here>",
        "changeme",
    ] {
        assert!(!scan::credential_bytes(
            Path::new("METADATA"),
            format!("aws_secret_access_key = {placeholder}").as_bytes()
        ));
    }
    assert!(!scan::credential_bytes(
        Path::new("notes.txt"),
        b"-----BEGIN PRIVATE KEY-----\njust a cited header"
    ));
}
#[test]
fn protected_sibling_subtree_is_excluded_and_safe_files_remain_authorized() {
    let temp = tempfile::tempdir().unwrap();
    let cache = temp.path().join(".npm/_cacache");
    fs::create_dir_all(cache.join("with-key")).unwrap();
    fs::create_dir_all(cache.join("safe")).unwrap();
    fs::write(cache.join("with-key/notes.txt"), key()).unwrap();
    fs::write(
        cache.join("with-key/ordinary"),
        "also retained in immediate directory",
    )
    .unwrap();
    fs::write(cache.join("safe/ordinary"), "eligible").unwrap();
    let snapshot = scan::scan(
        &[ApprovedRoot::open(temp.path(), ScopeKind::Projects).unwrap()],
        &ScanControl::default(),
        &ScanLimits::default(),
        |_| {},
    )
    .unwrap();
    let item = snapshot
        .candidates
        .iter()
        .find(|c| c.detector_id == "npm.cache")
        .unwrap();
    assert_eq!(item.risk, Risk::Review, "{}", item.reason);
    let target = &snapshot.targets[&item.id];
    assert!(target.complete);
    assert_eq!(target.files.len(), 1);
    assert!(target.files[0].0.ends_with("safe/ordinary"));
    assert_eq!(target.preserved.len(), 1);
    assert!(target.preserved[0].ends_with("with-key"));
    assert!(snapshot
        .candidates
        .iter()
        .any(|c| c.risk == Risk::Protected && c.display_path.ends_with("with-key")));
    assert!(item.impact.contains("Lixeira"));
    assert!(!item.impact.contains("Inventário não autoriza"));
}
