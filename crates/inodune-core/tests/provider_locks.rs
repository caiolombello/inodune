use inodune_core::{
    scan::{scan, ApprovedRoot, ScanControl, ScanLimits, ScopeKind},
    Risk,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn provider(root: &Path, name: &str, version: &str) -> PathBuf {
    let rel = PathBuf::from(format!("infrastructure/modules/lambda/.terraform/providers/registry.terraform.io/hashicorp/{name}/{version}/linux_amd64"));
    fs::create_dir_all(root.join(&rel)).unwrap();
    // Intentionally not ELF: discovery must not inspect provider contents.
    fs::write(
        root.join(&rel)
            .join(format!("terraform-provider-{name}_v{version}")),
        b"not read during scan",
    )
    .unwrap();
    fs::write(root.join(&rel).join("LICENSE.txt"), b"fixture").unwrap();
    rel
}
fn lock(root: &Path, dir: &str, versions: &[(&str, &str)]) {
    fs::create_dir_all(root.join(dir)).unwrap();
    let text: String = versions.iter().map(|(n,v)| format!("provider \"registry.terraform.io/hashicorp/{n}\" {{ version = \"{v}\" hashes = [\"h1:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\"] }}\n")).collect();
    fs::write(root.join(dir).join(".terraform.lock.hcl"), text).unwrap();
}
#[test]
fn parent_and_environment_locks_protect_referenced_versions_without_reading_binaries() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join(".git")).unwrap();
    for (n, v) in [
        ("aws", "6.63.0"),
        ("aws", "6.64.0"),
        ("null", "3.3.0"),
        ("null", "3.3.2"),
        ("archive", "2.8.0"),
    ] {
        provider(temp.path(), n, v);
    }
    lock(
        temp.path(),
        "infrastructure",
        &[("aws", "6.64.0"), ("null", "3.3.2")],
    );
    lock(
        temp.path(),
        "infrastructure/environments/dev",
        &[("aws", "6.63.0")],
    );
    let root = ApprovedRoot::open(temp.path(), ScopeKind::Projects).unwrap();
    let result = scan(
        &[root],
        &ScanControl::default(),
        &ScanLimits::default(),
        |_| {},
    )
    .unwrap();
    let providers: Vec<_> = result
        .candidates
        .iter()
        .filter(|c| c.detector_id == "terraform.provider-package")
        .collect();
    assert_eq!(providers.len(), 5);
    for c in providers {
        assert_eq!(c.risk, Risk::Review, "{}: {}", c.display_path, c.reason);
        let referenced =
            !(c.display_path.contains("null/3.3.0/") || c.display_path.contains("archive/2.8.0/"));
        if referenced {
            assert!(
                c.reason.contains("terraform init") || c.reason.contains("referenciada por lock"),
                "{}",
                c.reason
            );
        }
    }
}

#[test]
fn bounds_lock_search_at_git_and_scan_roots_and_refuses_symlink_locks() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    fs::create_dir_all(repo.join(".git")).unwrap();
    provider(&repo, "aws", "6.63.0");
    lock(temp.path(), ".", &[("aws", "6.64.0")]);
    let run = |path: &Path| {
        let root = ApprovedRoot::open(path, ScopeKind::Projects).unwrap();
        scan(
            &[root],
            &ScanControl::default(),
            &ScanLimits::default(),
            |_| {},
        )
        .unwrap()
    };
    for path in [
        temp.path(),
        repo.as_path(),
        repo.join("infrastructure").as_path(),
    ] {
        let out = run(path);
        let c = out
            .candidates
            .iter()
            .find(|c| c.detector_id == "terraform.provider-package")
            .unwrap();
        assert_eq!(c.risk, Risk::Protected);
        assert!(c.reason.contains("sem lock"));
    }
    symlink(
        temp.path().join(".terraform.lock.hcl"),
        repo.join("infrastructure/.terraform.lock.hcl"),
    )
    .unwrap();
    let out = run(&repo);
    assert!(out
        .candidates
        .iter()
        .filter(|c| c.detector_id == "terraform.provider-package")
        .all(|c| c.risk == Risk::Protected));
}

#[test]
fn sparse_provider_is_not_hashed_during_scan_and_new_limit_is_explicit() {
    let temp = tempfile::tempdir().unwrap();
    let rel = provider(temp.path(), "aws", "6.63.0");
    lock(temp.path(), "infrastructure", &[("aws", "6.64.0")]);
    let binary = temp
        .path()
        .join(&rel)
        .join("terraform-provider-aws_v6.63.0");
    for (size, risk) in [
        (300 * 1024 * 1024, Risk::Review),
        (2 * 1024 * 1024 * 1024 + 1, Risk::Protected),
    ] {
        fs::OpenOptions::new()
            .write(true)
            .open(&binary)
            .unwrap()
            .set_len(size)
            .unwrap();
        let root = ApprovedRoot::open(temp.path(), ScopeKind::Projects).unwrap();
        let out = scan(
            &[root],
            &ScanControl::default(),
            &ScanLimits::default(),
            |_| {},
        )
        .unwrap();
        let c = out
            .candidates
            .iter()
            .find(|c| c.detector_id == "terraform.provider-package")
            .unwrap();
        assert_eq!(c.risk, risk, "{}", c.reason);
        if risk == Risk::Protected {
            assert!(c.reason.contains("acima do limite"));
        }
    }
}

#[test]
fn symlink_nearest_lock_or_environment_never_grants_a_review_plan() {
    use std::os::unix::fs::symlink;
    for environment in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        provider(temp.path(), "aws", "6.63.0");
        provider(temp.path(), "aws", "6.64.0");
        lock(temp.path(), "infrastructure", &[("aws", "6.64.0")]);
        let outside = tempfile::tempdir().unwrap();
        lock(outside.path(), ".", &[("aws", "6.63.0")]);
        if environment {
            fs::create_dir_all(temp.path().join("infrastructure/environments")).unwrap();
            symlink(
                outside.path(),
                temp.path().join("infrastructure/environments/dev"),
            )
            .unwrap();
        } else {
            symlink(
                outside.path().join(".terraform.lock.hcl"),
                temp.path()
                    .join("infrastructure/modules/lambda/.terraform.lock.hcl"),
            )
            .unwrap();
        }
        let root = ApprovedRoot::open(temp.path(), ScopeKind::Projects).unwrap();
        let out = scan(
            &[root],
            &ScanControl::default(),
            &ScanLimits::default(),
            |_| {},
        )
        .unwrap();
        let providers: Vec<_> = out
            .candidates
            .iter()
            .filter(|c| c.detector_id == "terraform.provider-package")
            .collect();
        assert_eq!(providers.len(), 2);
        for c in providers {
            assert_eq!(c.risk, Risk::Protected, "{}", c.reason);
            assert!(c.reason.contains("symlink"), "{}", c.reason);
            assert!(matches!(
                c.supported_action,
                inodune_core::SupportedAction::None
            ));
        }
    }
}
