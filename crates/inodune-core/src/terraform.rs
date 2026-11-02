//! Positive, read-only verification of installed Terraform provider packages.
//!
//! An installed package is eligible only when its complete flat file set is
//! understood, all files are owned single-link regular files, its executable
//! has the expected ELF machine, and the content-derived h1 checksum matches
//! the canonical project's lock file. Terraform modules are intentionally not
//! classified here: the dependency lock file does not authenticate them.

use crate::scan::{ApprovedRoot, Fingerprint, ScanControl};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rustix::fs::{self, FileType, OFlags, RawDir};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    mem::MaybeUninit,
    path::{Component, Path, PathBuf},
};

const MAX_LOCK_BYTES: u64 = 1024 * 1024;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_PACKAGE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_PACKAGE_FILES: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verification {
    pub eligible: bool,
    pub reason: String,
    pub evidence: Vec<String>,
    pub rebuild_hint: String,
    /// Lock file identity used for the positive decision; consumers should
    /// compare this witness before and after any later content capture.
    pub lock_relative: Option<PathBuf>,
    pub lock_fingerprint: Option<Fingerprint>,
}

impl Verification {
    fn reject(reason: impl Into<String>) -> Self {
        Self {
            eligible: false,
            reason: reason.into(),
            evidence: vec!["terraform-provider-integrity-unproven".into()],
            rebuild_hint: "Preserve the complete provider package and review its lock file before considering any rebuild.".into(),
            lock_relative: None,
            lock_fingerprint: None,
        }
    }
}

/// Return true only for the standard installed package path shape. This is a
/// discovery classifier, not an eligibility decision; callers must still call
/// [`verify_provider`] before assigning any cleanup action.
pub fn is_provider_package_path(package_rel: &Path) -> bool {
    parse_package_path(package_rel).is_some()
}

#[derive(Clone, Debug)]
struct PackagePath {
    project: PathBuf,
    registry: String,
    namespace: String,
    provider_type: String,
    version: String,
    platform: String,
}

fn parse_package_path(path: &Path) -> Option<PackagePath> {
    let mut parts = Vec::new();
    let mut saw_normal = false;
    for component in path.components() {
        match component {
            Component::CurDir if !saw_normal && parts.is_empty() => {}
            Component::Normal(v) => {
                saw_normal = true;
                parts.push(v.to_str()?)
            }
            _ => return None,
        }
    }
    if parts.len() < 7
        || parts[parts.len() - 7] != ".terraform"
        || parts[parts.len() - 6] != "providers"
    {
        return None;
    }
    let tail = &parts[parts.len() - 5..];
    let [registry, namespace, provider_type, version, platform] = tail else {
        return None;
    };
    if !valid_registry(registry)
        || !valid_identifier(namespace)
        || !valid_identifier(provider_type)
        || !valid_semver(version)
        || !matches!(*platform, "linux_amd64" | "linux_arm64")
    {
        return None;
    }
    let mut project = PathBuf::new();
    for part in &parts[..parts.len() - 7] {
        project.push(part);
    }
    Some(PackagePath {
        project,
        registry: (*registry).to_string(),
        namespace: (*namespace).to_string(),
        provider_type: (*provider_type).to_string(),
        version: (*version).to_string(),
        platform: (*platform).to_string(),
    })
}

