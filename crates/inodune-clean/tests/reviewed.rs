use inodune_clean::{
    executor::{execute, ExecutionOptions, FileStatus},
    plan::{Action, PlanSession},
};
use inodune_core::{
    scan::{scan, ApprovedRoot, ScanControl, ScanLimits, ScopeKind},
    Risk, SupportedAction,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
struct Fixture {
    dir: tempfile::TempDir,
    root: PathBuf,
    provider: PathBuf,
    json: PathBuf,
    lock: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("projects");
        let project = root.join("project");
        let provider = project
            .join(".terraform/providers/registry.terraform.io/hashicorp/demo/1.2.3/linux_amd64");
        fs::create_dir_all(&provider).unwrap();
        let mut elf = vec![0u8; 64];
        elf[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        elf[18..20].copy_from_slice(&62u16.to_le_bytes());
        fs::write(provider.join("terraform-provider-demo_v1.2.3"), elf).unwrap();
        fs::write(provider.join("LICENSE.txt"), b"license text\n").unwrap();
        let lock = project.join(".terraform.lock.hcl");
        fs::write(&lock,b"provider \"registry.terraform.io/hashicorp/demo\" {\n version = \"1.2.4\"\n hashes = [\"h1:sBa6P0Es63rPk400M08oc+n2m1OBbCco8ayvM+ij82c=\"]\n}\n").unwrap();
        fs::write(project.join("main.tf"), b"terraform {}\n").unwrap();
        fs::write(project.join("terraform.tfstate"), b"private fixture state").unwrap();
        fs::write(
            project.join(".terraform/terraform.tfstate"),
            b"private backend",
        )
        .unwrap();
        fs::create_dir_all(project.join(".terraform/modules/local")).unwrap();
        fs::write(
            project.join(".terraform/modules/local/script.sh"),
            b"private script",
        )
        .unwrap();
        fs::write(
            project.join("Cargo.toml"),
            b"[package]\nname=\"fixture\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        let json =
            project.join("target/debug/.fingerprint/fixture-0123456789abcdef/lib-fixture.json");
        fs::create_dir_all(json.parent().unwrap()).unwrap();
        let value = serde_json::json!({"rustc":1,"features":"[]","declared_features":"[]","target":2,"profile":3,"path":4,"deps":[],"local":[],"rustflags":[],"config":5,"compile_kind":0});
        fs::write(&json, serde_json::to_vec(&value).unwrap()).unwrap();
        fs::write(json.with_extension(""), b"0123456789abcdef").unwrap();
        fs::write(project.join("target/final-binary"), b"keep final binary").unwrap();
        Self {
            dir,
            root,
            provider,
            json,
            lock,
        }
    }
    fn scan(&self) -> Arc<inodune_core::scan::ScanSnapshot> {
        Arc::new(
            scan(
                &[ApprovedRoot::open(&self.root, ScopeKind::Projects).unwrap()],
                &ScanControl::default(),
                &ScanLimits::default(),
                |_| {},
            )
            .unwrap(),
        )
    }
    fn options(&self) -> ExecutionOptions {
        ExecutionOptions {
            trash_directory: Some(self.dir.path().join("data/Trash")),
            history_directory: Some(self.dir.path().join("state/reports")),
        }
    }
    fn preserved(&self) {
        let project = self.lock.parent().unwrap();
        assert_eq!(
            fs::read(project.join("terraform.tfstate")).unwrap(),
            b"private fixture state"
        );
        assert_eq!(
            fs::read(project.join(".terraform/terraform.tfstate")).unwrap(),
            b"private backend"
        );
        assert_eq!(
            fs::read(project.join(".terraform/modules/local/script.sh")).unwrap(),
            b"private script"
        );
        assert_eq!(
            fs::read(project.join("target/final-binary")).unwrap(),
            b"keep final binary"
        );
        assert!(project.join("Cargo.toml").exists());
    }
}
fn selected(snapshot: &inodune_core::scan::ScanSnapshot, detector: &str) -> Vec<String> {
    snapshot
        .candidates
        .iter()
        .filter(|c| c.detector_id == detector && c.risk == Risk::Review)
        .map(|c| c.id.clone())
        .collect()
}
#[test]
fn real_scan_partitions_verified_leaves_without_double_counting_and_allows_a_batch() {
    let f = Fixture::new();
    let snapshot = f.scan();
    assert!(snapshot.complete, "{:?}", snapshot.issues);
    let tf = selected(&snapshot, "terraform.provider-package");
    let cargo = selected(&snapshot, "cargo.fingerprint-json");
    assert_eq!(tf.len(), 1, "{:?}", snapshot.candidates);
    assert_eq!(cargo.len(), 1);
    for id in tf.iter().chain(&cargo) {
        let c = snapshot.candidates.iter().find(|c| &c.id == id).unwrap();
        assert_eq!(c.risk, Risk::Review);
        assert!(!c.selected_by_default);
        assert!(matches!(
            c.supported_action,
            SupportedAction::ReviewedCacheContents
        ));
    }
    let parent = snapshot
        .candidates
        .iter()
        .find(|c| c.detector_id == "terraform.providers")
        .unwrap();
    assert_eq!(parent.logical_bytes, 0);
    assert_eq!(parent.risk, Risk::Protected);
    let parent = snapshot
        .candidates
        .iter()
        .find(|c| c.detector_id == "rust.target")
        .unwrap();
    assert_eq!(parent.logical_bytes, 16 + 17); // paired hash + final binary; JSON belongs to its own coverage.
    let plan = PlanSession::prepare(
        snapshot.clone(),
        &[tf[0].clone(), cargo[0].clone()],
        Action::Trash,
    )
    .unwrap();
    assert_eq!(plan.view().items.len(), 2);
    assert!(plan.view().review_required);
    let plan = PlanSession::prepare(snapshot, &tf, Action::Trash).unwrap();
    assert!(plan.view().review_required);
    f.preserved();
}
#[test]
fn reviewed_provider_cleanup_preserves_state_backend_modules_lock_and_parent_directories() {
    let f = Fixture::new();
    let snapshot = f.scan();
    let ids = selected(&snapshot, "terraform.provider-package");
    let p = PlanSession::prepare(snapshot, &ids, Action::Permanent).unwrap();
    let hash = p.view().hash;
    let report = execute(
        p.consume(&hash).unwrap(),
        &f.options(),
        &ScanControl::default(),
        |_| {},
    );
    assert!(report.complete, "{report:?}");
    assert_eq!(report.logical_bytes_processed, 77);
    assert!(f.provider.exists());
    assert_eq!(fs::read_dir(&f.provider).unwrap().count(), 0);
    assert!(f.lock.exists());
    f.preserved();
}
#[test]
fn cargo_json_only_is_removed_and_paired_hash_manifest_and_other_artifacts_stay() {
    let f = Fixture::new();
    let snapshot = f.scan();
    let ids = selected(&snapshot, "cargo.fingerprint-json");
    let p = PlanSession::prepare(snapshot, &ids, Action::Permanent).unwrap();
    let hash = p.view().hash;
    let report = execute(
        p.consume(&hash).unwrap(),
        &f.options(),
        &ScanControl::default(),
        |_| {},
    );
    assert!(report.complete, "{report:?}");
    assert!(!f.json.exists());
    assert_eq!(
        fs::read(f.json.with_extension("")).unwrap(),
        b"0123456789abcdef"
    );
    f.preserved();
}
#[test]
fn changed_lock_or_provider_never_reuses_the_scan_proof() {
    for change_lock in [true, false] {
        let f = Fixture::new();
        let snapshot = f.scan();
        let ids = selected(&snapshot, "terraform.provider-package");
        let p = PlanSession::prepare(snapshot, &ids, Action::Permanent).unwrap();
        let hash = p.view().hash;
        if change_lock {
            fs::write(&f.lock, b"new lock fixture").unwrap();
        } else {
            fs::write(
                f.provider.join("terraform-provider-demo_v1.2.3"),
                b"different payload",
            )
            .unwrap();
        }
        let report = execute(
            p.consume(&hash).unwrap(),
            &f.options(),
            &ScanControl::default(),
            |_| {},
        );
        assert!(!report.complete);
        assert_eq!(report.logical_bytes_processed, 0);
        assert!(f.provider.join("LICENSE.txt").exists());
        f.preserved();
    }
}
#[test]
fn changing_lock_between_leaves_yields_real_partial_result_without_rollback() {
    let f = Fixture::new();
    let snapshot = f.scan();
    let ids = selected(&snapshot, "terraform.provider-package");
    let p = PlanSession::prepare(snapshot, &ids, Action::Permanent).unwrap();
    let hash = p.view().hash;
    let report = execute(
        p.consume(&hash).unwrap(),
        &f.options(),
        &ScanControl::default(),
        |progress| {
            if progress.completed_files == 1 {
                fs::write(&f.lock, b"changed lock fixture").unwrap();
            }
        },
    );
    assert!(!report.complete);
    let files = &report.items[0].files;
    assert_eq!(
        files
            .iter()
            .filter(|f| f.status == FileStatus::Removed)
            .count(),
        1
    );
    assert_eq!(
        files
            .iter()
            .filter(|f| f.status == FileStatus::SkippedChanged)
            .count(),
        1
    );
    assert_eq!(fs::read_dir(&f.provider).unwrap().count(), 1);
    f.preserved();
}
#[test]
fn unconfirmed_provider_files_and_local_modules_remain_protected() {
    let f = Fixture::new();
    fs::write(f.provider.join("personal.txt"), b"private fixture").unwrap();
    let snapshot = f.scan();
    assert!(selected(&snapshot, "terraform.provider-package").is_empty());
    assert!(snapshot
        .candidates
        .iter()
        .filter(|c| c.detector_id.starts_with("terraform."))
        .all(|c| c.risk == Risk::Protected));
    assert!(Path::new(&f.lock).is_file());
    f.preserved();
}

#[test]
fn witnesses_writable_by_other_users_do_not_grant_review_actions() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    fs::set_permissions(&f.lock, fs::Permissions::from_mode(0o666)).unwrap();
    fs::set_permissions(f.json.with_extension(""), fs::Permissions::from_mode(0o666)).unwrap();
    let snapshot = f.scan();
    assert!(selected(&snapshot, "terraform.provider-package").is_empty());
    assert!(selected(&snapshot, "cargo.fingerprint-json").is_empty());
    f.preserved();
}
