use inodune_core::{
    discovery::discover_from,
    scan::{ScanControl, ScanLimits},
};
use std::fs;

#[test]
fn discovers_home_and_local_mount_roots_without_emitting_filesystem_root() {
    let t = tempfile::tempdir().unwrap();
    let home = t.path().join("home");
    let disk = t.path().join("disk with space");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&disk).unwrap();
    let mountinfo = format!(
        "42 1 8:1 / {} rw,relatime - ext4 /dev/fake rw\n",
        disk.display().to_string().replace(' ', "\\040")
    );
    let got = discover_from(
        &home,
        &mountinfo,
        &[],
        &ScanControl::default(),
        &ScanLimits::default(),
    )
    .unwrap();
    assert!(got.roots.iter().any(|r| r.path() == home));
    assert!(got.roots.iter().any(|r| r.path() == disk));
    assert!(got
        .roots
        .iter()
        .all(|r| r.path() != std::path::Path::new("/")));
    assert_eq!(got.volumes.len(), 1); // both fixture paths share one real filesystem
}

#[test]
fn never_follows_symlink_while_finding_owned_children_of_structural_seed() {
    let t = tempfile::tempdir().unwrap();
    let seed = t.path().join("seed");
    let owned = seed.join("owned-project");
    let outside = t.path().join("outside");
    fs::create_dir_all(&owned).unwrap();
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, seed.join("link")).unwrap();
    let got = discover_from(
        &seed,
        "",
        std::slice::from_ref(&seed),
        &ScanControl::default(),
        &ScanLimits::default(),
    )
    .unwrap();
    assert!(got.roots.iter().any(|r| r.path() == seed));
    assert!(!got.roots.iter().any(|r| r.path() == outside));
}

#[test]
fn rejects_malformed_mount_records_and_respects_cancel() {
    let t = tempfile::tempdir().unwrap();
    fs::create_dir(t.path().join("home")).unwrap();
    let control = ScanControl::default();
    control.cancel();
    let got = discover_from(
        &t.path().join("home"),
        "bad line\n",
        &[],
        &control,
        &ScanLimits::default(),
    )
    .unwrap();
    assert!(!got.complete);
    assert!(got.roots.is_empty());
}

#[test]
fn nested_roots_are_excluded_from_parent_without_losing_the_second_scan() {
    let t = tempfile::tempdir().unwrap();
    let home = t.path().join("home");
    let nested = home.join("disk");
    fs::create_dir_all(&nested).unwrap();
    let table = format!("42 1 8:1 / {} rw - ext4 /dev/fake rw\n", nested.display());
    let control = ScanControl::default();
    let limits = ScanLimits::default();
    let discovered = discover_from(&home, &table, &[], &control, &limits).unwrap();
    assert_eq!(discovered.roots.len(), 2);
    assert!(discovered
        .roots
        .iter()
        .find(|r| r.path() == home)
        .unwrap()
        .is_excluded(std::path::Path::new("disk")));
    assert!(
        inodune_core::discovery::scan_discovered(discovered, &control, &limits, |_| {}).is_ok()
    );
}

#[test]
fn network_home_is_not_reintroduced_by_structural_discovery() {
    let t = tempfile::tempdir().unwrap();
    let home = t.path().join("home");
    fs::create_dir(&home).unwrap();
    let table = format!(
        "42 1 8:1 / {} rw - nfs server:/fixture rw\n",
        home.display()
    );
    let got = discover_from(
        &home,
        &table,
        std::slice::from_ref(&home),
        &ScanControl::default(),
        &ScanLimits::default(),
    )
    .unwrap();
    assert!(!got.complete);
    assert!(got.roots.is_empty());
    assert!(got
        .issues
        .iter()
        .any(|i| i.code == "home_filesystem_protected"));
}
