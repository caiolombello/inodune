//! Narrow allowlist for build-cache metadata that can be discarded safely.
//!
//! A matching path is only a documented Cargo fingerprint diagnostic JSON
//! leaf. This module does not authorize deleting its parent directory.
use crate::scan::{ApprovedRoot, Fingerprint};
use rustix::{
    fd::AsFd,
    fs::{self, FileType, OFlags},
};
use serde::Serialize;
use std::{
    io::Read,
    os::unix::ffi::OsStrExt,
    path::{Component, Path, PathBuf},
};

const MAX_FINGERPRINT_JSON: u64 = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Decision {
    pub eligible: bool,
    pub detector: &'static str,
    pub reason: &'static str,
    pub evidence: Vec<String>,
    pub rebuild_hint: &'static str,
}

impl Decision {
    fn cargo_json(eligible: bool, reason: &'static str, evidence: Vec<String>) -> Self {
        Self {
            eligible,
            detector: "cargo.fingerprint-json",
            reason,
            evidence,
            rebuild_hint:
                "Cargo pode regenerar esse metadado; uma recompilação pode ser necessária.",
        }
    }

    fn protected(reason: &'static str) -> Self {
        Self {
            eligible: false,
            detector: "build-cache.conservative",
            reason,
            evidence: Vec::new(),
            rebuild_hint: "Formato ou conteúdo não validado; preserve e revise manualmente.",
        }
    }
}

/// Classifies one proposed leaf using its path and the scanner's bounded inventory.
/// `files` is the scanner's inventory for the containing candidate; the requested
/// leaf must occur exactly once with a single-link fingerprint.
pub fn classify(
    root: &ApprovedRoot,
    relative: &Path,
    files: &[(PathBuf, Fingerprint)],
) -> Decision {
    let Some(paths) = cargo_fingerprint_paths(relative) else {
        return Decision::protected("Não pertence ao subconjunto reconhecido de metadados Cargo.");
    };
    if !has_cargo_manifest(root, &paths.manifest) {
        return Decision::protected("Raiz do target sem Cargo.toml verificável; preserve.");
    }
    let Some((_, fingerprint)) = files.iter().find(|(path, _)| path == relative) else {
        return Decision::protected("Folha ausente do inventário; preserve.");
    };
    if files.iter().filter(|(path, _)| path == relative).count() != 1 {
        return Decision::protected("Identidade da folha ambígua no inventário; preserve.");
    }
    if fingerprint.nlink != 1 || fingerprint.uid != rustix::process::getuid().as_raw() {
        return Decision::protected("Hardlink ou proprietário inesperado; preserve.");
    }
    let hash_path = relative.with_extension("");
    let Some((_, hash_fp)) = files.iter().find(|(path, _)| path == &hash_path) else {
        return Decision::protected("Hash pareado do fingerprint ausente do inventário; preserve.");
    };
    if files.iter().filter(|(path, _)| path == &hash_path).count() != 1
        || hash_fp.nlink != 1
        || hash_fp.uid != rustix::process::getuid().as_raw()
    {
        return Decision::protected("Identidade do hash pareado é ambígua; preserve.");
    }
    if validate_leaf(root, relative).as_ref() != Some(fingerprint) {
        return Decision::protected("Formato Cargo ou identidade atual não validado; preserve.");
    }
    Decision::cargo_json(
        true,
        "Par JSON de fingerprint Cargo confirmado em target padrão de projeto Rust; somente o JSON é elegível.",
        vec![
            "Cargo Book: target/{debug,release}/.fingerprint armazena fingerprints de unidades de build.".into(),
            "Cargo rustdoc: o JSON detalhado serve para explicar por que um fingerprint está sujo.".into(),
        ],
    )
}

