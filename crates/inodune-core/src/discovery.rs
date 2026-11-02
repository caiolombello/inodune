//! Automatic, read-only discovery of the current user's home and local mounted filesystems.
//! Mount records follow the documented `/proc/self/mountinfo` format. Discovery never reads
//! regular-file contents and never emits `/` as a cleanup capability.
use crate::scan::{ApprovedRoot, ScanControl, ScanIssue, ScanLimits, ScopeKind, VolumeInfo};
use rustix::{
    fd::{AsFd, OwnedFd},
    fs::{self as rfs, Mode, OFlags, RawDir, ResolveFlags},
    io::Errno,
    process,
};
use std::{
    collections::HashSet,
    fs,
    io::Read,
    mem::MaybeUninit,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct DiscoveryResult {
    pub roots: Vec<ApprovedRoot>,
    pub issues: Vec<ScanIssue>,
    pub volumes: Vec<VolumeInfo>,
    pub complete: bool,
}

/// Filesystem totals are observations, never cleanup capabilities.
fn observe_volume(path: &Path) -> Result<VolumeInfo, Errno> {
    let fd = rfs::openat2(
        rfs::CWD,
        path,
        OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )?;
    let st = rfs::fstat(&fd)?;
    let volume = rfs::fstatvfs(&fd)?;
    Ok(VolumeInfo {
        scope_path: path.to_string_lossy().into_owned(),
        device: st.st_dev,
        total_bytes: volume.f_blocks.saturating_mul(volume.f_frsize),
        available_bytes: volume.f_bavail.saturating_mul(volume.f_frsize),
    })
}

/// Preserve partial coverage independently from each candidate's complete manifest.
pub fn scan_discovered(
    discovered: DiscoveryResult,
    control: &ScanControl,
    limits: &ScanLimits,
    progress: impl FnMut(crate::scan::ScanProgress),
) -> Result<crate::scan::ScanSnapshot, String> {
    let mut snapshot = if discovered.roots.is_empty() {
        crate::scan::ScanSnapshot {
            id: crate::scan::scan_id(),
            candidates: vec![],
            issues: vec![],
            stats: Default::default(),
            complete: false,
            cancelled: control.is_cancelled(),
            volumes: vec![],
            targets: Default::default(),
        }
    } else {
        crate::scan::scan(&discovered.roots, control, limits, progress)?
    };
    snapshot.complete &= discovered.complete;
    snapshot.cancelled |= control.is_cancelled();
    for issue in &discovered.issues {
        *snapshot
            .stats
            .skip_reasons
            .entry(issue.code.clone())
            .or_default() += 1;
    }
    snapshot.stats.skipped = snapshot
        .stats
        .skipped
        .saturating_add(discovered.issues.len() as u64);
    snapshot.issues.extend(
        discovered
            .issues
            .into_iter()
            .take(limits.max_issues.saturating_sub(snapshot.issues.len())),
    );
    snapshot.volumes.extend(discovered.volumes);
    let mut seen = HashSet::new();
    snapshot.volumes.retain(|v| seen.insert(v.device));
    Ok(snapshot)
}

/// Discover the current user's home and eligible local filesystems visible in this mount namespace.
pub fn discover(control: &ScanControl, limits: &ScanLimits) -> Result<DiscoveryResult, String> {
    if process::getuid().is_root() {
        return Err("Inicie o Inodune com seu usuário comum, sem root.".into());
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .ok_or("HOME ausente ou inválida")?;
    let mut mountinfo = String::new();
    fs::File::open("/proc/self/mountinfo")
        .and_then(|f| f.take(8 * 1024 * 1024 + 1).read_to_string(&mut mountinfo))
        .map_err(|e| format!("Não foi possível ler mountinfo: {e}"))?;
    if mountinfo.len() > 8 * 1024 * 1024 {
        return Err("Mountinfo excede o limite de 8 MiB.".into());
    }
    // Enumerate the namespace's structural directories to locate owned trees,
    // without granting a cleanup capability for the filesystem root.
    let mut structural = ["/", "/run/media"]
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if let Some(cache) = crate::scan::xdg_thumbnail_root() {
        structural.push(cache);
    }
    discover_from(&home, &mountinfo, &structural, control, limits)
}

/// Injectable mount table and search paths make coverage behavior testable with temporary fixtures.
/// `search_roots` are trusted test roots; paths from frontend input never reach this API.
pub fn discover_from(
    home: &Path,
    mountinfo: &str,
    search_roots: &[PathBuf],
    control: &ScanControl,
    limits: &ScanLimits,
) -> Result<DiscoveryResult, String> {
    if !home.is_absolute() || home == Path::new("/") {
        return Err("HOME inválida".into());
    }
    if limits.max_issues == 0 {
        return Err("max_issues precisa ser maior que zero".into());
    }
    let mut result = DiscoveryResult {
        roots: Vec::new(),
        issues: Vec::new(),
        volumes: Vec::new(),
        complete: true,
    };
    let mut paths = Vec::<(PathBuf, ScopeKind)>::new();
    let home_mount = mountinfo
        .lines()
        .take(4096)
        .filter_map(parse_mount_line)
        .filter(|r| home.starts_with(&r.mountpoint))
        .max_by_key(|r| r.mountpoint.components().count());
    if home_mount.as_ref().is_none_or(|r| {
        is_local_fs(&r.fstype) || (r.mountpoint == Path::new("/") && r.fstype == "overlay")
    }) {
        paths.push((home.to_path_buf(), ScopeKind::Projects));
    } else {
        issue(
            &mut result,
            limits,
            home,
            "home_filesystem_protected",
            "Home em filesystem de rede ou não reconhecido; conteúdo não examinado.",
        );
    }
    // Structural roots are enumerated for names only, so owned directories below /opt, /srv,
    // removable media, etc. are found without attempting to scan protected system content.
    if mountinfo.lines().count() > 4096 {
        issue(
            &mut result,
            limits,
            Path::new("/proc/self/mountinfo"),
            "mount_limit",
            "Mais de 4096 mounts; cobertura limitada.",
        );
    }
    for line in mountinfo.lines().take(4096) {
        if !control.checkpoint() {
            result.complete = false;
            break;
        }
        let Some(record) = parse_mount_line(line) else {
            issue(
                &mut result,
                limits,
                Path::new("/proc/self/mountinfo"),
                "mountinfo_invalid",
                "Registro mountinfo inválido; cobertura parcial.",
            );
            continue;
        };
        if record.mountpoint == Path::new("/") {
            if is_local_fs(&record.fstype) || record.fstype == "overlay" {
                if let Ok(volume) = observe_volume(&record.mountpoint) {
                    result.volumes.push(volume);
                }
            } else {
                issue(
                    &mut result,
                    limits,
                    &record.mountpoint,
                    "volume_protected",
                    "Filesystem raiz de rede ou não reconhecido; conteúdo não examinado.",
                );
            }
            continue;
        }
        if !is_local_fs(&record.fstype) {
            // Known system and network mounts are intentionally outside cleanup discovery.
            if !is_virtual_fs(&record.fstype) || record.mountpoint.starts_with("/run/media") {
                issue(
                    &mut result,
                    limits,
                    &record.mountpoint,
                    "volume_protected",
                    "Filesystem de rede, virtual ou não reconhecido: protegido e não analisado.",
                );
            }
            continue;
        }
        if is_system_path(&record.mountpoint) && !record.mountpoint.starts_with("/run/media") {
            continue;
        }
        if let Ok(volume) = observe_volume(&record.mountpoint) {
            result.volumes.push(volume);
        }
        paths.extend(find_owned_roots(
            &record.mountpoint,
            control,
            limits,
            &mut result,
        ));
    }
    for p in search_roots.iter().take(128) {
        let enclosing = mountinfo
            .lines()
            .take(4096)
            .filter_map(parse_mount_line)
            .filter(|r| p.starts_with(&r.mountpoint))
            .max_by_key(|r| r.mountpoint.components().count());
        if enclosing.as_ref().is_some_and(|r| {
            !(is_local_fs(&r.fstype) || r.mountpoint == Path::new("/") && r.fstype == "overlay")
        }) {
            continue;
        }
        paths.extend(find_owned_roots(p, control, limits, &mut result));
    }

    let max_roots = 128usize.min(limits.max_entries.max(1) as usize);
    let mut seen_paths = HashSet::new();
    let mut seen_ids = HashSet::new();
    for (path, kind) in paths {
        let enclosing = mountinfo
            .lines()
            .take(4096)
            .filter_map(parse_mount_line)
            .filter(|r| path.starts_with(&r.mountpoint))
            .max_by_key(|r| r.mountpoint.components().count());
        if enclosing.as_ref().is_some_and(|r| {
            !(is_local_fs(&r.fstype) || r.mountpoint == Path::new("/") && r.fstype == "overlay")
        }) {
            continue;
        }
        if result.roots.len() >= max_roots {
            issue(
                &mut result,
                limits,
                &path,
                "root_limit",
                "Limite de raízes atingido; restante não analisado.",
            );
            break;
        }
        if !control.checkpoint() {
            result.complete = false;
            break;
        }
        if !seen_paths.insert(path.clone()) {
            continue;
        }
        match ApprovedRoot::open(&path, kind) {
            Ok(root) => {
                if !seen_ids.insert((root.dev, root.ino)) {
                    continue;
                }
                match root.volume() {
                    Ok(v) => result.volumes.push(v),
                    Err(e) => issue(&mut result, limits, &path, "volume_stat_failed", &e),
                }
                result.roots.push(root);
            }
            Err(e) => {
                if path == home {
                    issue(
                        &mut result,
                        limits,
                        &path,
                        "home_protected",
                        &format!("Home não pode ser aprovada: {e}"),
                    );
                } else {
                    issue(
                        &mut result,
                        limits,
                        &path,
                        "mount_protected",
                        &format!("Volume protegido ou inacessível: {e}"),
                    );
                }
            }
        }
    }
    // Nested mount roots are scanned independently; exclude their paths from a parent root so
    // scan() never treats two lexical overlaps as two authorities for the same subtree.
    let root_paths: Vec<PathBuf> = result
        .roots
        .iter()
        .map(|r| r.path().to_path_buf())
        .collect();
    let mut exclusion_failures = Vec::new();
    for root in &mut result.roots {
        let nested: Vec<PathBuf> = root_paths
            .iter()
            .filter_map(|p| p.strip_prefix(root.path()).ok())
            .filter(|rel| !rel.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .collect();
        if !nested.is_empty() {
            match root.with_exclusions(nested) {
                Ok(scoped) => *root = scoped,
                Err(e) => exclusion_failures.push((root.path().to_path_buf(), e)),
            }
        }
    }
    for (path, e) in exclusion_failures {
        issue(
            &mut result,
            limits,
            &path,
            "nested_mount_exclusion_failed",
            &e,
        );
    }
    if result.roots.is_empty() {
        result.complete = false;
    }
    let mut volumes = HashSet::new();
    result.volumes.retain(|v| volumes.insert(v.device));
    Ok(result)
}

fn find_owned_roots(
    seed: &Path,
    control: &ScanControl,
    limits: &ScanLimits,
    result: &mut DiscoveryResult,
) -> Vec<(PathBuf, ScopeKind)> {
    let mut found = Vec::new();
    let mut visited = 0u64;
    if !control.checkpoint() {
        result.complete = false;
        return found;
    }
    let seed_fd = match open_structural(rfs::CWD, seed, None) {
        Ok(fd) => fd,
        Err(Errno::NOENT) => return found,
        Err(e) => {
            issue(
                result,
                limits,
                seed,
                "structural_denied",
                &format!("Raiz estrutural não enumerável: {e}"),
            );
            return found;
        }
    };
    let Ok(st) = rfs::fstat(&seed_fd) else {
        return found;
    };
    if st.st_uid == process::getuid().as_raw() {
        found.push((seed.to_path_buf(), ScopeKind::Projects));
    } else if st.st_uid == 0 {
        walk_structural(
            seed_fd,
            seed,
            0,
            &mut visited,
            &mut found,
            control,
            limits,
            result,
        );
    }
    found
}

#[allow(clippy::too_many_arguments)] // traversal state is explicit and entirely task-local
fn walk_structural(
    fd: OwnedFd,
    path: &Path,
    depth: usize,
    visited: &mut u64,
    found: &mut Vec<(PathBuf, ScopeKind)>,
    control: &ScanControl,
    limits: &ScanLimits,
    result: &mut DiscoveryResult,
) {
    match rfs::fstat(&fd) {
        Ok(st) if st.st_uid == process::getuid().as_raw() => {
            found.push((path.to_path_buf(), ScopeKind::Projects));
            return;
        }
        Ok(st) if st.st_uid == 0 => {}
        _ => {
            issue(
                result,
                limits,
                path,
                "owner_protected",
                "Diretório de outro usuário ou identidade incerta; protegido.",
            );
            return;
        }
    }
    if depth >= limits.max_depth.min(5) || *visited >= limits.max_entries.min(20_000) {
        result.complete = false;
        return;
    }
    let mut buf = [MaybeUninit::uninit(); 8192];
    let entries: Vec<Vec<u8>> = {
        let mut dir = RawDir::new(&fd, &mut buf);
        let mut names = Vec::new();
        while let Some(entry) = dir.next() {
            if !control.checkpoint() {
                result.complete = false;
                return;
            }
            match entry {
                Ok(entry) => {
                    let n = entry.file_name().to_bytes();
                    if n != b"." && n != b".." {
                        names.push(n.to_vec());
                    }
                }
                Err(_) => {
                    issue(
                        result,
                        limits,
                        path,
                        "structural_entry_error",
                        "Falha ao enumerar nomes; cobertura parcial.",
                    );
                    return;
                }
            }
            *visited += 1;
            if *visited >= limits.max_entries.min(20_000) {
                result.complete = false;
                return;
            }
        }
        names
    };
    for name in entries {
        let child_name = std::ffi::OsStr::from_bytes(&name);
        let child_path = path.join(child_name);
        if is_system_path(&child_path) {
            issue(
                result,
                limits,
                &child_path,
                "system_protected",
                "Área do sistema protegida; conteúdo não examinado.",
            );
            continue;
        }
        let child_rel = Path::new(child_name);
        let resolve = ResolveFlags::BENEATH
            | ResolveFlags::NO_SYMLINKS
            | ResolveFlags::NO_MAGICLINKS
            | ResolveFlags::NO_XDEV;
        let meta_fd = match rfs::openat2(
            &fd,
            child_rel,
            OFlags::PATH | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
            resolve,
        ) {
            Ok(fd) => fd,
            Err(Errno::NOENT) => continue,
            Err(e) => {
                issue(
                    result,
                    limits,
                    &child_path,
                    "structural_unavailable",
                    &format!("Entrada não examinada: {e}"),
                );
                continue;
            }
        };
        let st = match rfs::fstat(&meta_fd) {
            Ok(st) => st,
            Err(e) => {
                issue(
                    result,
                    limits,
                    &child_path,
                    "structural_stat_failed",
                    &e.to_string(),
                );
                continue;
            }
        };
        if !rustix::fs::FileType::from_raw_mode(st.st_mode).is_dir() {
            continue;
        }
        if st.st_uid != process::getuid().as_raw() && st.st_uid != 0 {
            issue(
                result,
                limits,
                &child_path,
                "owner_protected",
                "Diretório de outro usuário; protegido e não examinado.",
            );
            continue;
        }
        if st.st_uid == process::getuid().as_raw() {
            found.push((child_path, ScopeKind::Projects));
        } else if st.st_uid == 0 {
            match open_structural(&fd, child_rel, Some(path)) {
                Ok(child) => walk_structural(
                    child,
                    &child_path,
                    depth + 1,
                    visited,
                    found,
                    control,
                    limits,
                    result,
                ),
                Err(e) => issue(
                    result,
                    limits,
                    &child_path,
                    "structural_denied",
                    &format!("Diretório protegido ou inacessível: {e}"),
                ),
            }
        }
    }
}

fn open_structural(
    base: impl AsFd,
    path: &Path,
    display_parent: Option<&Path>,
) -> Result<OwnedFd, Errno> {
    let flags = OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NOATIME;
    let resolve = if display_parent.is_some() {
        ResolveFlags::BENEATH
            | ResolveFlags::NO_SYMLINKS
            | ResolveFlags::NO_MAGICLINKS
            | ResolveFlags::NO_XDEV
    } else {
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS
    };
    match rfs::openat2(&base, path, flags | OFlags::RDONLY, Mode::empty(), resolve) {
        Ok(fd) => Ok(fd),
        Err(Errno::PERM) => {
            // Fall back only for root-owned structural directories, never user-owned roots.
            let meta = rfs::openat2(
                &base,
                path,
                OFlags::PATH | OFlags::CLOEXEC | OFlags::NOFOLLOW,
                Mode::empty(),
                resolve,
            )?;
            if rfs::fstat(&meta)?.st_uid != 0 {
                return Err(Errno::PERM);
            }
            rfs::openat2(
                &base,
                path,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
                Mode::empty(),
                resolve,
            )
        }
        Err(e) => Err(e),
    }
}

struct MountRecord {
    mountpoint: PathBuf,
    fstype: String,
}
fn parse_mount_line(line: &str) -> Option<MountRecord> {
    let (left, right) = line.split_once(" - ")?;
    let l: Vec<_> = left.split_ascii_whitespace().collect();
    let r: Vec<_> = right.split_ascii_whitespace().collect();
    if l.len() < 6 || r.len() < 3 {
        return None;
    }
    let mountpoint = decode_mount_path(l[4])?;
    if !mountpoint.is_absolute()
        || mountpoint
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }
    Some(MountRecord {
        mountpoint,
        fstype: r[0].to_ascii_lowercase(),
    })
}
fn decode_mount_path(s: &str) -> Option<PathBuf> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' {
            if i + 3 >= b.len() {
                return None;
            }
            let code = &b[i + 1..i + 4];
            out.push(match code {
                b"040" => b' ',
                b"011" => b'\t',
                b"012" => b'\n',
                b"134" => b'\\',
                _ => return None,
            });
            i += 4;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    Some(PathBuf::from(std::ffi::OsString::from_vec(out)))
}
fn is_local_fs(fs: &str) -> bool {
    [
        "ext2", "ext3", "ext4", "btrfs", "xfs", "f2fs", "vfat", "exfat", "ntfs", "ntfs3",
        "fuseblk", "zfs", "bcachefs",
    ]
    .contains(&fs)
}
fn is_virtual_fs(fs: &str) -> bool {
    [
        "proc",
        "sysfs",
        "devtmpfs",
        "devpts",
        "tmpfs",
        "ramfs",
        "securityfs",
        "cgroup",
        "cgroup2",
        "pstore",
        "tracefs",
        "debugfs",
        "configfs",
        "fusectl",
        "mqueue",
        "overlay",
        "autofs",
    ]
    .contains(&fs)
}
fn is_system_path(p: &Path) -> bool {
    [
        "/proc", "/sys", "/dev", "/etc", "/usr", "/boot", "/bin", "/sbin", "/lib", "/lib64",
        "/var", "/run",
    ]
    .iter()
    .any(|x| p.starts_with(x))
        && !p.starts_with("/run/media")
}
fn issue(r: &mut DiscoveryResult, limits: &ScanLimits, path: &Path, code: &str, message: &str) {
    r.complete = false;
    if r.issues.len() < limits.max_issues {
        r.issues.push(ScanIssue {
            display_path: path.to_string_lossy().into_owned(),
            code: code.into(),
            message: message.into(),
        });
    }
}
