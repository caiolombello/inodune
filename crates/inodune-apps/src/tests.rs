use super::*;
#[test]
fn rejects_hostile_ids() {
    for id in [
        "-option",
        "has space",
        "x;y",
        "$(id)",
        "../escape",
        "a/../b",
        "",
        ".a",
    ] {
        assert!(!valid_id(id), "{id}");
    }
    for id in ["org.example.App", "libc6:amd64", "gcc-c++", "vim"] {
        assert!(valid_id(id));
    }
}
#[test]
fn distro_families() {
    for (s, f) in [
        ("ID=ubuntu\nID_LIKE=debian", "debian"),
        ("ID=debian", "debian"),
        ("ID=fedora", "fedora"),
        ("ID=derivative\nID_LIKE=\"rhel fedora\"", "fedora"),
    ] {
        assert!(Distro::parse(s).family(&[f]));
    }
    assert_eq!(Distro::parse("ID=$(command)\nBAD").id, "$(command)"); // data, never executed
}