/// Revalidates a proposed leaf via the retained root descriptor. It follows no
/// symlinks, reads at most 1 MiB, checks the known Cargo JSON shape, and compares
/// metadata before and after the read. Executors should also compare the returned
/// fingerprint to the inventory fingerprint immediately before unlinking.
pub fn validate_leaf(root: &ApprovedRoot, relative: &Path) -> Option<Fingerprint> {
    let paths = cargo_fingerprint_paths(relative)?;
    if !has_cargo_manifest(root, &paths.manifest) {
        return None;
    }
    let fd = root
        .open_relative(
            relative,
            OFlags::RDONLY | OFlags::NOATIME | OFlags::NONBLOCK,
        )
        .ok()?;
    let before = fs::fstat(&fd).ok()?;
    if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile
        || before.st_nlink != 1
        || before.st_uid != rustix::process::getuid().as_raw()
        || before.st_size < 0
        || before.st_size as u64 > MAX_FINGERPRINT_JSON
    {
        return None;
    }
    let expected = Fingerprint::from_stat(&before);
    let mut file = std::fs::File::from(fd);
    let mut bytes = Vec::with_capacity(expected.size as usize);
    file.by_ref()
        .take(MAX_FINGERPRINT_JSON + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 != expected.size || !cargo_fingerprint_json(&bytes) {
        return None;
    }
    let after = fs::fstat(file.as_fd()).ok()?;
    if Fingerprint::from_stat(&after) != expected {
        return None;
    }
    let reopened = root.open_relative(relative, OFlags::PATH).ok()?;
    if Fingerprint::from_stat(&fs::fstat(&reopened).ok()?) != expected {
        return None;
    }
    let hash_path = relative.with_extension("");
    if !valid_hash_pair(root, &hash_path) {
        return None;
    }
    Some(expected)
}

struct CargoFingerprintPaths {
    manifest: PathBuf,
}

fn cargo_fingerprint_paths(path: &Path) -> Option<CargoFingerprintPaths> {
    let parts: Vec<_> = path
        .components()
        .filter(|p| !matches!(p, Component::CurDir))
        .collect();
    if parts.len() < 4 || parts.iter().any(|p| !matches!(p, Component::Normal(_))) {
        return None;
    }
    let target_index = parts
        .iter()
        .position(|p| matches!(p, Component::Normal(x) if x.as_bytes() == b"target"))?;
    if parts.len() != target_index + 5
        || !matches!(parts.get(target_index + 1), Some(Component::Normal(x)) if x.as_bytes() == b"debug" || x.as_bytes() == b"release")
        || !matches!(parts.get(target_index + 2), Some(Component::Normal(x)) if x.as_bytes() == b".fingerprint")
    {
        return None;
    }
    let Component::Normal(unit_dir) = parts[target_index + 3] else {
        return None;
    };
    let dir_name = unit_dir.to_str()?;
    let (_, package_hash) = dir_name.rsplit_once('-')?;
    if !is_hex16(package_hash) {
        return None;
    }
    let Component::Normal(json_name) = parts[target_index + 4] else {
        return None;
    };
    let json_name = json_name.to_str()?;
    let stem = json_name.strip_suffix(".json")?;
    if stem.is_empty()
        || !stem
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return None;
    }
    let project = parts[..target_index]
        .iter()
        .fold(PathBuf::new(), |mut path, part| {
            if let Component::Normal(name) = part {
                path.push(name);
            }
            path
        });
    Some(CargoFingerprintPaths {
        manifest: project.join("Cargo.toml"),
    })
}

fn is_hex16(value: &str) -> bool {
    value.len() == 16 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Cargo's serialized diagnostic fingerprint has these required fields. Keeping
/// the shape check strict intentionally protects metadata from unknown formats.
fn cargo_fingerprint_json(bytes: &[u8]) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return false;
    };
    let Some(object) = value.as_object() else {
        return false;
    };
    let fields = [
        "rustc",
        "features",
        "declared_features",
        "target",
        "profile",
        "path",
        "deps",
        "local",
        "rustflags",
        "config",
        "compile_kind",
    ];
    object.len() == fields.len()
        && fields.iter().all(|key| object.contains_key(*key))
        && ["rustc", "target", "profile", "path", "config"]
            .iter()
            .all(|key| object.get(*key).is_some_and(serde_json::Value::is_u64))
        && ["features", "declared_features"]
            .iter()
            .all(|key| object.get(*key).is_some_and(serde_json::Value::is_string))
        && object.get("deps").is_some_and(serde_json::Value::is_array)
        && object.get("local").is_some_and(serde_json::Value::is_array)
        && object
            .get("rustflags")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|items| items.iter().all(serde_json::Value::is_string))
        && object
            .get("compile_kind")
            .is_some_and(serde_json::Value::is_u64)
}