fn valid_registry(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 253
        && s.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.as_bytes()[0].is_ascii_alphanumeric()
                && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

fn valid_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

fn valid_semver(s: &str) -> bool {
    let (without_build, build_ok) = match s.split_once('+') {
        Some((left, right)) => (
            left,
            !right.is_empty() && right.split('.').all(valid_semver_id),
        ),
        None => (s, true),
    };
    if !build_ok {
        return false;
    }
    let (core, pre_ok) = match without_build.split_once('-') {
        Some((left, right)) => (
            left,
            !right.is_empty() && right.split('.').all(valid_semver_id),
        ),
        None => (without_build, true),
    };
    if !pre_ok {
        return false;
    }
    let nums: Vec<_> = core.split('.').collect();
    nums.len() == 3
        && nums.iter().all(|n| {
            !n.is_empty()
                && n.bytes().all(|b| b.is_ascii_digit())
                && (n == &"0" || !n.starts_with('0'))
        })
}

fn valid_semver_id(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// Verify a provider package relative to the already-approved project root.
/// `files` is the scan-time manifest; package files are independently listed
/// and reopened under the retained root descriptor so that omitted or newly
/// added entries fail closed. The caller must repeat this verification before
/// acting on a prior scan result.
pub fn verify_provider(
    root: &ApprovedRoot,
    package_rel: &Path,
    files: &[(PathBuf, Fingerprint)],
    control: &ScanControl,
) -> Verification {
    let Some(pkg) = parse_package_path(package_rel) else {
        return Verification::reject("Path is not a supported installed Linux provider package.");
    };
    if root.revalidate().is_err() {
        return Verification::reject(
            "Approved project root changed; provider contents are preserved.",
        );
    }
    if !control.checkpoint() {
        return Verification::reject("Verification cancelled; provider contents are preserved.");
    }

    let Ok(dir_fd) = root.open_relative(
        package_rel,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOATIME,
    ) else {
        return Verification::reject(
            "Provider directory could not be opened without following links or crossing mounts.",
        );
    };
    let Ok(dir_before) = fs::fstat(&dir_fd) else {
        return Verification::reject("Provider directory metadata could not be read.");
    };
    if FileType::from_raw_mode(dir_before.st_mode) != FileType::Directory
        || dir_before.st_uid != rustix::process::getuid().as_raw()
    {
        return Verification::reject("Provider directory type or ownership is not eligible.");
    }

    let mut actual_names = Vec::<String>::new();
    let mut raw_buf = [MaybeUninit::uninit(); 8192];
    {
        let mut entries = RawDir::new(&dir_fd, &mut raw_buf);
        while let Some(entry) = entries.next() {
            if !control.checkpoint() {
                return Verification::reject(
                    "Verification cancelled; provider contents are preserved.",
                );
            }
            let Ok(entry) = entry else {
                return Verification::reject("Provider directory enumeration was incomplete.");
            };
            let bytes = entry.file_name().to_bytes();
            if bytes == b"." || bytes == b".." {
                continue;
            }
            let Ok(name) = std::str::from_utf8(bytes) else {
                return Verification::reject("Provider package contains an unrecognized pathname.");
            };
            actual_names.push(name.to_string());
            if actual_names.len() > MAX_PACKAGE_FILES {
                return Verification::reject("Provider package exceeds the bounded file count.");
            }
        }
    }
    let Ok(dir_after) = fs::fstat(&dir_fd) else {
        return Verification::reject(
            "Provider directory post-enumeration metadata could not be read.",
        );
    };
    if Fingerprint::from_stat(&dir_after) != Fingerprint::from_stat(&dir_before) {
        return Verification::reject(
            "Provider directory changed during enumeration; preserve the package.",
        );
    }
    actual_names.sort();
    if actual_names.is_empty() {
        return Verification::reject("Provider package is empty.");
    }

    let binary_prefix = format!("terraform-provider-{}_v{}", pkg.provider_type, pkg.version);
    let mut binary_name: Option<String> = None;
    let mut license_count = 0;
    for name in &actual_names {
        if name == "LICENSE" || name == "LICENSE.txt" {
            license_count += 1;
            continue;
        }
        if valid_binary_name(name, &binary_prefix) && binary_name.is_none() {
            binary_name = Some(name.clone());
        } else {
            return Verification::reject("Provider package has an unexpected file or ambiguous executable; preserve the whole package.");
        }
    }
    let Some(binary_name) = binary_name else {
        return Verification::reject("Expected provider executable is missing.");
    };
    if license_count != 1 {
        return Verification::reject(
            "Provider package must contain exactly one LICENSE or LICENSE.txt file.",
        );
    }

    let expected_manifest: BTreeMap<PathBuf, &Fingerprint> = files
        .iter()
        .filter(|(path, _)| path.parent() == Some(package_rel))
        .map(|(path, fingerprint)| (path.clone(), fingerprint))
        .collect();
    if expected_manifest.len() != actual_names.len()
        || actual_names
            .iter()
            .any(|name| !expected_manifest.contains_key(&package_rel.join(name)))
    {
        return Verification::reject("Provider package scan manifest differs from the current directory; preserve the package.");
    }

    let mut total = 0u64;
    let mut entries_for_hash = Vec::<(String, [u8; 32])>::new();
    let mut binary_machine_ok = false;
    for name in &actual_names {
        if !control.checkpoint() {
            return Verification::reject(
                "Verification cancelled; provider contents are preserved.",
            );
        }
        let rel = package_rel.join(name);
        let Ok(fd) = root.open_relative(&rel, OFlags::RDONLY | OFlags::NOATIME | OFlags::NONBLOCK)
        else {
            return Verification::reject("Package entry is a link, special file, inaccessible, or crosses a mount; preserve the package.");
        };
        let Ok(before) = fs::fstat(&fd) else {
            return Verification::reject("Package entry metadata could not be read.");
        };
        let before_fp = Fingerprint::from_stat(&before);
        if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile
            || before.st_uid != rustix::process::getuid().as_raw()
            || before.st_nlink != 1
            || before.st_mode & 0o022 != 0
            || before.st_size < 0
            || before.st_size as u64 > MAX_FILE_BYTES
            || expected_manifest
                .get(&rel)
                .is_none_or(|expected| *expected != &before_fp)
        {
            return Verification::reject("Package entry is not an owned, single-link regular file matching the scan manifest.");
        }
        total = match total.checked_add(before.st_size as u64) {
            Some(n) if n <= MAX_PACKAGE_BYTES => n,
            _ => return Verification::reject("Provider package exceeds the bounded content size."),
        };
        let mut file = std::fs::File::from(fd);
        let mut content_hash = Sha256::new();
        let mut binary_header = Vec::with_capacity(20);
        let is_binary = name == &binary_name;
        let mut count = 0u64;
        let mut chunk = [0u8; 64 * 1024];
        loop {
            if !control.checkpoint() {
                return Verification::reject(
                    "Verification cancelled; provider contents are preserved.",
                );
            }
            let n = match file.read(&mut chunk) {
                Ok(n) => n,
                Err(_) => {
                    return Verification::reject("Package contents could not be read completely.")
                }
            };
            if n == 0 {
                break;
            }
            count = match count.checked_add(n as u64) {
                Some(n) if n <= MAX_FILE_BYTES && n <= before.st_size as u64 => n,
                _ => {
                    return Verification::reject("Package file changed or exceeded its read bound.")
                }
            };
            content_hash.update(&chunk[..n]);
            if is_binary && binary_header.len() < 20 {
                let take = (20 - binary_header.len()).min(n);
                binary_header.extend_from_slice(&chunk[..take]);
            }
        }
        let Ok(after) = fs::fstat(&file) else {
            return Verification::reject("Package entry post-read metadata could not be read.");
        };
        if count != before.st_size as u64 || Fingerprint::from_stat(&after) != before_fp {
            return Verification::reject(
                "Package entry changed while being verified; preserve the package.",
            );
        }
        if is_binary {
            binary_machine_ok = elf_machine_matches(&binary_header, &pkg.platform);
        }
        let rel_name = name.clone();
        entries_for_hash.push((rel_name, content_hash.finalize().into()));
    }
    let Ok(current_dir_fd) = root.open_relative(
        package_rel,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOATIME,
    ) else {
        return Verification::reject(
            "Provider package directory changed during verification; preserve the package.",
        );
    };
    let Ok(current_dir) = fs::fstat(&current_dir_fd) else {
        return Verification::reject(
            "Provider package directory post-read metadata could not be read.",
        );
    };
    if Fingerprint::from_stat(&current_dir) != Fingerprint::from_stat(&dir_before) {
        return Verification::reject(
            "Provider package directory changed during content verification; preserve the package.",
        );
    }
    if !binary_machine_ok {
        return Verification::reject(
            "Provider executable does not have a valid ELF header for the installed platform.",
        );
    }

    let actual_h1 = match hash1(entries_for_hash) {
        Some(value) => value,
        None => {
            return Verification::reject(
                "Provider package pathnames cannot be represented by Terraform h1.",
            )
        }
    };
    let lock_rel = pkg.project.join(".terraform.lock.hcl");
    let Ok(lock_fd) = root.open_relative(
        &lock_rel,
        OFlags::RDONLY | OFlags::NOATIME | OFlags::NONBLOCK,
    ) else {
        return Verification::reject(
            "Canonical project lock file is missing or cannot be opened safely.",
        );
    };
    let Ok(lock_before) = fs::fstat(&lock_fd) else {
        return Verification::reject("Canonical project lock metadata could not be read.");
    };
    if FileType::from_raw_mode(lock_before.st_mode) != FileType::RegularFile
        || lock_before.st_uid != rustix::process::getuid().as_raw()
        || lock_before.st_nlink != 1
        || lock_before.st_mode & 0o022 != 0
        || lock_before.st_size < 0
        || lock_before.st_size as u64 > MAX_LOCK_BYTES
    {
        return Verification::reject(
            "Canonical project lock is not a bounded owned single-link regular file.",
        );
    }
    let mut lock_file = std::fs::File::from(lock_fd);
    let mut lock_bytes = Vec::with_capacity(lock_before.st_size as usize);
    let mut limited = (&mut lock_file).take(MAX_LOCK_BYTES + 1);
    if limited.read_to_end(&mut lock_bytes).is_err() || lock_bytes.len() as u64 > MAX_LOCK_BYTES {
        return Verification::reject(
            "Canonical project lock exceeds the bounded read limit or is unreadable.",
        );
    }
    let Ok(lock_after) = fs::fstat(&lock_file) else {
        return Verification::reject(
            "Canonical project lock post-read metadata could not be read.",
        );
    };
    let lock_fp = Fingerprint::from_stat(&lock_before);
    if Fingerprint::from_stat(&lock_after) != lock_fp
        || lock_bytes.len() as u64 != lock_before.st_size as u64
    {
        return Verification::reject("Canonical project lock changed while being verified.");
    }
    let Ok(lock_text) = std::str::from_utf8(&lock_bytes) else {
        return Verification::reject("Canonical project lock is not valid UTF-8.");
    };
    let lock_map = match parse_lockfile(lock_text) {
        Ok(map) => map,
        Err(reason) => return Verification::reject(reason),
    };
    let address = format!("{}/{}/{}", pkg.registry, pkg.namespace, pkg.provider_type);
    let Some(record) = lock_map.get(&address) else {
        return Verification::reject("Provider has no matching canonical lock-file entry.");
    };
    if record.version != pkg.version {
        return Verification::reject(
            "Installed provider version differs from the canonical lock selection.",
        );
    }
    if !record.hashes.iter().any(|h| h == &actual_h1) {
        return Verification::reject(
            "Installed provider content does not match an h1 checksum in the canonical lock file.",
        );
    }

    Verification {
        eligible: true,
        reason: "Provider package content matches its canonical Terraform lock-file h1 checksum.".into(),
        evidence: vec![
            "terraform-provider-h1-verified".into(),
            format!("package-h1:{actual_h1}"),
            format!("lock-fingerprint:{}:{}:{}:{}:{}", lock_fp.dev, lock_fp.ino, lock_fp.size, lock_fp.mtime, lock_fp.ctime),
            format!("platform:{}", pkg.platform),
        ],
        rebuild_hint: "The authenticated provider package can be restored by running Terraform init for this project and platform after preserving the lock file.".into(),
        lock_relative: Some(lock_rel),
        lock_fingerprint: Some(lock_fp),
    }
}

/// Decisão sem leitura dos binários. Locks considerados: o mais próximo na
/// subida, até a primeira raiz Git ou raiz aprovada; também todos os locks em
/// environments/* de cada ancestral dentro desse limite. Não inferimos uso.
/// Links nunca são seguidos. Locks ilegíveis ou sintaxe desconhecida recusam.
#[derive(Clone, Debug)]
pub struct ProviderAssessment {
    pub eligible: bool,
    pub reason: String,
    pub witnesses: Vec<(PathBuf, Fingerprint)>,
}
pub fn assess_provider(
    root: &ApprovedRoot,
    relative: &Path,
    files: &[(PathBuf, Fingerprint)],
) -> ProviderAssessment {
    let result = assess_inner(root, relative, files);
    match result {
        Ok((eligible, reason, witnesses)) => ProviderAssessment {
            eligible,
            reason,
            witnesses,
        },
        Err(reason) => ProviderAssessment {
            eligible: false,
            reason,
            witnesses: vec![],
        },
    }
}
fn directory_names(root: &ApprovedRoot, relative: &Path) -> Result<Vec<String>, String> {
    let fd = root
        .open_relative(
            relative,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOATIME,
        )
        .map_err(|_| "Diretório relevante inacessível ou symlink")?;
    let mut buf = [MaybeUninit::uninit(); 8192];
    let mut entries = RawDir::new(&fd, &mut buf);
    let mut names = vec![];
    while let Some(entry) = entries.next() {
        let entry = entry.map_err(|_| "Enumeração incompleta")?;
        let bytes = entry.file_name().to_bytes();
        if bytes == b"." || bytes == b".." {
            continue;
        }
        names.push(
            std::str::from_utf8(bytes)
                .map_err(|_| "Nome ambíguo")?
                .to_owned(),
        );
        if names.len() > 4096 {
            return Err("Enumeração acima do limite".into());
        }
    }
    names.sort();
    Ok(names)
}
type ParsedLock = (BTreeMap<String, LockRecord>, Fingerprint);
fn read_lock(root: &ApprovedRoot, relative: &Path) -> Result<Option<ParsedLock>, String> {
    let fd = match root.open_relative(
        relative,
        OFlags::RDONLY | OFlags::NOATIME | OFlags::NONBLOCK,
    ) {
        Ok(fd) => fd,
        Err(rustix::io::Errno::NOENT) => return Ok(None),
        Err(rustix::io::Errno::LOOP) => {
            return Err("symlink em lock relevante; todas as versões protegidas".into())
        }
        Err(_) => return Err("Lock relevante inacessível".into()),
    };
    let st = fs::fstat(&fd).map_err(|_| "Metadados do lock indisponíveis")?;
    if FileType::from_raw_mode(st.st_mode) != FileType::RegularFile
        || st.st_uid != rustix::process::getuid().as_raw()
        || st.st_nlink != 1
        || st.st_mode & 0o022 != 0
        || st.st_size < 0
        || st.st_size as u64 > MAX_LOCK_BYTES
    {
        return Err("Lock relevante inseguro ou acima do limite".into());
    }
    let before = Fingerprint::from_stat(&st);
    let mut file = std::fs::File::from(fd);
    let mut bytes = vec![];
    (&mut file)
        .take(MAX_LOCK_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Lock ilegível")?;
    if bytes.len() as u64 != before.size
        || Fingerprint::from_stat(&fs::fstat(&file).map_err(|_| "Lock indisponível")?) != before
    {
        return Err("Lock alterado durante leitura".into());
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| "Lock não é UTF-8")?;
    Ok(Some((parse_lockfile(text)?, before)))
}
type AssessmentResult = (bool, String, Vec<(PathBuf, Fingerprint)>);
fn assess_inner(
    root: &ApprovedRoot,
    relative: &Path,
    files: &[(PathBuf, Fingerprint)],
) -> Result<AssessmentResult, String> {
    root.revalidate()?;
    let pkg = parse_package_path(relative).ok_or("Layout de provider desconhecido")?;
    crate::scan::cleanup_ancestors_allowed(root, relative)?;
    let names = directory_names(root, relative)?;
    let prefix = format!("terraform-provider-{}_v{}", pkg.provider_type, pkg.version);
    if names.len() != files.len()
        || names.len() > MAX_PACKAGE_FILES
        || names
            .iter()
            .filter(|n| valid_binary_name(n, &prefix))
            .count()
            != 1
        || names
            .iter()
            .filter(|n| n.as_str() == "LICENSE" || n.as_str() == "LICENSE.txt")
            .count()
            != 1
    {
        return Err("Layout de pacote incompleto ou desconhecido".into());
    }
    let mut total = 0u64;
    for name in &names {
        if !valid_binary_name(name, &prefix) && name != "LICENSE" && name != "LICENSE.txt" {
            return Err("Arquivo desconhecido no pacote".into());
        }
        let rel = relative.join(name);
        let fd = root
            .open_relative(&rel, OFlags::PATH)
            .map_err(|_| "symlink ou arquivo inacessível")?;
        let st = fs::fstat(&fd).map_err(|_| "Metadados indisponíveis")?;
        let fp = Fingerprint::from_stat(&st);
        if fp.size > MAX_FILE_BYTES {
            return Err("Provider acima do limite de 2 GiB por arquivo".into());
        }
        total = total
            .checked_add(fp.size)
            .filter(|n| *n <= MAX_PACKAGE_BYTES)
            .ok_or("Pacote acima do limite de 4 GiB")?;
        if FileType::from_raw_mode(st.st_mode) != FileType::RegularFile
            || st.st_nlink != 1
            || st.st_uid != rustix::process::getuid().as_raw()
            || st.st_mode & 0o022 != 0
            || !files.iter().any(|(p, f)| p == &rel && f == &fp)
        {
            return Err("Arquivo alterado, hardlink, permissões ou dono inseguros".into());
        }
    }
    assess_locks(root, &pkg)
}
fn assess_locks(root: &ApprovedRoot, pkg: &PackagePath) -> Result<AssessmentResult, String> {
    let mut witnesses = vec![];
    let mut locks = BTreeSet::new();
    let mut nearest = false;
    let mut current = pkg.project.clone();
    loop {
        let dir = if current.as_os_str().is_empty() {
            Path::new(".")
        } else {
            &current
        };
        let fd = root
            .open_relative(dir, OFlags::PATH | OFlags::DIRECTORY)
            .map_err(|_| "Ancestral inacessível ou symlink")?;
        witnesses.push((
            dir.to_path_buf(),
            Fingerprint::from_stat(&fs::fstat(fd).map_err(|_| "Ancestral indisponível")?),
        ));
        if !nearest {
            let rel = current.join(".terraform.lock.hcl");
            if let Some((map, fp)) = read_lock(root, &rel)? {
                locks.insert(rel.clone());
                witnesses.push((rel, fp));
                nearest = true;
                drop(map);
            }
        }
        let envs = current.join("environments");
        match root.open_relative(&envs, OFlags::PATH | OFlags::DIRECTORY) {
            Ok(fd) => {
                witnesses.push((
                    envs.clone(),
                    Fingerprint::from_stat(&fs::fstat(fd).map_err(|_| "Ambientes indisponíveis")?),
                ));
                for name in directory_names(root, &envs)? {
                    let path = envs.join(name);
                    let fd = root
                        .open_relative(&path, OFlags::PATH | OFlags::DIRECTORY)
                        .map_err(|_| "Ambiente relevante inacessível ou symlink")?;
                    witnesses.push((
                        path.clone(),
                        Fingerprint::from_stat(
                            &fs::fstat(fd).map_err(|_| "Ambiente indisponível")?,
                        ),
                    ));
                    let rel = path.join(".terraform.lock.hcl");
                    if let Some((_, fp)) = read_lock(root, &rel)? {
                        locks.insert(rel.clone());
                        witnesses.push((rel, fp));
                    }
                }
            }
            Err(rustix::io::Errno::NOENT) => {}
            Err(_) => return Err("Ambientes relevantes inacessíveis ou symlink".into()),
        }
        // .git can be a directory or a worktree marker. A symlink marker is a
        // boundary too, never a reason to traverse outside the approved root.
        let has_git = match root.open_relative(&current.join(".git"), OFlags::PATH) {
            Ok(_) | Err(rustix::io::Errno::LOOP) => true,
            Err(rustix::io::Errno::NOENT) => false,
            Err(_) => return Err("Limite Git desconhecido".into()),
        };
        if has_git || current.as_os_str().is_empty() || current == Path::new(".") {
            break;
        }
        current = current.parent().unwrap_or(Path::new("")).to_path_buf();
    }
    if locks.is_empty() {
        return Err("sem lock relevante dentro da raiz Git/análise".into());
    }
    let address = format!("{}/{}/{}", pkg.registry, pkg.namespace, pkg.provider_type);
    for rel in locks {
        let (map, _) = read_lock(root, &rel)?.ok_or("Lock desapareceu")?;
        if map.get(&address).is_some_and(|r| r.version == pkg.version) {
            return Ok((
                false,
                "Versão referenciada por lock relevante; uso não detectado".into(),
                witnesses,
            ));
        }
    }
    Ok((
        true,
        "Versão não referenciada pelos locks relevantes; hash somente ao preparar o plano".into(),
        witnesses,
    ))
}
pub fn provider_locks_unchanged(
    root: &ApprovedRoot,
    relative: &Path,
    expected: &[(PathBuf, Fingerprint)],
) -> bool {
    let Some(pkg) = parse_package_path(relative) else {
        return false;
    };
    let Ok((true, _, witnesses)) = assess_locks(root, &pkg) else {
        return false;
    };
    let locks: Vec<_> = witnesses
        .into_iter()
        .filter(|(p, _)| p.file_name().is_some_and(|n| n == ".terraform.lock.hcl"))
        .collect();
    locks == expected
}
/// Leitura sob demanda. Não autentica publicação: captura conteúdo/identidade
/// estável e exige novamente que nenhum lock relevante selecione esta versão.
pub fn verify_unused_provider(
    root: &ApprovedRoot,
    relative: &Path,
    files: &[(PathBuf, Fingerprint)],
    control: &ScanControl,
) -> Result<(), String> {
    let check = assess_provider(root, relative, files);
    if !check.eligible {
        return Err(check.reason);
    }
    for (rel, expected) in files {
        let fd = root
            .open_relative(rel, OFlags::RDONLY | OFlags::NOATIME | OFlags::NONBLOCK)
            .map_err(|_| "Provider inacessível")?;
        if Fingerprint::from_stat(&fs::fstat(&fd).map_err(|_| "Metadados indisponíveis")?)
            != *expected
        {
            return Err("Provider mudou".into());
        }
        let mut file = std::fs::File::from(fd);
        let mut digest = Sha256::new();
        let mut count = 0u64;
        let mut buf = [0u8; 65536];
        loop {
            if !control.checkpoint() {
                return Err("Análise cancelada".into());
            }
            let n = file.read(&mut buf).map_err(|_| "Provider ilegível")?;
            if n == 0 {
                break;
            }
            count += n as u64;
            if count > expected.size {
                return Err("Provider cresceu".into());
            }
            digest.update(&buf[..n]);
        }
        let _hash = digest.finalize();
        if count != expected.size
            || Fingerprint::from_stat(&fs::fstat(&file).map_err(|_| "Metadados indisponíveis")?)
                != *expected
        {
            return Err("Provider mudou durante hash".into());
        }
    }
    let after = assess_provider(root, relative, files);
    if !after.eligible || after.witnesses != check.witnesses {
        return Err("Locks mudaram durante hash".into());
    }
    Ok(())
}

fn valid_binary_name(name: &str, prefix: &str) -> bool {
    let Some(suffix) = name.strip_prefix(prefix) else {
        return false;
    };
    if suffix.is_empty() {
        return true;
    }
    let Some(n) = suffix.strip_prefix("_x") else {
        return false;
    };
    !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) && n != "0" && !n.starts_with('0')
}

fn elf_machine_matches(header: &[u8], platform: &str) -> bool {
    if header.len() < 20
        || &header[..4] != b"\x7fELF"
        || header[4] != 2
        || header[5] != 1
        || header[6] != 1
    {
        return false;
    }
    let machine = u16::from_le_bytes([header[18], header[19]]);
    match platform {
        "linux_amd64" => machine == 62,
        "linux_arm64" => machine == 183,
        _ => false,
    }
}

fn hash1(mut files: Vec<(String, [u8; 32])>) -> Option<String> {
    files.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let mut summary = Sha256::new();
    for (path, digest) in files {
        if path.contains('\n')
            || path.contains('\r')
            || path.starts_with('/')
            || path.split('/').any(|p| p == ".." || p.is_empty())
        {
            return None;
        }
        for byte in digest {
            summary.update(format!("{byte:02x}").as_bytes());
        }
        summary.update(b"  ");
        summary.update(path.as_bytes());
        summary.update(b"\n");
    }
    Some(format!("h1:{}", STANDARD.encode(summary.finalize())))
}

#[derive(Debug)]
struct LockRecord {
    version: String,
    hashes: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Ident(String),
    String(String),
    Eq,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
}

fn parse_lockfile(input: &str) -> Result<BTreeMap<String, LockRecord>, String> {
    let tokens = tokenize(input).map_err(|_| {
        "Terraform lock file syntax is outside the supported generated subset; preserve it."
            .to_string()
    })?;
    let mut parser = Parser { tokens, pos: 0 };
    let mut result = BTreeMap::new();
    while parser.pos < parser.tokens.len() {
        parser.expect_ident("provider")?;
        let address = parser.take_string()?;
        if !valid_lock_address(&address) {
            return Err(
                "Terraform lock file has a noncanonical provider address; preserve it.".into(),
            );
        }
        parser.expect(Token::LBrace)?;
        let mut version: Option<String> = None;
        let mut constraints_seen = false;
        let mut hashes: Option<BTreeSet<String>> = None;
        while !parser.next_is(&Token::RBrace) {
            if parser.pos >= parser.tokens.len() {
                return Err(
                    "Terraform lock file has an unterminated provider block; preserve it.".into(),
                );
            }
            let attr = parser.take_ident()?;
            parser.expect(Token::Eq)?;
            match attr.as_str() {
                "version" => {
                    if version.is_some() {
                        return Err("Terraform lock file duplicates a provider version attribute; preserve it.".into());
                    }
                    version = Some(parser.take_string()?);
                }
                "constraints" => {
                    if constraints_seen {
                        return Err(
                            "Terraform lock file duplicates a constraints attribute; preserve it."
                                .into(),
                        );
                    }
                    constraints_seen = true;
                    let _constraints = parser.take_string()?;
                }
                "hashes" => {
                    if hashes.is_some() {
                        return Err(
                            "Terraform lock file duplicates a hashes attribute; preserve it."
                                .into(),
                        );
                    }
                    parser.expect(Token::LBracket)?;
                    let mut values = BTreeSet::new();
                    while !parser.next_is(&Token::RBracket) {
                        let hash = parser.take_string()?;
                        if let Some(encoded) = hash.strip_prefix("h1:") {
                            let decoded = STANDARD.decode(encoded).map_err(|_| "Terraform lock file contains malformed h1 checksum; preserve it.".to_string())?;
                            if decoded.len() != 32 || STANDARD.encode(&decoded) != encoded {
                                return Err("Terraform lock file contains malformed h1 checksum; preserve it.".into());
                            }
                        }
                        if !values.insert(hash) {
                            return Err(
                                "Terraform lock file duplicates a provider checksum; preserve it."
                                    .into(),
                            );
                        }
                        if parser.next_is(&Token::Comma) {
                            parser.expect(Token::Comma)?;
                        } else if !parser.next_is(&Token::RBracket) {
                            return Err(
                                "Terraform lock file has malformed checksum list; preserve it."
                                    .into(),
                            );
                        }
                    }
                    parser.expect(Token::RBracket)?;
                    hashes = Some(values);
                }
                _ => {
                    return Err(
                        "Terraform lock file has an unknown provider attribute; preserve it."
                            .into(),
                    )
                }
            }
        }
        parser.expect(Token::RBrace)?;
        let version = version.ok_or_else(|| {
            "Terraform lock provider entry has no version; preserve it.".to_string()
        })?;
        if !valid_semver(&version) {
            return Err(
                "Terraform lock provider version is not canonical semver; preserve it.".into(),
            );
        }
        let hashes = hashes.ok_or_else(|| {
            "Terraform lock provider entry has no checksum list; preserve it.".to_string()
        })?;
        if !hashes.iter().any(|h| h.starts_with("h1:")) {
            return Err(
                "Terraform lock provider entry has no usable h1 checksum; preserve it.".into(),
            );
        }
        if result
            .insert(address, LockRecord { version, hashes })
            .is_some()
        {
            return Err("Terraform lock file duplicates a provider block; preserve it.".into());
        }
    }
    Ok(result)
}

fn valid_lock_address(address: &str) -> bool {
    let p: Vec<_> = address.split('/').collect();
    p.len() == 3 && valid_registry(p[0]) && valid_identifier(p[1]) && valid_identifier(p[2])
}

fn tokenize(input: &str) -> Result<Vec<Token>, ()> {
    let bytes = input.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            b'#' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                let mut closed = false;
                while i + 1 < bytes.len() {
                    if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                        i += 2;
                        closed = true;
                        break;
                    }
                    i += 1;
                }
                if !closed {
                    return Err(());
                }
            }
            b'=' => {
                out.push(Token::Eq);
                i += 1;
            }
            b'{' => {
                out.push(Token::LBrace);
                i += 1;
            }
            b'}' => {
                out.push(Token::RBrace);
                i += 1;
            }
            b'[' => {
                out.push(Token::LBracket);
                i += 1;
            }
            b']' => {
                out.push(Token::RBracket);
                i += 1;
            }
            b',' => {
                out.push(Token::Comma);
                i += 1;
            }
            b'"' => {
                i += 1;
                let mut value = String::new();
                let mut closed = false;
                while i < bytes.len() {
                    match bytes[i] {
                        b'"' => {
                            i += 1;
                            closed = true;
                            break;
                        }
                        b'\\' => {
                            i += 1;
                            let Some(&esc) = bytes.get(i) else {
                                return Err(());
                            };
                            match esc {
                                b'"' => value.push('"'),
                                b'\\' => value.push('\\'),
                                b'n' => value.push('\n'),
                                b'r' => value.push('\r'),
                                b't' => value.push('\t'),
                                _ => return Err(()),
                            }
                            i += 1;
                        }
                        b'$' if bytes.get(i + 1) == Some(&b'{') => return Err(()),
                        b if b < 0x20 => return Err(()),
                        _ => {
                            let tail = std::str::from_utf8(&bytes[i..]).map_err(|_| ())?;
                            let ch = tail.chars().next().ok_or(())?;
                            value.push(ch);
                            i += ch.len_utf8();
                        }
                    }
                }
                if !closed {
                    return Err(());
                }
                out.push(Token::String(value));
            }
            b if b.is_ascii_alphabetic() || b == b'_' => {
                let start = i;
                i += 1;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-')
                {
                    i += 1;
                }
                out.push(Token::Ident(
                    std::str::from_utf8(&bytes[start..i])
                        .map_err(|_| ())?
                        .to_string(),
                ));
            }
            _ => return Err(()),
        }
    }
    Ok(out)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}
