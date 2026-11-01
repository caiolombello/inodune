//! Adversarial ownership-boundary lane. The fixture is created as root inside a
//! disposable container, then this test process runs as UID/GID 1000. Never run
//! against host paths: `INODUNE_DISCOVERY_OWNERSHIP_FIXTURE` must name the
//! harness-created fixture root.
use inodune_core::{
    discovery::discover_from,
    scan::{ScanControl, ScanLimits},
};
use std::{fs, os::unix::fs::MetadataExt, path::PathBuf};

#[test]
#[ignore = "run only in disposable Fedora container with the ownership fixture harness"]
fn ownership_boundaries_symlinks_noatime_limits_and_cancel_are_conservative() {
    assert_eq!(rustix::process::getuid().as_raw(), 1000);
    let fixture = PathBuf::from(
        std::env::var_os("INODUNE_DISCOVERY_OWNERSHIP_FIXTURE")
            .expect("harness must provide an isolated fixture path"),
    );
    let home = fixture.join("home");
    let structural = fixture.join("structural");
    let project = structural.join("owned-project");
    let foreign = structural.join("foreign-private");
    let outside = fixture.join("outside");
    let symlink_escape = outside.join("escape-project");
    for path in [&fixture, &home, &structural, &project, &foreign, &outside] {
        assert!(path.exists(), "missing fixture path: {}", path.display());
    }

    let tracked = [project.clone(), project.join("marker")];
    let before: Vec<_> = tracked
        .iter()
        .map(|p| fs::metadata(p).unwrap())
        .map(|m| {
            (
                m.ino(),
                m.atime(),
                m.atime_nsec(),
                m.mtime(),
                m.mtime_nsec(),
            )
        })
        .collect();

    let limits = ScanLimits::default();
    let got = discover_from(
        &home,
        "",
        std::slice::from_ref(&structural),
        &ScanControl::default(),
        &limits,
    )
    .expect("discovery on task-created fixture");
    assert!(got.roots.iter().any(|r| r.path() == home));
    assert!(got.roots.iter().any(|r| r.path() == project));
    assert!(!got.roots.iter().any(|r| r.path() == foreign));
    assert!(!got.roots.iter().any(|r| r.path() == outside));
    assert!(!got.roots.iter().any(|r| r.path() == symlink_escape));
    assert!(got
        .issues
        .iter()
        .any(|i| { i.code == "owner_protected" && i.display_path == foreign.to_string_lossy() }));
    assert!(tracked
        .iter()
        .map(|p| fs::metadata(p).unwrap())
        .map(|m| (
            m.ino(),
            m.atime(),
            m.atime_nsec(),
            m.mtime(),
            m.mtime_nsec()
        ))
        .eq(before));

    let bounded = ScanLimits {
        max_depth: 0,
        ..Default::default()
    };
    let limited = discover_from(
        &home,
        "",
        std::slice::from_ref(&structural),
        &ScanControl::default(),
        &bounded,
    )
    .unwrap();
    assert!(
        !limited.complete,
        "depth-limited traversal must not claim full coverage"
    );

    let cancelled = ScanControl::default();
    cancelled.cancel();
    let stopped = discover_from(
        &home,
        "",
        std::slice::from_ref(&structural),
        &cancelled,
        &limits,
    )
    .unwrap();
    assert!(!stopped.complete);
    assert!(stopped.roots.is_empty());
}