fn has_cargo_manifest(root: &ApprovedRoot, path: &Path) -> bool {
    let Ok(fd) = root.open_relative(path, OFlags::RDONLY | OFlags::NOATIME | OFlags::NONBLOCK)
    else {
        return false;
    };
    let Ok(before) = fs::fstat(&fd) else {
        return false;
    };
    if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile
        || before.st_nlink != 1
        || before.st_uid != rustix::process::getuid().as_raw()
        || before.st_size < 0
        || before.st_size as u64 > MAX_FINGERPRINT_JSON
    {
        return false;
    }
    let expected = Fingerprint::from_stat(&before);
    let mut file = std::fs::File::from(fd);
    let mut bytes = Vec::with_capacity(expected.size as usize);
    if file
        .by_ref()
        .take(MAX_FINGERPRINT_JSON + 1)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return false;
    }
    if bytes.len() as u64 != expected.size
        || fs::fstat(file.as_fd())
            .map(|st| Fingerprint::from_stat(&st) != expected)
            .unwrap_or(true)
    {
        return false;
    }
    let Ok(reopened) = root.open_relative(path, OFlags::PATH) else {
        return false;
    };
    if fs::fstat(&reopened)
        .map(|st| Fingerprint::from_stat(&st) != expected)
        .unwrap_or(true)
    {
        return false;
    }
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return false;
    };
    text.lines()
        .map(str::trim)
        .any(|line| line == "[package]" || line == "[workspace]")
}

fn valid_hash_pair(root: &ApprovedRoot, path: &Path) -> bool {
    let Ok(fd) = root.open_relative(path, OFlags::RDONLY | OFlags::NOATIME | OFlags::NONBLOCK)
    else {
        return false;
    };
    let Ok(before) = fs::fstat(&fd) else {
        return false;
    };
    if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile
        || before.st_nlink != 1
        || before.st_uid != rustix::process::getuid().as_raw()
        || before.st_size != 16
    {
        return false;
    }
    let expected = Fingerprint::from_stat(&before);
    let mut file = std::fs::File::from(fd);
    let mut bytes = [0u8; 16];
    if file.read_exact(&mut bytes).is_err()
        || !bytes
            .iter()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return false;
    }
    if fs::fstat(file.as_fd())
        .map(|st| Fingerprint::from_stat(&st) != expected)
        .unwrap_or(true)
    {
        return false;
    }
    let Ok(reopened) = root.open_relative(path, OFlags::PATH) else {
        return false;
    };
    fs::fstat(&reopened)
        .map(|st| Fingerprint::from_stat(&st) == expected)
        .unwrap_or(false)
}