impl Parser {
    fn next_is(&self, expected: &Token) -> bool {
        self.tokens.get(self.pos) == Some(expected)
    }
    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.tokens.get(self.pos) == Some(&expected) {
            self.pos += 1;
            Ok(())
        } else {
            Err("Terraform lock file syntax is outside the supported generated subset; preserve it.".into())
        }
    }
    fn take_string(&mut self) -> Result<String, String> {
        match self.tokens.get(self.pos).cloned() { Some(Token::String(s)) => { self.pos += 1; Ok(s) }, _ => Err("Terraform lock file syntax is outside the supported generated subset; preserve it.".into()) }
    }
    fn take_ident(&mut self) -> Result<String, String> {
        match self.tokens.get(self.pos).cloned() { Some(Token::Ident(s)) => { self.pos += 1; Ok(s) }, _ => Err("Terraform lock file syntax is outside the supported generated subset; preserve it.".into()) }
    }
    fn expect_ident(&mut self, want: &str) -> Result<(), String> {
        if self.tokens.get(self.pos) == Some(&Token::Ident(want.into())) {
            self.pos += 1;
            Ok(())
        } else {
            Err("Terraform lock file syntax is outside the supported generated subset; preserve it.".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_only_installed_linux_provider_package_paths() {
        assert!(is_provider_package_path(Path::new(
            "work/app/.terraform/providers/registry.terraform.io/hashicorp/aws/5.12.0/linux_amd64"
        )));
        assert!(is_provider_package_path(Path::new(
            ".terraform/providers/example.org/acme/thing/1.2.3-rc.1+build/linux_arm64"
        )));
        for path in [
            "work/.terraform/modules/network",
            "work/.terraform/providers/registry.terraform.io/hashicorp/aws/5.12.0/darwin_amd64",
            "work/.terraform/providers/registry.terraform.io/hashicorp/aws/05.12.0/linux_amd64",
            "work/.terraform/providers/registry.terraform.io/hashicorp/aws/5.12.0/linux_amd64/child",
            "work/.terraform/providers/registry.terraform.io/hashicorp/aws/5.12.0/../linux_amd64",
        ] { assert!(!is_provider_package_path(Path::new(path)), "{path}"); }
    }

    #[test]
    fn hashes_names_and_content_with_go_dirhash_h1_contract() {
        let got = hash1(vec![
            ("a".into(), Sha256::digest(b"one").into()),
            ("b".into(), Sha256::digest(b"two").into()),
        ])
        .unwrap();
        let mut summary = Sha256::new();
        summary.update(format!("{}  a\n", hex(&Sha256::digest(b"one"))).as_bytes());
        summary.update(format!("{}  b\n", hex(&Sha256::digest(b"two"))).as_bytes());
        assert_eq!(got, format!("h1:{}", STANDARD.encode(summary.finalize())));
        assert_ne!(
            got,
            hash1(vec![
                ("b".into(), Sha256::digest(b"two").into()),
                ("a".into(), Sha256::digest(b"changed").into())
            ])
            .unwrap()
        );
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn parses_generated_lock_subset_and_uses_only_h1_for_directories() {
        let lock = r#"
        # comment
        provider "registry.terraform.io/hashicorp/aws" {
          version = "5.12.0"
          constraints = "~> 5.0"
          hashes = [
            "zh:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "h1:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
          ]
        }
        "#;
        let parsed = parse_lockfile(lock).unwrap();
        let record = parsed.get("registry.terraform.io/hashicorp/aws").unwrap();
        assert_eq!(record.version, "5.12.0");
        assert!(record.hashes.iter().any(|h| h.starts_with("h1:")));
        assert!(parse_lockfile(&lock.replace(
            "version = \"5.12.0\"",
            "version = \"5.12.0\"\n  future = \"unknown\" "
        ))
        .is_err());
        assert!(parse_lockfile(&lock.replace(
            "h1:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
            "zh:aaaaaaaa"
        ))
        .is_err());
    }

    #[test]
    fn rejects_unknown_and_script_entries() {
        assert!(!valid_binary_name(
            "install.sh",
            "terraform-provider-aws_v5.12.0"
        ));
        assert!(!valid_binary_name(
            "terraform-provider-aws_v5.12.0_x05",
            "terraform-provider-aws_v5.12.0"
        ));
        assert!(valid_binary_name(
            "terraform-provider-aws_v5.12.0_x5",
            "terraform-provider-aws_v5.12.0"
        ));
    }

    #[test]
    fn verifies_only_locked_package_contents_and_returns_lock_witness() {
        use crate::scan::ScopeKind;
        use std::fs;

        let temp = tempfile::tempdir().unwrap();
        let package_rel = Path::new(
            "./work/.terraform/providers/registry.terraform.io/hashicorp/demo/1.2.3/linux_amd64",
        );
        let package_dir = temp.path().join(package_rel);
        fs::create_dir_all(&package_dir).unwrap();
        let executable = b"terraform-provider-demo_v1.2.3";
        let mut elf = vec![0u8; 20];
        elf[..4].copy_from_slice(b"\x7fELF");
        elf[4] = 2;
        elf[5] = 1;
        elf[6] = 1;
        elf[18..20].copy_from_slice(&62u16.to_le_bytes());
        elf.extend_from_slice(executable);
        fs::write(package_dir.join("terraform-provider-demo_v1.2.3"), &elf).unwrap();
        fs::write(package_dir.join("LICENSE.txt"), b"license text\n").unwrap();

        let expected_h1 = hash1(vec![
            (
                "LICENSE.txt".into(),
                Sha256::digest(b"license text\n").into(),
            ),
            (
                "terraform-provider-demo_v1.2.3".into(),
                Sha256::digest(&elf).into(),
            ),
        ])
        .unwrap();
        let project = temp.path().join("work");
        fs::write(
            project.join(".terraform.lock.hcl"),
            format!(
                "provider \"registry.terraform.io/hashicorp/demo\" {{\n  version = \"1.2.3\"\n  hashes = [\n    \"{expected_h1}\",\n  ]\n}}\n"
            ),
        )
        .unwrap();
        let root = ApprovedRoot::open(temp.path(), ScopeKind::Projects).unwrap();
        let files = ["LICENSE.txt", "terraform-provider-demo_v1.2.3"]
            .into_iter()
            .map(|name| {
                let rel = package_rel.join(name);
                let fd = root
                    .open_relative(&rel, OFlags::RDONLY | OFlags::NOATIME)
                    .unwrap();
                (
                    rel,
                    Fingerprint::from_stat(&rustix::fs::fstat(&fd).unwrap()),
                )
            })
            .collect::<Vec<_>>();
        let control = ScanControl::default();

        let verified = verify_provider(&root, package_rel, &files, &control);
        assert!(verified.eligible, "{}", verified.reason);
        assert_eq!(
            verified.lock_relative.as_deref(),
            Some(Path::new("work/.terraform.lock.hcl"))
        );
        assert!(verified.lock_fingerprint.is_some());

        fs::write(
            project.join(".terraform.lock.hcl"),
            "provider \"registry.terraform.io/hashicorp/demo\" { version = \"1.2.3\" hashes = [\"zh:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\"] }\n",
        )
        .unwrap();
        let zh_only = verify_provider(&root, package_rel, &files, &control);
        assert!(!zh_only.eligible);
        assert!(zh_only.reason.contains("h1"));
    }
}