/// Non-deletable evidence that must remain unchanged between scan and cleanup.
pub fn witnesses(root: &ApprovedRoot, relative: &Path) -> Option<Vec<(PathBuf, Fingerprint)>> {
    let paths = cargo_fingerprint_paths(relative)?;
    [paths.manifest, relative.with_extension("")]
        .into_iter()
        .map(|path| {
            let fd = root.open_relative(&path, OFlags::PATH).ok()?;
            let stat = fs::fstat(fd).ok()?;
            (FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile
                && stat.st_nlink == 1
                && stat.st_mode & 0o022 == 0
                && stat.st_uid == rustix::process::getuid().as_raw())
            .then(|| (path, Fingerprint::from_stat(&stat)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::ScopeKind;
    use std::{fs, os::unix::fs::MetadataExt};
    use tempfile::tempdir;

    fn root() -> (tempfile::TempDir, ApprovedRoot) {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("project")).unwrap();
        fs::write(
            dir.path().join("project/Cargo.toml"),
            b"[package]\nname = \"cargo-cache-fixture\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let root = ApprovedRoot::open(dir.path(), ScopeKind::Projects).unwrap();
        (dir, root)
    }

    fn make_file(root: &Path, rel: &Path, bytes: &[u8]) -> Fingerprint {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        let md = fs::symlink_metadata(path).unwrap();
        Fingerprint {
            dev: md.dev(),
            ino: md.ino(),
            size: md.size(),
            blocks: md.blocks(),
            mode: md.mode(),
            nlink: md.nlink(),
            mtime: md.mtime(),
            mtime_nsec: md.mtime_nsec() as u64,
            ctime: md.ctime(),
            ctime_nsec: md.ctime_nsec() as u64,
            uid: md.uid(),
        }
    }

    fn known_json() -> &'static [u8] {
        br#"{"rustc":5470738401306409665,"features":"[]","declared_features":"[]","target":6563865812267954449,"profile":1722584277633009122,"path":5168694791878198938,"deps":[],"local":[{"CheckDepInfo":{"dep_info":"debug/.fingerprint/demo-0123456789abcdef/dep-lib-demo","checksum":false}}],"rustflags":[],"config":7646935837425161248,"compile_kind":0}"#
    }

    fn make_pair(root: &Path, rel: &Path, json: &[u8]) -> Vec<(PathBuf, Fingerprint)> {
        let json_fp = make_file(root, rel, json);
        let hash_path = rel.with_extension("");
        let hash_fp = make_file(root, &hash_path, b"0123456789abcdef");
        vec![(rel.to_path_buf(), json_fp), (hash_path, hash_fp)]
    }

    #[test]
    fn accepts_only_known_cargo_fingerprint_diagnostic_leaf() {
        let (dir, root) = root();
        let rel =
            Path::new("project/target/debug/.fingerprint/demo-0123456789abcdef/lib-demo.json");
        let files = make_pair(dir.path(), rel, known_json());
        let fp = files[0].1.clone();
        assert!(classify(&root, rel, &files).eligible);
        assert_eq!(validate_leaf(&root, rel), Some(fp));
    }

    #[test]
    fn protects_unknown_paths_content_and_identity() {
        let (dir, root) = root();
        let rel =
            Path::new("project/target/debug/.fingerprint/demo-0123456789abcdef/lib-demo.json");
        let files = make_pair(dir.path(), rel, b"{\"user\":\"notes\"}");
        assert!(!classify(&root, rel, &files).eligible);
        assert!(validate_leaf(&root, rel).is_none());
        for other in [
            "project/target/debug/.fingerprint/demo-0123456789abcdef/custom.json",
            "target/incremental/private.json",
            ".terraform/modules/registry.example/x/main.tf",
            ".terraform/providers/registry.terraform.io/acme/tool/1.0/linux_amd64/terraform-provider-tool_v1.0",
            "terraform.tfstate",
            ".terraform.lock.hcl",
        ] {
            assert!(!classify(&root, Path::new(other), &[]).eligible, "{other}");
            assert!(validate_leaf(&root, Path::new(other)).is_none(), "{other}");
        }
    }

    #[test]
    fn protects_missing_duplicate_hardlink_and_symlink_leaves() {
        let (dir, root) = root();
        let rel =
            Path::new("project/target/release/.fingerprint/demo-0123456789abcdef/lib-demo.json");
        let files = make_pair(dir.path(), rel, known_json());
        let fp = files[0].1.clone();
        assert!(!classify(&root, rel, &[]).eligible);
        let mut duplicate = files.clone();
        duplicate.push((rel.to_path_buf(), fp));
        assert!(!classify(&root, rel, &duplicate).eligible);
        let linked =
            Path::new("project/target/release/.fingerprint/demo-0123456789abcdef/bin-demo.json");
        let linked_files = make_pair(dir.path(), linked, known_json());
        fs::hard_link(dir.path().join(linked), dir.path().join("linked-copy")).unwrap();
        assert!(!classify(&root, linked, &linked_files).eligible);
        assert!(validate_leaf(&root, linked).is_none());
        let symlink = Path::new(
            "project/target/release/.fingerprint/demo-0123456789abcdef/test-lib-demo.json",
        );
        make_pair(dir.path(), symlink, known_json());
        fs::remove_file(dir.path().join(symlink)).unwrap();
        std::os::unix::fs::symlink(dir.path().join(rel), dir.path().join(symlink)).unwrap();
        assert!(validate_leaf(&root, symlink).is_none());
    }
}
