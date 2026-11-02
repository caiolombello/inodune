//! Linux read-only discovery. All traversal is relative to a retained root descriptor.
//! No fallback to path-based walking if openat2 containment is unavailable.
use crate::{Candidate, Identity, Risk, SupportedAction};
use rustix::{
    fd::{AsFd, BorrowedFd, OwnedFd},
    fs::{self, FileType, Mode, OFlags, RawDir, ResolveFlags, Stat},
    io::Errno,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    io::Read,
    mem::MaybeUninit,
    os::unix::ffi::OsStrExt,
    path::{Component, Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Condvar, Mutex,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeKind {
    Projects,
    Thumbnails,
    Protected,
}
/// Known agent namespaces stay protected even when a descendant is approved as
/// a generic project root. The selected scope kind cannot downgrade this rule.
pub fn is_protected_agent_path(path: &Path) -> bool {
    path.components().any(|c|matches!(c,Component::Normal(name) if name==".codex"||name==".claude"||name=="opencode"))
}
pub fn cleanup_ancestors_allowed(root: &ApprovedRoot, relative: &Path) -> Result<(), String> {
    let groups = crate::access::SystemGroups;
    let mut current = Some(relative);
    while let Some(rel) = current {
        let fd = root
            .open_relative(rel, OFlags::PATH | OFlags::DIRECTORY)
            .map_err(|e| e.to_string())?;
        let st = fs::fstat(&fd).map_err(|e| e.to_string())?;
        if let Err(refusal) = crate::access::stat_write_allowed(
            crate::access::WriteRole::CleanupComponent,
            st.st_uid,
            st.st_gid,
            st.st_mode,
            &groups,
        ) {
            return Err(crate::access::describe_write_refusal_at(
                refusal,
                true,
                &root.path.join(rel),
            ));
        }
        current = rel.parent().filter(|p| !p.as_os_str().is_empty());
    }
    let st = fs::fstat(root.directory_fd()).map_err(|e| e.to_string())?;
    if let Err(refusal) = crate::access::stat_write_allowed(
        crate::access::WriteRole::CleanupRoot,
        st.st_uid,
        st.st_gid,
        st.st_mode,
        &groups,
    ) {
        return Err(crate::access::describe_write_refusal_at(
            refusal, true, &root.path,
        ));
    }
    Ok(())
}
fn xdg_cache_home() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(path) = CACHE_HOME_OVERRIDE.with(|slot| slot.borrow().clone()) {
        return Some(path);
    }
    std::env::var_os("XDG_CACHE_HOME")
        .filter(|p| !p.is_empty() && Path::new(p).is_absolute())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|p| Path::new(p).is_absolute())
                .map(|p| PathBuf::from(p).join(".cache"))
        })
}
fn under_uv_cache(path: &Path) -> bool {
    xdg_cache_home()
        .map(|cache| path.starts_with(cache.join("uv")))
        .unwrap_or(false)
}
fn directory_has_pyvenv_cfg(root: &ApprovedRoot, rel: &Path) -> bool {
    root.open_relative(&rel.join("pyvenv.cfg"), OFlags::PATH)
        .and_then(|fd| fs::fstat(&fd))
        .is_ok_and(|st| FileType::from_raw_mode(st.st_mode) == FileType::RegularFile)
}
#[cfg(test)]
thread_local! {
    static CACHE_HOME_OVERRIDE: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
    static TRASH_DIR_OVERRIDE: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}
/// `Not a directory (os error 20)` -> Portuguese, keeping unknown text as is.
pub fn os_error_pt(message: &str) -> String {
    let Some(start) = message.find("(os error ") else {
        return message.to_owned();
    };
    let code = message[start + 10..]
        .split(')')
        .next()
        .and_then(|n| n.parse::<i32>().ok());
    let text = match code {
        Some(1) => "operação não permitida",
        Some(2) => "arquivo ou pasta não existe mais",
        Some(4) => "operação interrompida",
        Some(5) => "erro de leitura no disco",
        Some(6) => "dispositivo ou endereço inexistente",
        Some(9) => "descritor de arquivo inválido",
        Some(11) => "recurso temporariamente indisponível",
        Some(12) => "memória insuficiente",
        Some(13) => "permissão negada",
        Some(16) => "recurso ocupado",
        Some(17) => "o destino já existe",
        Some(18) => "está em outro sistema de arquivos (ponto de montagem ou volume diferente)",
        Some(19) => "dispositivo não suportado",
        Some(20) => "um arquivo está onde se esperava uma pasta",
        Some(21) => "uma pasta está onde se esperava um arquivo",
        Some(22) => "argumento inválido",
        Some(23 | 24) => "arquivos abertos em excesso",
        Some(26) => "arquivo em uso",
        Some(27) => "arquivo grande demais",
        Some(28) => "sem espaço no dispositivo",
        Some(30) => "sistema de arquivos somente leitura",
        Some(31) => "links em excesso",
        Some(36) => "nome de arquivo longo demais",
        Some(39) => "a pasta não está vazia",
        Some(40) => "links simbólicos em excesso",
        Some(61) => "atributo inexistente",
        Some(75) => "valor grande demais para o tipo de dado",
        Some(95) => "operação não suportada",
        Some(107) => "montagem desconectada",
        Some(110) => "tempo de espera esgotado",
        Some(116) => "arquivo de rede desatualizado",
        Some(121) => "erro de E/S remota",
        Some(122) => "cota de disco excedida",
        _ => return message.to_owned(),
    };
    let prefix = message[..start].trim_end();
    let english = prefix.rsplit_once(": ").map_or(prefix, |(_, tail)| tail);
    if english.is_empty() || english == prefix {
        text.to_owned()
    } else {
        format!("{}: {text}", &prefix[..prefix.len() - english.len() - 2])
    }
}

/// "1 item protegido" / "N itens protegidos" (file or immediate directory).
fn preserved_label(count: usize) -> String {
    if count == 1 {
        "1 item protegido (arquivo ou diretório)".into()
    } else {
        format!("{count} itens protegidos (arquivos ou diretórios)")
    }
}
/// Display form of a path: drops `.` components (`/home/u/./x` -> `/home/u/x`).
/// Presentation only; `..` and symlinks are never resolved here.
pub fn tidy_display_path(path: &str) -> String {
    tidy_path(Path::new(path)).to_string_lossy().into_owned()
}
/// Same path without `.` components, byte-exact for non-UTF-8 names.
pub fn tidy_path(path: &Path) -> PathBuf {
    path.components()
        .filter(|c| !matches!(c, std::path::Component::CurDir))
        .collect()
}
pub fn xdg_thumbnail_root() -> Option<PathBuf> {
    xdg_cache_home().map(|p| p.join("thumbnails"))
}
#[derive(Clone)]
pub struct ApprovedRoot {
    pub(crate) fd: Arc<OwnedFd>,
    pub(crate) path: PathBuf,
    pub kind: ScopeKind,
    pub(crate) dev: u64,
    pub(crate) ino: u64,
    exclusions: Arc<Vec<PathBuf>>,
}
impl ApprovedRoot {
    pub fn open(path: &Path, kind: ScopeKind) -> Result<Self, String> {
        Self::open_with_exclusions(path, kind, Vec::new())
    }
    fn open_with_exclusions(
        path: &Path,
        kind: ScopeKind,
        exclusions: Vec<PathBuf>,
    ) -> Result<Self, String> {
        if path
            .to_str()
            .is_none_or(|s| s.chars().any(char::is_control))
        {
            return Err("Caminho da raiz contém bytes ou controles ambíguos; protegido.".into());
        }
        if kind == ScopeKind::Thumbnails && xdg_thumbnail_root().as_deref() != Some(path) {
            return Err("A pasta não corresponde a XDG_CACHE_HOME/thumbnails. Use inventário protegido para localizações não confirmadas.".into());
        }
        if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err("Escolha um caminho absoluto sem '..'.".into());
        }
        if path == Path::new("/")
            || [
                "/proc", "/sys", "/dev", "/etc", "/usr", "/boot", "/bin", "/sbin", "/lib",
                "/lib64", "/var",
            ]
            .iter()
            .any(|p| path.starts_with(p))
            || (path.starts_with("/run") && !path.starts_with("/run/media"))
        {
            return Err("Raiz de sistema não permitida.".into());
        }
        let fd = fs::openat2(
            fs::CWD,
            path,
            OFlags::RDONLY
                | OFlags::DIRECTORY
                | OFlags::CLOEXEC
                | OFlags::NOFOLLOW
                | OFlags::NOATIME,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|e| format!("Não foi possível aprovar a pasta (sem seguir links): {e}"))?;
        let st = fs::fstat(&fd).map_err(|e| e.to_string())?;
        if st.st_uid != rustix::process::getuid().as_raw() {
            return Err("A pasta não pertence ao usuário atual.".into());
        }
        let fstype = fs::fstatfs(&fd).map_err(|e| e.to_string())?.f_type;
        if [0x9fa0, 0x62656572, 0x63677270, 0x1cd1, 0x27e0eb].contains(&fstype) {
            return Err("Filesystem virtual não permitido.".into());
        }
        Ok(Self {
            fd: Arc::new(fd),
            path: path.to_path_buf(),
            kind,
            dev: st.st_dev,
            ino: st.st_ino,
            exclusions: Arc::new(exclusions),
        })
    }
    pub fn with_exclusions(&self, exclusions: Vec<PathBuf>) -> Result<Self, String> {
        if exclusions.len() > 128 || exclusions.iter().any(|p| p.as_os_str().len() > 4096) {
            return Err("Limite de 128 exclusões e 4.096 bytes por caminho.".into());
        }
        let mut normalized = Vec::with_capacity(exclusions.len());
        for path in exclusions {
            if path.as_os_str().is_empty()
                || path
                    .to_str()
                    .is_none_or(|s| s.chars().any(char::is_control))
                || path.is_absolute()
                || path.components().any(|c| {
                    matches!(
                        c,
                        Component::ParentDir | Component::RootDir | Component::Prefix(_)
                    )
                })
            {
                return Err("Exclusão inválida: use um caminho relativo sem '..' ou caracteres de controle.".into());
            }
            let mut clean = PathBuf::new();
            for component in path.components() {
                match component {
                    Component::CurDir => {}
                    Component::Normal(name) => clean.push(name),
                    _ => return Err("Exclusão inválida.".into()),
                }
            }
            if clean.as_os_str().is_empty() {
                clean.push(".");
            }
            if !normalized.contains(&clean) {
                normalized.push(clean);
            }
        }
        let mut root = self.clone();
        root.exclusions = Arc::new(normalized);
        Ok(root)
    }
    pub fn is_excluded(&self, path: &Path) -> bool {
        let Some(normalized) = normalize_relative(path) else {
            return false;
        };
        self.exclusions.iter().any(|excluded| {
            excluded == Path::new(".")
                || normalized == *excluded
                || normalized.starts_with(excluded)
        })
    }
    fn intersects_exclusion(&self, path: &Path) -> bool {
        let Some(normalized) = normalize_relative(path) else {
            return false;
        };
        self.exclusions.iter().any(|excluded| {
            excluded == Path::new(".")
                || normalized.starts_with(excluded)
                || excluded.starts_with(&normalized)
        })
    }
    pub fn has_exclusions(&self) -> bool {
        !self.exclusions.is_empty()
    }
    pub fn exclusions(&self) -> &[PathBuf] {
        &self.exclusions
    }
    pub fn revalidate(&self) -> Result<(), String> {
        let current = Self::open(&self.path, self.kind)?;
        if (current.dev, current.ino) != (self.dev, self.ino) {
            return Err("A pasta aprovada mudou; selecione-a novamente.".into());
        }
        Ok(())
    }
    pub fn display_path(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }
    /// Rust capability retained by the backend, never serialized to the renderer.
    pub fn directory_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn open_relative(&self, path: &Path, flags: OFlags) -> Result<OwnedFd, Errno> {
        if self.is_excluded(path) {
            return Err(Errno::NOENT);
        }
        fs::openat2(
            &*self.fd,
            path,
            flags | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
            ResolveFlags::BENEATH
                | ResolveFlags::NO_SYMLINKS
                | ResolveFlags::NO_MAGICLINKS
                | ResolveFlags::NO_XDEV,
        )
    }
    /// Stat the final component without following it. Ancestors still cannot be symlinks or cross a mount.
    pub fn lstat_relative(&self, path: &Path) -> Result<Stat, Errno> {
        if path.as_os_str().is_empty() || path == Path::new(".") {
            return fs::fstat(&*self.fd);
        }
        let name = path.file_name().ok_or(Errno::INVAL)?;
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let parent_fd = self.open_relative(parent, OFlags::RDONLY | OFlags::DIRECTORY)?;
        let fd = fs::openat2(
            &parent_fd,
            name,
            OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_MAGICLINKS | ResolveFlags::NO_XDEV,
        )?;
        fs::fstat(&fd)
    }
    pub fn volume(&self) -> Result<VolumeInfo, String> {
        let st = fs::fstatvfs(&*self.fd).map_err(|e| e.to_string())?;
        Ok(VolumeInfo {
            scope_path: self.display_path(),
            device: self.dev,
            total_bytes: st
                .f_blocks
                .checked_mul(st.f_frsize)
                .ok_or("Volume maior que o limite")?,
            available_bytes: st
                .f_bavail
                .checked_mul(st.f_frsize)
                .ok_or("Volume maior que o limite")?,
        })
    }
}
fn normalize_relative(path: &Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(name) => normalized.push(name),
            _ => return None,
        }
    }
    Some(normalized)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub scope_path: String,
    pub device: u64,
    pub total_bytes: u64,
    pub available_bytes: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanIssue {
    pub display_path: String,
    pub code: String,
    pub message: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ScanStats {
    pub visited: u64,
    pub skipped: u64,
    pub elapsed_ms: u64,
    /// Entries left out of a candidate. Internal symlinks of a deletable item are not counted.
    #[serde(default)]
    pub skip_reasons: std::collections::BTreeMap<String, u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanProgress {
    pub visited: u64,
    pub candidates: usize,
    pub stage: String,
    #[serde(default)]
    pub recent_findings: Vec<ScanFinding>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanFinding {
    pub id: String,
    pub title: String,
    pub category: String,
    pub risk: Risk,
    pub logical_bytes: u64,
}
#[derive(Clone, Debug)]
pub struct ScanLimits {
    pub max_entries: u64,
    pub max_depth: usize,
    pub max_candidates: usize,
    pub max_issues: usize,
}
impl Default for ScanLimits {
    fn default() -> Self {
        Self {
            max_entries: 1_000_000,
            max_depth: 64,
            max_candidates: 50_000,
            max_issues: 1000,
        }
    }
}
#[derive(Default)]
pub struct ScanControl {
    cancelled: AtomicBool,
    paused: Mutex<bool>,
    wake: Condvar,
}
impl ScanControl {
    pub fn cancel(&self) {
        // Synchronize with the condvar predicate to avoid a lost cancellation wakeup.
        let _guard = self.paused.lock().unwrap_or_else(|p| p.into_inner());
        self.cancelled.store(true, Ordering::Release);
        self.wake.notify_all();
    }
    pub fn pause(&self) {
        *self.paused.lock().unwrap_or_else(|p| p.into_inner()) = true;
    }
    pub fn resume(&self) {
        *self.paused.lock().unwrap_or_else(|p| p.into_inner()) = false;
        self.wake.notify_all();
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
    pub(crate) fn checkpoint(&self) -> bool {
        let mut paused = self.paused.lock().unwrap_or_else(|p| p.into_inner());
        while *paused && !self.is_cancelled() {
            paused = self.wake.wait(paused).unwrap_or_else(|p| p.into_inner());
        }
        !self.is_cancelled()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Fingerprint {
    pub dev: u64,
    pub ino: u64,
    pub size: u64,
    pub blocks: u64,
    pub mode: u32,
    pub nlink: u64,
    pub mtime: i64,
    pub mtime_nsec: u64,
    pub ctime: i64,
    pub ctime_nsec: u64,
    pub uid: u32,
}
impl Fingerprint {
    pub fn from_stat(st: &Stat) -> Self {
        Self {
            dev: st.st_dev,
            ino: st.st_ino,
            size: st.st_size.max(0) as u64,
            blocks: st.st_blocks.max(0) as u64,
            mode: st.st_mode,
            nlink: st.st_nlink,
            mtime: st.st_mtime,
            mtime_nsec: st.st_mtime_nsec,
            ctime: st.st_ctime,
            ctime_nsec: st.st_ctime_nsec,
            uid: st.st_uid,
        }
    }
}
struct Measurement {
    logical: u64,
    allocated: u64,
    complete: bool,
    files: Vec<(PathBuf, Fingerprint)>,
    blocks: Vec<String>,
    preserved: Vec<PathBuf>,
}
#[derive(Clone)]
pub struct ScannedTarget {
    pub root: ApprovedRoot,
    pub relative: PathBuf,
    pub fingerprint: Fingerprint,
    pub files: Vec<(PathBuf, Fingerprint)>,
    /// Non-deletable evidence (manifest/lock/source) bound to an eventual plan.
    pub witnesses: Vec<(PathBuf, Fingerprint)>,
    /// Intentional, non-deletable subtrees/files. Bound into the plan hash.
    pub preserved: Vec<PathBuf>,
    pub complete: bool,
}
#[derive(Clone)]
pub struct ScanSnapshot {
    pub id: String,
    pub candidates: Vec<Candidate>,
    pub issues: Vec<ScanIssue>,
    pub stats: ScanStats,
    pub complete: bool,
    pub cancelled: bool,
    pub volumes: Vec<VolumeInfo>,
    pub targets: HashMap<String, ScannedTarget>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ScanView<'a> {
    pub scan_id: &'a str,
    pub mode: &'static str,
    pub candidates: &'a [Candidate],
    pub issues: &'a [ScanIssue],
    pub stats: &'a ScanStats,
    pub complete: bool,
    pub cancelled: bool,
    pub volumes: &'a [VolumeInfo],
}
impl ScanSnapshot {
    pub fn view(&self) -> ScanView<'_> {
        ScanView {
            scan_id: &self.id,
            mode: "read_only",
            candidates: &self.candidates,
            issues: &self.issues,
            stats: &self.stats,
            complete: self.complete,
            cancelled: self.cancelled,
            volumes: &self.volumes,
        }
    }
}
static SEQUENCE: AtomicU64 = AtomicU64::new(1);
pub(crate) fn scan_id() -> String {
    format!(
        "scan-{:x}-{:x}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}
struct CandidatePolicy<'a> {
    detector: &'a str,
    title: &'a str,
    category: &'a str,
    risk: Risk,
    reason: &'a str,
}
struct Walker<'a, F: FnMut(ScanProgress)> {
    out: ScanSnapshot,
    control: &'a ScanControl,
    limits: &'a ScanLimits,
    start: Instant,
    last: Instant,
    on_progress: F,
    identities: HashSet<(u64, u64)>,
    git_paths: HashSet<PathBuf>,
    validation_bytes_left: u64,
}
impl<F: FnMut(ScanProgress)> Walker<'_, F> {
    fn progress_event(&self, stage: &str) -> ScanProgress {
        let first = self.out.candidates.len().saturating_sub(4);
        ScanProgress {
            visited: self.out.stats.visited,
            candidates: self.out.candidates.len(),
            stage: stage.into(),
            recent_findings: self.out.candidates[first..]
                .iter()
                .map(|candidate| ScanFinding {
                    id: candidate.id.clone(),
                    title: candidate.title.clone(),
                    category: candidate.category.clone(),
                    risk: candidate.risk.clone(),
                    logical_bytes: candidate.logical_bytes,
                })
                .collect(),
        }
    }
    fn count_skip(&mut self, code: &str) {
        self.out.stats.skipped += 1;
        *self
            .out
            .stats
            .skip_reasons
            .entry(code.to_owned())
            .or_default() += 1;
    }
    fn issue(&mut self, path: &Path, code: &str, message: &str) {
        self.out.complete = false;
        self.count_skip(code);
        if self.out.issues.len() < self.limits.max_issues {
            self.out.issues.push(ScanIssue {
                display_path: tidy_display_path(&path.to_string_lossy()),
                code: code.into(),
                message: os_error_pt(message),
            });
        }
    }
    fn progress(&mut self) {
        if self.last.elapsed().as_millis() >= 100 {
            let event = self.progress_event("Descobrindo itens conhecidos");
            (self.on_progress)(event);
            self.last = Instant::now();
        }
    }
    fn allowed(&mut self) -> bool {
        if self.out.stats.visited >= self.limits.max_entries
            || self.out.candidates.len() >= self.limits.max_candidates
        {
            self.out.complete = false;
            return false;
        }
        if !self.control.checkpoint() {
            self.out.cancelled = true;
            self.out.complete = false;
            return false;
        }
        true
    }
    fn names(&mut self, root: &ApprovedRoot, rel: &Path) -> Option<Vec<OsString>> {
        let fd = match root.open_relative(rel, OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOATIME)
        {
            Ok(fd) => fd,
            Err(e) => {
                self.issue(&root.path.join(rel), "inaccessible", &e.to_string());
                return None;
            }
        };
        let mut buf = [MaybeUninit::uninit(); 8192];
        let mut entries = RawDir::new(&fd, &mut buf);
        let mut names = Vec::new();
        while let Some(entry) = entries.next() {
            if !self.allowed() {
                return None;
            }
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    self.issue(&root.path.join(rel), "read_error", &e.to_string());
                    return None;
                }
            };
            let name = entry.file_name().to_bytes();
            if name == b"." || name == b".." {
                continue;
            }
            self.out.stats.visited += 1;
            self.progress();
            let child = rel.join(OsString::from(std::ffi::OsStr::from_bytes(name)));
            if root.is_excluded(&child) {
                self.count_skip("policy_excluded");
                if self.out.issues.len() < self.limits.max_issues {
                    self.out.issues.push(ScanIssue {
                        display_path: root.path.join(&child).to_string_lossy().into(),
                        code: "policy_excluded".into(),
                        message: "Caminho excluído por política do escopo; conteúdo não acessado."
                            .into(),
                    });
                }
                continue;
            }
            if names.len() >= 65_536 {
                self.issue(
                    &root.path.join(rel),
                    "directory_limit",
                    "Diretório excede o limite de memória; preservado.",
                );
                return None;
            }
            names.push(OsString::from(std::ffi::OsStr::from_bytes(name)));
        }
        names.sort();
        Some(names)
    }
    fn metadata(&mut self, root: &ApprovedRoot, rel: &Path) -> Option<Stat> {
        match root
            .open_relative(rel, OFlags::PATH)
            .and_then(|fd| fs::fstat(&fd))
        {
            Ok(st) if st.st_uid == rustix::process::getuid().as_raw() => Some(st),
            Ok(_) => {
                self.issue(
                    &root.path.join(rel),
                    "owner",
                    "Outro proprietário; preservado.",
                );
                None
            }
            Err(e) => {
                self.issue(
                    &root.path.join(rel),
                    if e == Errno::XDEV {
                        "mount"
                    } else if e == Errno::LOOP {
                        "symlink"
                    } else {
                        "inaccessible"
                    },
                    &e.to_string(),
                );
                None
            }
        }
    }
    fn measure(
        &mut self,
        root: &ApprovedRoot,
        rel: &Path,
        partial_credentials: bool,
    ) -> Measurement {
        let mut preserved: Vec<PathBuf> = Vec::new();
        let mut pending = vec![(rel.to_path_buf(), 0usize)];
        let mut logical = 0u64;
        let mut allocated = 0u64;
        let mut complete = true;
        let mut files = Vec::new();
        let mut seen = HashSet::new();
        let mut hardlinks: HashMap<(u64, u64), (u64, u64)> = HashMap::new();
        let mut blocks: Vec<String> = Vec::new();
        let note = |blocks: &mut Vec<String>, code: &str| {
            if !blocks.iter().any(|item| item == code) {
                blocks.push(code.to_owned());
            }
        };
        while let Some((path, depth)) = pending.pop() {
            if preserved.iter().any(|p| path.starts_with(p)) {
                continue;
            }
            if !self.allowed() {
                complete = false;
                break;
            }
            if depth > self.limits.max_depth {
                self.issue(
                    &root.path.join(&path),
                    "depth_limit",
                    "Limite de profundidade; cobertura incompleta.",
                );
                complete = false;
                continue;
            }
            let st = match root.lstat_relative(&path) {
                Ok(st) if st.st_uid == rustix::process::getuid().as_raw() => st,
                Ok(_) => {
                    self.issue(
                        &root.path.join(&path),
                        "owner",
                        "Outro proprietário; preservado.",
                    );
                    note(&mut blocks, "owner");
                    complete = false;
                    continue;
                }
                Err(e) => {
                    let code = if e == Errno::XDEV {
                        "mount"
                    } else if e == Errno::LOOP {
                        "symlink"
                    } else {
                        "inaccessible"
                    };
                    self.issue(&root.path.join(&path), code, &e.to_string());
                    note(&mut blocks, code);
                    complete = false;
                    continue;
                }
            };
            let credential = path.file_name().is_some_and(credential_name)
                || (FileType::from_raw_mode(st.st_mode) == FileType::RegularFile
                    && sensitive_content(root, &path));
            if partial_credentials && credential {
                let parent = path.parent().unwrap_or(rel);
                let keep = if FileType::from_raw_mode(st.st_mode) == FileType::Directory
                    || parent == rel
                {
                    path.clone()
                } else {
                    parent.to_owned()
                };
                if !preserved.iter().any(|p| keep.starts_with(p)) {
                    preserved.retain(|p| !p.starts_with(&keep));
                    preserved.push(keep);
                }
                self.count_skip("credential_preserved");
                continue;
            }
            if is_protected_agent_path(&root.path.join(&path))
                || path.file_name().is_some_and(|n| {
                    n == ".git" || n == ".ssh" || n == ".gnupg" || protected_name(n)
                })
            {
                note(&mut blocks, "credencial ou estado protegido");
                complete = false;
                continue;
            }
            if credential {
                note(
                    &mut blocks,
                    "assinatura de chave privada ou leitura insegura",
                );
                complete = false;
                continue;
            }
            match FileType::from_raw_mode(st.st_mode) {
                FileType::Directory => {
                    if cleanup_ancestors_allowed(root, &path).is_err() {
                        complete = false;
                    }
                    if let Some(names) = self.names(root, &path) {
                        for name in names {
                            pending.push((path.join(name), depth + 1));
                        }
                    } else {
                        complete = false;
                    }
                }
                FileType::RegularFile => {
                    if crate::access::stat_write_allowed(
                        crate::access::WriteRole::CacheLeaf,
                        st.st_uid,
                        st.st_gid,
                        st.st_mode,
                        &crate::access::SystemGroups,
                    )
                    .is_err()
                    {
                        complete = false;
                    }
                    if st.st_nlink > 1 {
                        let slot = hardlinks
                            .entry((st.st_dev, st.st_ino))
                            .or_insert((st.st_nlink, 0));
                        if slot.0 != st.st_nlink {
                            note(&mut blocks, "hardlink");
                            complete = false;
                        }
                        slot.1 += 1;
                    }
                    if seen.insert((st.st_dev, st.st_ino))
                        && self.identities.insert((st.st_dev, st.st_ino))
                    {
                        if let (Some(a), Some(b)) = (
                            logical.checked_add(st.st_size.max(0) as u64),
                            (st.st_blocks.max(0) as u64)
                                .checked_mul(512)
                                .and_then(|b| allocated.checked_add(b)),
                        ) {
                            logical = a;
                            allocated = b;
                        } else {
                            complete = false;
                        }
                    }
                    if files.len() >= 100_000 {
                        self.issue(
                            &root.path.join(&path),
                            "manifest_limit",
                            "Manifesto excede limite; item preservado.",
                        );
                        complete = false;
                        break;
                    }
                    files.push((path, Fingerprint::from_stat(&st)));
                }
                FileType::Symlink => {
                    // The link itself can be removed later. Never follow it and do not count it as skipped.
                    files.push((path, Fingerprint::from_stat(&st)));
                }
                _ => {
                    self.issue(
                        &root.path.join(&path),
                        "special",
                        "Arquivo especial preservado; conteúdo não seguido.",
                    );
                    note(&mut blocks, "special");
                    complete = false;
                }
            }
        }
        if !preserved.is_empty() {
            files.retain(|(path, _)| !preserved.iter().any(|keep| path.starts_with(keep)));
            logical = 0;
            allocated = 0;
            hardlinks.clear();
            let mut counted = HashSet::new();
            for (_, fp) in &files {
                if FileType::from_raw_mode(fp.mode) != FileType::RegularFile {
                    continue;
                }
                if counted.insert((fp.dev, fp.ino)) {
                    logical = logical.saturating_add(fp.size);
                    allocated = allocated.saturating_add(fp.blocks.saturating_mul(512));
                }
                if fp.nlink > 1 {
                    hardlinks.entry((fp.dev, fp.ino)).or_insert((fp.nlink, 0)).1 += 1;
                }
            }
        }
        for (nlink, seen_here) in hardlinks.values() {
            if *seen_here != *nlink {
                self.issue(
                    &root.path.join(rel),
                    "hardlink",
                    "Hardlink com elo fora deste item; preservado.",
                );
                note(&mut blocks, "hardlink");
                complete = false;
            }
        }
        Measurement {
            logical,
            allocated,
            complete,
            files,
            blocks,
            preserved,
        }
    }
    fn candidate(
        &mut self,
        root: &ApprovedRoot,
        rel: &Path,
        project: &str,
        policy: CandidatePolicy<'_>,
    ) {
        let CandidatePolicy {
            detector,
            title,
            category,
            risk,
            reason,
        } = policy;
        let Some(st) = self.metadata(root, rel) else {
            return;
        };
        let partial_credentials = matches!(
            detector,
            "npm.cache"
                | "python.pip-cache"
                | "rust.target"
                | "node.modules"
                | "next.cache"
                | "python.mypy-cache"
                | "python.ruff-cache"
                | "uv.cache"
        );
        let Measurement {
            logical,
            allocated,
            mut complete,
            files,
            blocks,
            preserved,
        } = self.measure(root, rel, partial_credentials);
        // Nothing left to clean (only empty folders and/or the preserved
        // credential subtree, e.g. after a cleanup): the item stays
        // non-cleanable exactly as before (complete = false below), but it is
        // reported as "nothing to clean" instead of "incomplete coverage"
        // when the walk itself found nothing unsafe.
        let measured_complete = complete;
        let nothing_to_clean =
            partial_credentials && files.is_empty() && blocks.is_empty() && measured_complete;
        if partial_credentials && files.is_empty() {
            complete = false;
        }
        for keep in &preserved {
            if !self.allowed() {
                complete = false;
                break;
            }
            self.candidate(root, keep, project, CandidatePolicy { detector: "protected.state", title: "Credencial / diretório preservado", category, risk: Risk::Protected, reason: "Credencial ou conteúdo não verificável: arquivo e diretório imediato ficam fora da limpeza." });
        }
        if root.intersects_exclusion(rel) {
            complete = false;
        }
        let coverage_complete = complete;
        // Ambiguous display names must never become a mutation authorization.
        let unambiguous = |path: &Path| {
            path.to_str()
                .is_some_and(|s| !s.chars().any(char::is_control))
        };
        complete &= unambiguous(&root.path.join(rel))
            && files.iter().all(|(p, _)| unambiguous(&root.path.join(p)));
        let py = detector == "python.pycache";
        let thumbs = detector == "thumbnails";
        if py {
            complete &= !files.is_empty()
                && files.iter().all(|(p, _)| {
                    p.parent() == Some(rel)
                        && matching_source(root, p)
                        && cache_header_valid(root, p, true)
                        && p.file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(valid_pyc_name)
                });
        }
        if thumbs {
            complete &= !files.is_empty()
                && files.iter().all(|(p, _)| {
                    p.parent() == Some(rel)
                        && p.file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(valid_thumbnail_name)
                        && cache_header_valid(root, p, false)
                });
        }
        let stable = root
            .open_relative(rel, OFlags::PATH)
            .and_then(|fd| fs::fstat(&fd))
            .is_ok_and(|after| Fingerprint::from_stat(&after) == Fingerprint::from_stat(&st));
        complete &= stable;
        let nothing_to_clean = nothing_to_clean && stable;
        // A candidate that is not a directory (.env, tfstate, a preserved
        // credential file) is never cleanable: same outcome as 0.1.10, where
        // opening it as a directory failed (complete = false, Risk::Protected,
        // no coverage_key). Only the text changed: no raw ENOTDIR message.
        let ancestor_refusal = if FileType::from_raw_mode(st.st_mode) == FileType::Directory {
            cleanup_ancestors_allowed(root, rel)
                .err()
                .map(|why| os_error_pt(&why))
        } else {
            Some("Arquivo avulso: fica fora da limpeza.".to_owned())
        };
        if ancestor_refusal.is_some() {
            complete = false;
        }
        let risk = if complete { risk } else { Risk::Protected };
        let uv_managed = reason.contains("uv cache clean");
        let rebuildable = matches!(
            detector,
            "python.pip-cache"
                | "node.modules"
                | "npm.cache"
                | "next.cache"
                | "python.mypy-cache"
                | "python.ruff-cache"
                | "rust.target"
        );
        let low_cache = complete && risk == Risk::Low && (py || thumbs);

        let mut witnesses = Vec::new();
        if detector == "rust.target" {
            let manifest = rel.parent().unwrap_or(Path::new(".")).join("Cargo.toml");
            if let Some(st) = self.metadata(root, &manifest) {
                witnesses.push((manifest, Fingerprint::from_stat(&st)));
            } else {
                complete = false;
            }
        }

        let id = format!("{}-{}", self.out.id, self.out.candidates.len());
        let reviewed_cache = complete && risk == Risk::Review && rebuildable;
        let nothing_to_clean = nothing_to_clean && ancestor_refusal.is_none();
        self.out.targets.insert(
            id.clone(),
            ScannedTarget {
                root: root.clone(),
                relative: rel.to_path_buf(),
                fingerprint: Fingerprint::from_stat(&st),
                files,
                witnesses,
                preserved: preserved.clone(),
                complete,
            },
        );
        self.out.candidates.push(Candidate{
            id, title:title.into(),display_path:root.path.join(rel).to_string_lossy().into(),detector_id:detector.into(),detector_version:"1".into(),scope_id:root.display_path(),category:category.into(),project:project.into(),
            selected_by_default:risk==Risk::Low,risk,logical_bytes:logical,allocated_bytes_estimate:Some(allocated),identity:Some(Identity{dev:st.st_dev,ino:st.st_ino,mtime_unix:st.st_mtime}),coverage_key:(ancestor_refusal.is_none() && coverage_complete && stable).then(||format!("{}:{}",st.st_dev,st.st_ino)),
            reason:if complete && !preserved.is_empty(){format!("{reason} {} fora da limpeza; somente o manifesto seguro vai para a Lixeira com revisão.",preserved_label(preserved.len()))}else if nothing_to_clean{if preserved.is_empty(){format!("{reason} Nada a limpar: restam só pastas vazias.")}else{format!("{reason} Nada a limpar: só resta {}, que fica no lugar.",preserved_label(preserved.len()))}}else if let Some(why)=ancestor_refusal{format!("{reason} {why}")}else if complete{reason.into()}else if blocks.is_empty(){format!("{reason} Cobertura incompleta: protegido.")}else{format!("{reason} Cobertura incompleta ({}): protegido.", blocks.join(", "))},
            impact:if partial_credentials {"Somente os arquivos revisados do manifesto vão para a Lixeira; exclusões protegidas permanecem no lugar. Exclusão permanente exige escolha explícita.".into()}else if py||thumbs {"O aplicativo recriará o cache; a primeira abertura pode ser mais lenta.".into()}else if reviewed_cache{"Pode exigir reconstrução ou novo download. Só vai para a Lixeira se você marcar este item e confirmar o plano.".into()}else{"Pode conter dados de trabalho ou exigir reconstrução/download. Inventário não autoriza remoção.".into()},
            rebuild_hint:if uv_managed{"use `uv cache clean`".into()}else if detector=="python.pip-cache" {"Equivalente oficial: pip cache purge (apenas informativo).".into()}else if detector=="npm.cache" {"Equivalente oficial: npm cache clean --force (apenas informativo).".into()}else if detector=="rust.target" {"Equivalente oficial: cargo clean (apenas informativo).".into()}else if rebuildable {"A ferramenta recria o cache; pode exigir download/recompilação.".into()}else if py {"Bytecode recriado ao importar o módulo.".into()}else if thumbs{"Miniaturas recriadas pelo aplicativo.".into()}else{"Custo não medido; revisão necessária.".into()},
            preserved:if partial_credentials{"Credenciais e diretórios excluídos permanecem no lugar; a raiz do cache nunca é removida inteira."}else{"Código, estado, configuração, credenciais, sessões e memória nunca entram em remoção genérica."}.into(),evidence:vec!["root-approved".into(),"openat2-no-symlinks-no-xdev".into(),if complete{"coverage-complete"}else if nothing_to_clean{"nothing-to-clean"}else{"coverage-incomplete"}.into()],activity:format!("mtime Unix {} (metadado; não comprova desuso)",st.st_mtime),supported_action:if low_cache{SupportedAction::CacheContents}else if reviewed_cache{SupportedAction::ReviewedCacheContents}else{SupportedAction::None},
        });
    }
    fn build_subsets(
        &mut self,
        root: &ApprovedRoot,
        container_id: &str,
        project: &str,
        providers: bool,
    ) {
        let Some(container) = self.out.targets.get(container_id).cloned() else {
            return;
        };
        if !container.complete {
            return;
        }
        let mut proposals = Vec::new();
        if providers {
            let mut groups: std::collections::BTreeMap<PathBuf, Vec<(PathBuf, Fingerprint)>> =
                std::collections::BTreeMap::new();
            for (p, fp) in &container.files {
                if let Some(parent) = p.parent() {
                    if crate::terraform::is_provider_package_path(parent) {
                        groups
                            .entry(parent.to_owned())
                            .or_default()
                            .push((p.clone(), fp.clone()));
                    }
                }
            }
            for (path, files) in groups {
                if !self.allowed() {
                    break;
                }
                let check = crate::terraform::assess_provider(root, &path, &files);
                let Some(st) = self.metadata(root, &path) else {
                    continue;
                };
                let witnesses = check
                    .witnesses
                    .into_iter()
                    .filter(|(p, _)| p.file_name().is_some_and(|n| n == ".terraform.lock.hcl"))
                    .collect();
                proposals.push((
                    path,
                    Fingerprint::from_stat(&st),
                    files,
                    witnesses,
                    "terraform.provider-package",
                    "Provider Terraform",
                    vec!["terraform-provider-metadata-only".into()],
                    if check.eligible {
                        Risk::Review
                    } else {
                        Risk::Protected
                    },
                    check.reason,
                ));
            }
        } else {
            for (path, fp) in &container.files {
                if !self.allowed() {
                    break;
                }
                if !crate::cache_rules::classify(root, path, &container.files).eligible {
                    continue;
                }
                let charge = fp.size.saturating_add(1024 * 1024 + 16);
                if charge > self.validation_bytes_left {
                    continue;
                }
                self.validation_bytes_left -= charge;
                if crate::cache_rules::validate_leaf(root, path).as_ref() != Some(fp) {
                    continue;
                }
                let Some(witnesses) = crate::cache_rules::witnesses(root, path) else {
                    continue;
                };
                proposals.push((
                    path.clone(),
                    fp.clone(),
                    vec![(path.clone(), fp.clone())],
                    witnesses,
                    "cargo.fingerprint-json",
                    "Metadado de fingerprint Cargo",
                    vec![
                        "cargo-rust-1.93.0-schema".into(),
                        "json-and-paired-hash-validated".into(),
                    ],
                    Risk::Review,
                    "Somente JSON diagnóstico Cargo reconhecido; revisão individual.".into(),
                ));
            }
        }
        for (path, fp, files, witnesses, detector, title, evidence, risk, reason) in proposals {
            if !self.allowed() {
                break;
            }
            let bytes = files.iter().map(|(_, f)| f.size).sum::<u64>();
            let allocated = files
                .iter()
                .map(|(_, f)| f.blocks.saturating_mul(512))
                .sum::<u64>();
            let id = format!("{}-{}", self.out.id, self.out.candidates.len());
            self.out.targets.insert(
                id.clone(),
                ScannedTarget {
                    root: root.clone(),
                    relative: path.clone(),
                    fingerprint: fp.clone(),
                    files,
                    witnesses,
                    preserved: Vec::new(),
                    complete: true,
                },
            );
            self.out.candidates.push(Candidate{id,title:title.into(),display_path:root.path.join(&path).to_string_lossy().into(),detector_id:detector.into(),detector_version:"f3-v1".into(),scope_id:root.display_path(),category:"Desenvolvimento".into(),project:project.into(),risk:risk.clone(),logical_bytes:bytes,allocated_bytes_estimate:Some(allocated),identity:Some(Identity{dev:fp.dev,ino:fp.ino,mtime_unix:fp.mtime}),coverage_key:Some(format!("{}:{}",fp.dev,fp.ino)),selected_by_default:false,reason,impact:if providers{"A próxima inicialização Terraform poderá precisar baixar o provider novamente. Não executamos terraform init."}else{"Cargo pode reconstruir artefatos ao perder o metadado de fingerprint; custo não medido."}.into(),rebuild_hint:if providers{"Reinstalação pelo Terraform, conforme lockfile preservado; sem estimativa de tempo/rede."}else{"Cargo regenera metadados; uma recompilação pode ser necessária."}.into(),preserved:"Pastas, manifests, lockfiles, scripts, estado, configuração e conteúdo desconhecido permanecem.".into(),evidence,activity:"Revisão por item; idade não comprova desuso.".into(),supported_action:if risk==Risk::Review{SupportedAction::ReviewedCacheContents}else{SupportedAction::None}});
            if !providers {
                if let Some(parent) = self.out.targets.get_mut(container_id) {
                    parent.files.retain(|(p, _)| p != &path);
                }
            }
            // Partition coverage: the container now describes only the remainder.
            if let Some(parent) = self
                .out
                .candidates
                .iter_mut()
                .find(|c| c.id == container_id)
            {
                parent.logical_bytes = parent.logical_bytes.saturating_sub(bytes);
                parent.allocated_bytes_estimate = parent
                    .allocated_bytes_estimate
                    .map(|n| n.saturating_sub(allocated));
                parent.risk = if providers {
                    Risk::Protected
                } else {
                    Risk::Review
                };
                parent.selected_by_default = false;
                parent.supported_action = if providers {
                    SupportedAction::None
                } else {
                    SupportedAction::ReviewedCacheContents
                };
                parent.coverage_key = parent.coverage_key.as_ref().map(|key| {
                    if key.starts_with("remainder:") {
                        key.clone()
                    } else {
                        format!("remainder:{key}")
                    }
                });
                parent.reason=if providers {"Conteúdo remanescente protegido; pacotes aparecem separadamente, sem dupla contagem."} else {"Artefatos Cargo recriáveis; metadados particionados aparecem separadamente."}.into();
            }
        }
    }
    fn build_pytest_nodeids_subset(
        &mut self,
        root: &ApprovedRoot,
        container_id: &str,
        project: &str,
    ) {
        let Some(container) = self.out.targets.get(container_id).cloned() else {
            return;
        };
        if !container.complete || !self.allowed() {
            return;
        }
        let path = container.relative.join("v/cache/nodeids");
        let matching: Vec<_> = container
            .files
            .iter()
            .filter(|(candidate_path, _)| candidate_path == &path)
            .collect();
        if matching.len() != 1 {
            return;
        }
        let fingerprint = matching[0].1.clone();
        let validation_cost = crate::cache_rules::pytest_nodeids_max_validation_bytes();
        if validation_cost > self.validation_bytes_left {
            return;
        }
        self.validation_bytes_left -= validation_cost;
        let event = self.progress_event("Validando índice de testes pytest");
        (self.on_progress)(event);
        let decision = crate::cache_rules::classify_pytest_nodeids(root, &path, &container.files);
        if !decision.eligible
            || crate::cache_rules::validate_pytest_nodeids_leaf(root, &path).as_ref()
                != Some(&fingerprint)
        {
            return;
        }
        let Some(witnesses) = crate::cache_rules::pytest_nodeids_witnesses(root, &path) else {
            return;
        };

        let id = format!("{}-{}", self.out.id, self.out.candidates.len());
        let allocated = fingerprint.blocks.saturating_mul(512);
        self.out.targets.insert(
            id.clone(),
            ScannedTarget {
                root: root.clone(),
                relative: path.clone(),
                fingerprint: fingerprint.clone(),
                files: vec![(path.clone(), fingerprint.clone())],
                witnesses,
                preserved: Vec::new(),
                complete: true,
            },
        );
        self.out.candidates.push(Candidate {
            id,
            title: "Índice de testes vistos (pytest --new-first)".into(),
            display_path: root.path.join(&path).to_string_lossy().into_owned(),
            detector_id: "python.pytest-nodeids".into(),
            detector_version: "pytest-9.1-nodeids-layout-v1".into(),
            scope_id: root.display_path(),
            category: "Desenvolvimento".into(),
            project: project.into(),
            risk: Risk::Review,
            logical_bytes: fingerprint.size,
            allocated_bytes_estimate: Some(allocated),
            identity: Some(Identity {
                dev: fingerprint.dev,
                ino: fingerprint.ino,
                mtime_unix: fingerprint.mtime,
            }),
            coverage_key: Some(format!("{}:{}", fingerprint.dev, fingerprint.ino)),
            selected_by_default: false,
            reason: decision.reason.into(),
            impact: decision.impact.into(),
            rebuild_hint: "O pytest recria o índice ao rodar a suíte; --new-first não separa arquivos vistos até então.".into(),
            preserved: "Código, histórico em v/cache/lastfailed, v/cache/stepwise e estado arbitrário de plugins permanecem protegidos.".into(),
            evidence: decision.evidence,
            activity: "Metadado mantido entre sessões; não comprova desuso.".into(),
            supported_action: SupportedAction::ReviewedCacheContents,
        });

        if let Some(parent) = self
            .out
            .candidates
            .iter_mut()
            .find(|c| c.id == container_id)
        {
            parent.logical_bytes = parent.logical_bytes.saturating_sub(fingerprint.size);
            parent.allocated_bytes_estimate = parent
                .allocated_bytes_estimate
                .map(|bytes| bytes.saturating_sub(allocated));
            parent.risk = Risk::Protected;
            parent.selected_by_default = false;
            parent.supported_action = SupportedAction::None;
            parent.coverage_key = parent.coverage_key.as_ref().map(|key| {
                if key.starts_with("remainder:") {
                    key.clone()
                } else {
                    format!("remainder:{key}")
                }
            });
            parent.reason = "Cache pytest protegido; índice --new-first verificado aparece à parte. Histórico de falhas, estado de plugins e demais dados continuam protegidos.".into();
        }
    }

    fn git_inventory(&mut self, root: &ApprovedRoot, rel: &Path, project: &str) {
        let path = root.path.join(rel);
        if self.git_paths.contains(&path) {
            return;
        }
        let event = self.progress_event("Verificando worktrees Git em sandbox somente leitura");
        (self.on_progress)(event);
        let inventory = crate::git::inspect(root, rel, self.control);
        let git_complete = inventory.complete;
        let records = if inventory.worktrees.is_empty() {
            vec![(path, None, inventory.reasons.clone())]
        } else {
            inventory.worktrees.into_iter().map(|tree|{let summary=format!("main={} dirty={} untracked={} locked={} detached={} prunable={} upstream={:?} ahead={:?} behind={:?}",tree.is_main,tree.dirty,tree.untracked,tree.locked,tree.detached,tree.prunable,tree.upstream,tree.ahead,tree.behind);(tree.path,Some(summary),tree.reasons)}).collect()
        };
        for (path, summary, reasons) in records {
            if !self.allowed() {
                break;
            }
            if !self.git_paths.insert(path.clone()) {
                continue;
            }
            let id = format!("{}-{}", self.out.id, self.out.candidates.len());
            let mut evidence = vec![
                if git_complete {
                    "git-porcelain-worktree-and-status-verified"
                } else {
                    "git-inspection-incomplete-protected"
                }
                .into(),
                "read-only-sandbox-required".into(),
                "agent-namespace-content-excluded".into(),
            ];
            if git_complete {
                if let Some(summary) = summary {
                    evidence.push(summary);
                }
            }
            self.out.candidates.push(Candidate{id,title:"Worktree / repositório Git".into(),display_path:path.to_string_lossy().into(),detector_id:"git.worktree".into(),detector_version:"f3-v1".into(),scope_id:root.display_path(),category:"Git Worktrees".into(),project:project.into(),risk:Risk::Protected,logical_bytes:0,allocated_bytes_estimate:None,identity:None,coverage_key:None,selected_by_default:false,reason:if reasons.is_empty(){"Atividade e publicação remota atuais não comprovadas. Worktree preservada.".into()}else{reasons.join(" ")},impact:"Remoção Git opcional permanece indisponível quando atividade/publicação são incertas.".into(),rebuild_hint:"Commits locais, conteúdo ignorado e sessões não têm garantia de recriação.".into(),preserved:"Worktree, arquivos ignorados, registro Git e metadados completos.".into(),evidence,activity:"Somente refs locais; não há fetch, inferência por idade nem teste sobre repositório do usuário.".into(),supported_action:SupportedAction::None});
        }
    }
    fn scan_trash(&mut self, root: &ApprovedRoot, rel: &Path) {
        let files_rel = rel.join("files");
        let info_rel = rel.join("info");
        let Some(names) = self.names(root, &files_rel) else {
            return;
        };
        let uid = rustix::process::getuid().as_raw();
        let files_dev = root.lstat_relative(&files_rel).ok().map(|st| st.st_dev);
        // Hardlinked regular files found in Trash/files: (dev, ino, nlink, path).
        let mut links: Vec<(u64, u64, u64, PathBuf)> = Vec::new();
        let mut walked_all = true;
        for name in names {
            if !self.allowed() {
                walked_all = false;
                break;
            }
            let entry = files_rel.join(&name);
            if self.trash_entry_must_stay(root, &entry) {
                self.push_protected_trash_entry(root, &entry);
                continue;
            }
            let mut deletable = Vec::new();
            let mut sensitive = Vec::new();
            // Original location from .trashinfo: name hint only (see
            // trashinfo_original). None keeps the stricter evaluation.
            let hint = trashinfo_original(
                root,
                &info_rel.join(format!("{}.trashinfo", name.to_string_lossy())),
            );
            if hint.as_deref().is_some_and(trash_keeps_entry) {
                self.push_protected_trash_entry(root, &entry);
                continue;
            }
            self.collect_trash(
                root,
                &entry,
                false,
                hint.as_deref(),
                &mut deletable,
                &mut sensitive,
                &mut links,
            );
            if !sensitive.is_empty() {
                for (path, fp) in &sensitive {
                    if !self.allowed() {
                        break;
                    }
                    let id = format!("{}-{}", self.out.id, self.out.candidates.len());
                    self.out.candidates.push(Candidate {
                        id,
                        title: "Item protegido na Lixeira".into(),
                        display_path: root.path.join(path).to_string_lossy().into_owned(),
                        detector_id: "protected.state".into(),
                        detector_version: "1".into(),
                        scope_id: root.display_path(),
                        category: "Lixeira".into(),
                        project: "Lixeira".into(),
                        risk: Risk::Protected,
                        logical_bytes: fp.size,
                        allocated_bytes_estimate: Some(fp.blocks.saturating_mul(512)),
                        identity: Some(Identity {
                            dev: fp.dev,
                            ino: fp.ino,
                            mtime_unix: fp.mtime,
                        }),
                        coverage_key: None,
                        selected_by_default: false,
                        reason: "Credencial, .env, tfstate ou tfvars na Lixeira; somente análise."
                            .into(),
                        impact: "Este arquivo permanece. O restante da entrada pode ser esvaziado."
                            .into(),
                        rebuild_hint: "Restaure manualmente se ainda precisar do arquivo.".into(),
                        preserved: "Credenciais, estado Terraform e arquivos de outro dono.".into(),
                        evidence: vec!["trash-sensitive".into()],
                        activity: "Metadado; não comprova desuso.".into(),
                        supported_action: SupportedAction::None,
                    });
                }
            }
            if deletable.is_empty() {
                continue;
            }
            if sensitive.is_empty() {
                let info = info_rel.join(format!("{}.trashinfo", name.to_string_lossy()));
                if let Ok(st) = root.lstat_relative(&info) {
                    if FileType::from_raw_mode(st.st_mode) == FileType::RegularFile
                        && st.st_uid == uid
                        && st.st_nlink == 1
                    {
                        deletable.push((info, Fingerprint::from_stat(&st)));
                    }
                }
            }
            let Ok(st) = root.lstat_relative(&entry) else {
                continue;
            };
            if st.st_uid != uid || FileType::from_raw_mode(st.st_mode) != FileType::Directory {
                continue;
            }
            let id = format!("{}-{}", self.out.id, self.out.candidates.len());
            let logical = deletable.iter().map(|(_, fp)| fp.size).sum();
            let allocated = deletable
                .iter()
                .map(|(_, fp)| fp.blocks.saturating_mul(512))
                .sum();
            self.out.targets.insert(
                id.clone(),
                ScannedTarget {
                    root: root.clone(),
                    relative: entry.clone(),
                    fingerprint: Fingerprint::from_stat(&st),
                    files: deletable,
                    witnesses: Vec::new(),
                    preserved: Vec::new(),
                    complete: true,
                },
            );
            self.out.candidates.push(Candidate {
                id,
                title: "Lixeira".into(),
                display_path: root.path.join(&entry).to_string_lossy().into_owned(),
                detector_id: "trash.entry".into(),
                detector_version: "1".into(),
                scope_id: root.display_path(),
                category: "Lixeira".into(),
                project: "Lixeira".into(),
                risk: Risk::Low,
                logical_bytes: logical,
                allocated_bytes_estimate: Some(allocated),
                identity: Some(Identity { dev: st.st_dev, ino: st.st_ino, mtime_unix: st.st_mtime }),
                coverage_key: Some(format!("{}:{}", st.st_dev, st.st_ino)),
                selected_by_default: false,
                reason: "Entrada da Lixeira do próprio usuário. Esvaziar apaga direto; credenciais e tfstate ficam de fora.".into(),
                impact: "A exclusão da Lixeira é permanente e libera espaço.".into(),
                rebuild_hint: "Não há reconstrução. Confira o resumo antes de confirmar.".into(),
                preserved: "Credenciais, .env, tfstate, tfvars e itens de outro dono.".into(),
                evidence: vec!["trash-owner-nofollow".into()],
                activity: "Metadado; não comprova desuso.".into(),
                supported_action: SupportedAction::CacheContents,
            });
        }
        let walked_all = walked_all && self.allowed();
        self.settle_trash_links(root, files_dev, links, walked_all);
    }
    fn trash_entry_must_stay(&mut self, root: &ApprovedRoot, rel: &Path) -> bool {
        if !self.allowed() || trash_keeps_entry(rel) {
            return true;
        }
        let uid = rustix::process::getuid().as_raw();
        let st = match root.lstat_relative(rel) {
            Ok(st) if st.st_uid == uid => st,
            Ok(_) => return false,
            Err(_) => return true,
        };
        if FileType::from_raw_mode(st.st_mode) != FileType::Directory {
            return false;
        }
        let Some(names) = self.names(root, rel) else {
            return true;
        };
        names
            .iter()
            .any(|name| self.trash_entry_must_stay(root, &rel.join(name)))
    }
    fn push_protected_trash_entry(&mut self, root: &ApprovedRoot, entry: &Path) {
        let Ok(st) = root.lstat_relative(entry) else {
            return;
        };
        if st.st_uid != rustix::process::getuid().as_raw() {
            return;
        }
        let fp = Fingerprint::from_stat(&st);
        let id = format!("{}-{}", self.out.id, self.out.candidates.len());
        self.out.candidates.push(Candidate {
            id,
            title: "Item protegido na Lixeira".into(),
            display_path: root.path.join(entry).to_string_lossy().into_owned(),
            detector_id: "protected.state".into(),
            detector_version: "1".into(),
            scope_id: root.display_path(),
            category: "Lixeira".into(),
            project: "Lixeira".into(),
            risk: Risk::Protected,
            logical_bytes: fp.size,
            allocated_bytes_estimate: Some(fp.blocks.saturating_mul(512)),
            identity: Some(Identity { dev: fp.dev, ino: fp.ino, mtime_unix: fp.mtime }),
            coverage_key: None,
            selected_by_default: false,
            reason: "Entrada da Lixeira com sessão de agente, repositório Git ou credencial; somente análise.".into(),
            impact: "A entrada inteira permanece, inclusive o .trashinfo correspondente.".into(),
            rebuild_hint: "Restaure manualmente se ainda precisar do conteúdo.".into(),
            preserved: "Sessões, repositórios Git, credenciais e o .trashinfo.".into(),
            evidence: vec!["trash-whole-entry".into()],
            activity: "Metadado; não comprova desuso.".into(),
            supported_action: SupportedAction::None,
        });
    }
    /// Hardlinks whose links are all inside the user's Trash/files (e.g. a
    /// node-gyp build moved to Trash with both links) stay preserved without
    /// making the analysis incomplete. Any link elsewhere, a different device or
    /// an interrupted walk keeps the blocking `hardlink` issue.
    fn settle_trash_links(
        &mut self,
        root: &ApprovedRoot,
        files_dev: Option<u64>,
        links: Vec<(u64, u64, u64, PathBuf)>,
        walked_all: bool,
    ) {
        let mut seen: HashMap<(u64, u64), u64> = HashMap::new();
        for (dev, ino, _, _) in &links {
            *seen.entry((*dev, *ino)).or_insert(0) += 1;
        }
        for (dev, ino, nlink, path) in links {
            let internal = walked_all
                && files_dev == Some(dev)
                && seen.get(&(dev, ino)).copied() == Some(nlink);
            if internal {
                self.count_skip("trash_internal_hardlink");
            } else {
                self.issue(
                    &root.path.join(&path),
                    "hardlink",
                    "Hardlink com elo fora da Lixeira; preservado.",
                );
            }
        }
    }
    fn collect_trash(
        &mut self,
        root: &ApprovedRoot,
        rel: &Path,
        inherited: bool,
        hint: Option<&Path>,
        deletable: &mut Vec<(PathBuf, Fingerprint)>,
        sensitive: &mut Vec<(PathBuf, Fingerprint)>,
        links: &mut Vec<(u64, u64, u64, PathBuf)>,
    ) {
        let uid = rustix::process::getuid().as_raw();
        let st = match root.lstat_relative(rel) {
            Ok(st) if st.st_uid == uid => st,
            Ok(_) => {
                self.issue(
                    &root.path.join(rel),
                    "owner",
                    "Outro proprietário na Lixeira; preservado.",
                );
                return;
            }
            Err(e) => {
                let code = if e == Errno::XDEV {
                    "mount"
                } else {
                    "inaccessible"
                };
                self.issue(&root.path.join(rel), code, &e.to_string());
                return;
            }
        };
        // Name rules on the Trash name AND on the original name (union).
        let named =
            |name: &std::ffi::OsStr| protected_name(name) || name == ".ssh" || name == ".gnupg";
        let name_sensitive =
            rel.file_name().is_some_and(named) || hint.and_then(Path::file_name).is_some_and(named);
        let sensitive_here = inherited
            || name_sensitive
            || (FileType::from_raw_mode(st.st_mode) == FileType::RegularFile
                && sensitive_content_as(root, rel, hint.unwrap_or(rel)));
        match FileType::from_raw_mode(st.st_mode) {
            FileType::Directory => {
                let Some(names) = self.names(root, rel) else {
                    return;
                };
                for name in names {
                    if !self.allowed() {
                        return;
                    }
                    let child_hint = hint.map(|h| h.join(&name));
                    self.collect_trash(
                        root,
                        &rel.join(name),
                        sensitive_here,
                        child_hint.as_deref(),
                        deletable,
                        sensitive,
                        links,
                    );
                }
            }
            FileType::RegularFile | FileType::Symlink => {
                let fp = Fingerprint::from_stat(&st);
                if sensitive_here {
                    sensitive.push((rel.to_path_buf(), fp));
                } else if FileType::from_raw_mode(st.st_mode) == FileType::RegularFile
                    && st.st_nlink > 1
                {
                    // Decided once the whole Trash/files was walked.
                    if links.len() < 100_000 {
                        links.push((st.st_dev, st.st_ino, st.st_nlink, rel.to_path_buf()));
                    } else {
                        self.issue(
                            &root.path.join(rel),
                            "hardlink",
                            "Hardlinks em excesso na Lixeira; preservado.",
                        );
                    }
                } else if deletable.len() < 100_000 {
                    deletable.push((rel.to_path_buf(), fp));
                } else {
                    self.issue(
                        &root.path.join(rel),
                        "manifest_limit",
                        "Entrada da Lixeira excede o manifesto.",
                    );
                }
            }
            _ => {
                self.issue(
                    &root.path.join(rel),
                    "special",
                    "Arquivo especial na Lixeira; preservado.",
                );
            }
        }
    }
    fn walk_root(&mut self, root: &ApprovedRoot) {
        if root.is_excluded(Path::new(".")) {
            self.count_skip("policy_excluded");
            if self.out.issues.len() < self.limits.max_issues {
                self.out.issues.push(ScanIssue {
                    display_path: root.display_path(),
                    code: "policy_excluded".into(),
                    message: "Escopo excluído por política; conteúdo não acessado.".into(),
                });
            }
            return;
        }
        if root.kind == ScopeKind::Protected
            || is_protected_agent_path(root.path())
            || root
                .path()
                .components()
                .any(|c| protected_name(c.as_os_str()))
        {
            self.candidate(root,Path::new("."),&root.path.file_name().unwrap_or_default().to_string_lossy(),CandidatePolicy{detector:"ai.history",title:"Inventário protegido",category:"IA & Agentes",risk:Risk::Protected,reason:"Escopo de conteúdo não regenerável; apenas metadados e tamanhos são inventariados."});
            return;
        }
        let project = root
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let mut pending = vec![(PathBuf::from("."), 0usize, project)];
        while let Some((rel, depth, project)) = pending.pop() {
            if !self.allowed() {
                break;
            }
            if depth > self.limits.max_depth {
                self.issue(
                    &root.path.join(&rel),
                    "depth_limit",
                    "Limite de profundidade.",
                );
                self.out.complete = false;
                continue;
            }
            let Some(names) = self.names(root, &rel) else {
                continue;
            };
            let has = |name: &str| names.iter().any(|n| n == name);
            let is_project = has("Cargo.toml")
                || has("package.json")
                || has("pyproject.toml")
                || has(".git")
                || names
                    .iter()
                    .any(|n| Path::new(n).extension().is_some_and(|e| e == "tf"));
            let project = if is_project {
                root.path
                    .join(&rel)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            } else {
                project
            };
            for name in names.iter().rev() {
                if !self.allowed() {
                    break;
                }
                let path = rel.join(name);
                let st = match root.lstat_relative(&path) {
                    Ok(st) if st.st_uid == rustix::process::getuid().as_raw() => st,
                    _ => {
                        let _ = self.metadata(root, &path);
                        continue;
                    }
                };
                let kind = FileType::from_raw_mode(st.st_mode);
                if kind == FileType::Symlink {
                    self.count_skip("symlink");
                    if self.out.issues.len() < self.limits.max_issues {
                        self.out.issues.push(ScanIssue {
                            display_path: root.path.join(&path).to_string_lossy().into(),
                            code: "symlink".into(),
                            message: "Link preservado; alvo não acessado.".into(),
                        });
                    }
                    continue;
                }
                if name == ".git" {
                    self.git_inventory(root, &rel, &project);
                    continue;
                }
                if kind == FileType::RegularFile {
                    if protected_name(name) || sensitive_content(root, &path) {
                        self.candidate(
                            root,
                            &path,
                            &project,
                            CandidatePolicy {
                                detector: "protected.state",
                                title: "Estado / configuração protegida",
                                category: "Desenvolvimento",
                                risk: Risk::Protected,
                                reason: "Estado, locks, credenciais ou instruções não são cache.",
                            },
                        );
                    }
                    continue;
                }
                if kind != FileType::Directory {
                    self.issue(
                        &root.path.join(&path),
                        "special",
                        "Arquivo especial preservado.",
                    );
                    continue;
                }
                if protected_name(name) {
                    self.candidate(root, &path, &project, CandidatePolicy { detector: "protected.state", title: "Credenciais / estado protegido", category: "Desenvolvimento", risk: Risk::Protected, reason: "Diretório de credenciais ou estado protegido; nenhuma limpeza autorizada." });
                    continue;
                }
                let n = name.to_string_lossy();
                if is_user_trash(&root.path.join(&path)) {
                    self.scan_trash(root, &path);
                    continue;
                }
                if n == "node_modules" {
                    self.candidate(root,&path,&project,CandidatePolicy{detector:"node.modules",title:"Dependências Node",category:"Desenvolvimento",risk:Risk::Review,reason:"node_modules recriável pelo gerenciador do projeto. Fora da limpeza de 1 clique."});
                    if let Some(item) = self.out.candidates.last_mut() {
                        item.selected_by_default = false;
                    }
                    continue;
                }
                if [".venv", "venv", ".ssh", ".gnupg"].contains(&n.as_ref())
                    || directory_has_pyvenv_cfg(root, &path)
                {
                    continue;
                }
                if n.starts_with(".inodune-") {
                    self.candidate(root,&path,&project,CandidatePolicy{detector:"inodune.recovery",title:"Área de recuperação protegida",category:"Sistema",risk:Risk::Protected,reason:"Operação anterior pode ter preservado arquivos; recuperação manual, nunca limpeza genérica."});
                    continue;
                }
                if [".codex", ".claude", "opencode"].contains(&n.as_ref()) {
                    self.candidate(
                        root,
                        &path,
                        &project,
                        CandidatePolicy {
                            detector: "ai.history",
                            title: "Sessões e memória de agentes",
                            category: "IA & Agentes",
                            risk: Risk::Protected,
                            reason:
                                "Localização de ferramenta de agentes: todo conteúdo é protegido.",
                        },
                    );
                    continue;
                }
                if (n == "cache" && rel.file_name().is_some_and(|p| p == ".next"))
                    || n == "_cacache"
                    || (n == "pip"
                        && root.path.join(&path).parent()
                            == xdg_thumbnail_root().as_deref().and_then(Path::parent))
                {
                    let (detector, title) = if n == "cache" {
                        ("next.cache", "Cache Next.js")
                    } else if n == "_cacache" {
                        ("npm.cache", "Cache npm")
                    } else {
                        ("python.pip-cache", "Cache pip")
                    };
                    self.candidate(root,&path,&project,CandidatePolicy{detector,title,category:"Desenvolvimento",risk:Risk::Review,reason:"Cache recriável em localização reconhecida; exige revisão e confirmação. Uso não detectado."});
                    continue;
                }
                if n == "__pycache__" {
                    if under_uv_cache(&root.path.join(&path)) {
                        self.candidate(root,&path,&project,CandidatePolicy{detector:"python.pycache",title:"Bytecode Python do uv",category:"Desenvolvimento",risk:Risk::Protected,reason:"Cache gerenciado pelo uv. Não remova arquivo a arquivo; use `uv cache clean`."});
                        continue;
                    }
                    let source = names
                        .iter()
                        .any(|n| Path::new(n).extension().is_some_and(|e| e == "py"));
                    self.candidate(root,&path,&project,CandidatePolicy{detector:"python.pycache",title:"Bytecode Python",category:"Desenvolvimento",risk:if source{Risk::Low}else{Risk::Protected},reason:if source{"Cache __pycache__ junto de módulo Python; formato será revalidado antes de qualquer ação."}else{"Nome de cache sem módulo Python correspondente: inventário protegido."}});
                    continue;
                }
                if n == ".pytest_cache" {
                    self.candidate(root,&path,&project,CandidatePolicy{detector:"python.toolcache",title:"Cache pytest protegido",category:"Desenvolvimento",risk:Risk::Protected,reason:"Estado de plugins e histórico de testes são preservados; apenas o índice conhecido de --new-first pode ser revisado separadamente."});
                    if let Some(id) = self.out.candidates.last().map(|c| c.id.clone()) {
                        self.build_pytest_nodeids_subset(root, &id, &project);
                    }
                    continue;
                }
                if [".mypy_cache", ".ruff_cache"].contains(&n.as_ref()) {
                    self.candidate(root,&path,&project,CandidatePolicy{detector:if n==".mypy_cache"{"python.mypy-cache"}else{"python.ruff-cache"},title:"Cache de ferramenta Python",category:"Desenvolvimento",risk:Risk::Review,reason:"Cache recriável de ferramenta Python; exige revisão. Uso não detectado."});
                    continue;
                }
                if n == "target" && has("Cargo.toml") {
                    self.candidate(root,&path,&project,CandidatePolicy{detector:"rust.target",title:"Artefatos Cargo",category:"Desenvolvimento",risk:Risk::Review,reason:"target junto de Cargo.toml; pode conter binários finais e exige revisão."});
                    if let Some(id) = self.out.candidates.last().map(|c| c.id.clone()) {
                        self.build_subsets(root, &id, &project, false);
                    }
                    continue;
                }
                if ((root.kind == ScopeKind::Thumbnails && rel == Path::new("."))
                    || xdg_thumbnail_root().as_deref() == Some(root.path.join(&rel).as_path()))
                    && ["normal", "large", "x-large", "xx-large"].contains(&n.as_ref())
                {
                    self.candidate(root,&path,"Cache do usuário",CandidatePolicy{detector:"thumbnails",title:"Miniaturas XDG",category:"Sistema",risk:Risk::Low,reason:"Subpasta de miniaturas na localização XDG confirmada pelo backend."});
                    continue;
                }
                if rel.file_name().is_some_and(|n| n == ".terraform")
                    && ["providers", "modules"].contains(&n.as_ref())
                {
                    self.candidate(
                        root,
                        &path,
                        &project,
                        CandidatePolicy {
                            detector: if n == "providers" {
                                "terraform.providers"
                            } else {
                                "terraform.modules"
                            },
                            title: if n == "providers" {
                                "Providers Terraform"
                            } else {
                                "Módulos Terraform"
                            },
                            category: "Desenvolvimento",
                            risk: Risk::Protected,
                            reason:
                                "Inventário Terraform; uso, links e backend não autorizam remoção.",
                        },
                    );
                    if n == "providers" {
                        if let Some(id) = self.out.candidates.last().map(|c| c.id.clone()) {
                            self.build_subsets(root, &id, &project, true);
                        }
                    }
                    continue;
                }
                pending.push((path, depth + 1, project.clone()));
            }
        }
    }
}
/// Only minimal format signatures are read, never payloads/prompts or source content.
/// This confirms a documented cache shape, not semantic validity of arbitrary bytecode.
pub fn cache_header_valid(root: &ApprovedRoot, path: &Path, python: bool) -> bool {
    let Ok(fd) = root.open_relative(path, OFlags::RDONLY | OFlags::NOATIME | OFlags::NONBLOCK)
    else {
        return false;
    };
    let Ok(before) = fs::fstat(&fd) else {
        return false;
    };
    if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile || before.st_nlink != 1 {
        return false;
    }
    let mut file = std::fs::File::from(fd);
    let mut header = [0u8; 33];
    let length = if python { 17 } else { 33 };
    if file.read_exact(&mut header[..length]).is_err() {
        return false;
    }
    let Ok(after) = fs::fstat(&file) else {
        return false;
    };
    if Fingerprint::from_stat(&before) != Fingerprint::from_stat(&after) {
        return false;
    }
    if python {
        let tag = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.split_once(".cpython-"))
            .map(|(_, t)| t.split('.').next().unwrap_or(""));
        let magic = match tag {
            Some("311") => 3495,
            Some("312") => 3531,
            Some("313") => 3571,
            Some("314") => 3627,
            Some("315") => 3666,
            _ => return false,
        };
        u16::from_le_bytes([header[0], header[1]]) == magic
            && header[2..4] == *b"\r\n"
            && matches!(
                u32::from_le_bytes(header[4..8].try_into().unwrap()),
                0 | 1 | 3
            )
            && header[16] & 0x7f == b'c'
            && before.st_size >= 64
    } else {
        header[..8] == [137, 80, 78, 71, 13, 10, 26, 10]
            && header[8..12] == [0, 0, 0, 13]
            && &header[12..16] == b"IHDR"
    }
}

pub fn matching_source(root: &ApprovedRoot, path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let Some((stem, _)) = name.split_once(".cpython-") else {
        return false;
    };
    let Some(parent) = path.parent().and_then(Path::parent) else {
        return false;
    };
    root.open_relative(&parent.join(format!("{stem}.py")), OFlags::PATH)
        .and_then(|fd| fs::fstat(&fd))
        .is_ok_and(|st| {
            FileType::from_raw_mode(st.st_mode) == FileType::RegularFile
                && st.st_uid == rustix::process::getuid().as_raw()
        })
}

fn valid_pyc_name(name: &str) -> bool {
    let Some(name) = name.strip_suffix(".pyc") else {
        return false;
    };
    let Some((stem, tag)) = name.split_once(".cpython-") else {
        return false;
    };
    if stem.is_empty() {
        return false;
    }
    let mut parts = tag.split('.');
    let Some(version) = parts.next() else {
        return false;
    };
    if !["311", "312", "313", "314", "315"].contains(&version) {
        return false;
    }
    match parts.next() {
        None => true,
        Some("opt-1" | "opt-2") => parts.next().is_none(),
        _ => false,
    }
}
fn valid_thumbnail_name(name: &str) -> bool {
    name.strip_suffix(".png")
        .is_some_and(|h| h.len() == 32 && h.chars().all(|c| c.is_ascii_hexdigit()))
}
fn trash_keeps_entry(path: &Path) -> bool {
    let Some(name) = path.file_name() else {
        return false;
    };
    let n = name.to_string_lossy();
    if matches!(
        n.as_ref(),
        ".codex"
            | ".claude"
            | "opencode"
            | ".cursor"
            | ".aws"
            | ".kube"
            | ".git"
            | ".netrc"
            | ".npmrc"
            | ".pypirc"
            | ".git-credentials"
    ) {
        return true;
    }
    if n.starts_with("id_rsa") || n.starts_with("id_ed25519") {
        return true;
    }
    if n.ends_with(".key") || n.ends_with(".p12") || n.ends_with(".pfx") || n.ends_with(".kdbx") {
        return true;
    }
    n == "config.json"
        && path
            .parent()
            .and_then(|parent| parent.file_name())
            .is_some_and(|parent| parent == ".docker")
}
/// Exact sensitive names, not substrings of source filenames.
pub fn credential_name(name: &std::ffi::OsStr) -> bool {
    let n = name.to_string_lossy().to_ascii_lowercase();
    if n.ends_with(".pub") {
        return false;
    }
    for base in ["id_rsa", "id_ed25519", "id_ecdsa", "id_dsa", "id_xmss"] {
        if let Some(suffix) = n.strip_prefix(base) {
            if ["", "_sk", ".old", ".bak", ".backup", "_sk.old", "_sk.bak"].contains(&suffix) {
                return true;
            }
        }
    }
    [
        ".netrc",
        ".pgpass",
        ".git-credentials",
        ".npmrc",
        ".pypirc",
        ".ssh",
        ".gnupg",
        ".kube",
        ".docker",
        ".aws",
        ".azure",
        "azure",
        "gcloud",
        "keyrings",
        "token",
        "token.json",
        "auth.json",
        "credentials",
        "credentials.json",
        "access_tokens.db",
        "application_default_credentials.json",
    ]
    .contains(&n.as_str())
        || [".p12", ".pfx", ".kdbx"].iter().any(|ext| n.ends_with(ext))
        || (n.starts_with(".env")
            && ![".example", ".sample", ".template"]
                .iter()
                .any(|suffix| n.ends_with(suffix)))
}
pub fn protected_name(name: &std::ffi::OsStr) -> bool {
    let n = name.to_string_lossy().to_ascii_lowercase();
    credential_name(name)
        || n.contains(".tfstate")
        || n.ends_with(".tfvars")
        || n.ends_with(".tfplan")
        || n == ".terraform.lock.hcl"
        || n.ends_with(".ovpn")
        || n == ".cursor"
}
/// Content policy shared by the scanner and the executor (see `credentials`).
pub use crate::credentials::credential_bytes;
/// Bounded read through the retained root, without following symlinks: the
/// first 64 KiB of every file and up to 1 MiB of text files (no NUL in the first
/// window). Unreadable or changing files are fail-closed (protected).
pub fn sensitive_content(root: &ApprovedRoot, path: &Path) -> bool {
    sensitive_content_as(root, path, path)
}

/// Same bounded read of `path`, classified as if it were named `name_hint`.
/// The hint only feeds `credential_bytes` (extension/documentation rules); a
/// private-key header and real values protect whatever the hint says.
pub fn sensitive_content_as(root: &ApprovedRoot, path: &Path, name_hint: &Path) -> bool {
    let Ok(fd) = root.open_relative(path, OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOATIME)
    else {
        return true;
    };
    let Ok(before) = fs::fstat(&fd) else {
        return true;
    };
    if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile {
        return true;
    }
    let mut file = std::fs::File::from(fd);
    use crate::credentials::{FIRST_WINDOW, TEXT_LIMIT};
    let mut bytes = Vec::with_capacity(4096);
    if (&mut file)
        .take(FIRST_WINDOW as u64)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return true;
    }
    if bytes.len() == FIRST_WINDOW
        && !bytes.contains(&0)
        && (&mut file)
            .take((TEXT_LIMIT - FIRST_WINDOW) as u64)
            .read_to_end(&mut bytes)
            .is_err()
    {
        return true;
    }
    let Ok(after) = fs::fstat(&file) else {
        return true;
    };
    Fingerprint::from_stat(&before) != Fingerprint::from_stat(&after)
        || credential_bytes(name_hint, &bytes)
}

/// Original location of a Trash entry, from `info/<name>.trashinfo`.
/// Untrusted: it is only a name hint for the same content rules used in
/// caches, and name rules apply to it in addition to the Trash name. Any
/// doubt (missing, not private, too big, malformed) returns None, which keeps
/// the stricter evaluation by the random Trash name.
fn trashinfo_original(root: &ApprovedRoot, info_rel: &Path) -> Option<PathBuf> {
    let fd = root
        .open_relative(
            info_rel,
            OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOATIME,
        )
        .ok()?;
    let st = fs::fstat(&fd).ok()?;
    if FileType::from_raw_mode(st.st_mode) != FileType::RegularFile
        || st.st_uid != rustix::process::getuid().as_raw()
        || st.st_nlink != 1
        || !(0..=8192).contains(&st.st_size)
    {
        return None;
    }
    let mut text = String::new();
    std::fs::File::from(fd)
        .take(8193)
        .read_to_string(&mut text)
        .ok()?;
    let mut lines = text.lines();
    if lines.next()? != "[Trash Info]" {
        return None;
    }
    let mut found = None;
    for line in lines {
        if let Some(value) = line.strip_prefix("Path=") {
            if found.is_some() {
                return None;
            }
            found = Some(value);
        }
    }
    let encoded = found?.as_bytes();
    let mut decoded = Vec::with_capacity(encoded.len());
    let mut i = 0;
    while i < encoded.len() {
        if encoded[i] == b'%' {
            let hex = std::str::from_utf8(encoded.get(i + 1..i + 3)?).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            decoded.push(encoded[i]);
            i += 1;
        }
    }
    if decoded.is_empty()
        || decoded.len() > 4096
        || decoded[0] != b'/'
        || decoded.iter().any(|b| *b == 0 || b.is_ascii_control())
    {
        return None;
    }
    // Only plain components: "." and ".." (e.g. /x/node_modules/../README.md)
    // or empty ones from "//" or a trailing "/" could fake a package or
    // TypeScript location, so such a hint is rejected (stricter evaluation).
    if decoded[1..]
        .split(|b| *b == b'/')
        .any(|part| part.is_empty() || part == b"." || part == b"..")
    {
        return None;
    }
    use std::os::unix::ffi::OsStringExt;
    Some(PathBuf::from(std::ffi::OsString::from_vec(decoded)))
}

/// For a path inside the user's `Trash/files/<entry>/...`: the original path
/// of the same file according to the entry's `.trashinfo`.
fn trash_name_hint(root: &ApprovedRoot, relative: &Path) -> Option<PathBuf> {
    let mut ancestor = relative;
    let mut rest: Vec<&std::ffi::OsStr> = Vec::new();
    while let Some(parent) = ancestor.parent() {
        if parent.file_name().is_some_and(|n| n == "files")
            && parent
                .parent()
                .is_some_and(|trash| is_user_trash(&root.path.join(trash)))
        {
            let entry = ancestor.file_name()?;
            let info = parent
                .parent()?
                .join("info")
                .join(format!("{}.trashinfo", entry.to_string_lossy()));
            let mut hint = trashinfo_original(root, &info)?;
            for part in rest.iter().rev() {
                hint.push(part);
            }
            return Some(hint);
        }
        rest.push(ancestor.file_name()?);
        ancestor = parent;
    }
    None
}

/// Executor revalidation for any manifest file: content as in the scan, with
/// the Trash name hint when the file lives in the user's Trash, plus the name
/// rules on the hinted name (never fewer rules than the Trash name alone).
pub fn cleanup_content_sensitive(root: &ApprovedRoot, relative: &Path) -> bool {
    match trash_name_hint(root, relative) {
        Some(hint) => {
            hint.file_name().is_some_and(protected_name)
                || sensitive_content_as(root, relative, &hint)
        }
        None => sensitive_content(root, relative),
    }
}

pub fn user_trash_dir() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(path) = TRASH_DIR_OVERRIDE.with(|slot| slot.borrow().clone()) {
        return Some(path);
    }
    let data = std::env::var_os("XDG_DATA_HOME")
        .filter(|value| Path::new(value).is_absolute())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|value| Path::new(value).is_absolute())
                .map(|home| PathBuf::from(home).join(".local/share"))
        })?;
    Some(data.join("Trash"))
}
fn is_user_trash(path: &Path) -> bool {
    user_trash_dir().is_some_and(|trash| path == trash)
}
/// `files/<name>` pairs with `info/<name>.trashinfo` in the same trash directory.
pub fn paired_trashinfo(relative: &Path) -> Option<PathBuf> {
    let name = relative.file_name()?;
    let files = relative.parent()?;
    if files.file_name()? != "files" {
        return None;
    }
    let trash = files.parent()?;
    Some(
        trash
            .join("info")
            .join(format!("{}.trashinfo", name.to_string_lossy())),
    )
}

pub fn scan(
    roots: &[ApprovedRoot],
    control: &ScanControl,
    limits: &ScanLimits,
    on_progress: impl FnMut(ScanProgress),
) -> Result<ScanSnapshot, String> {
    scan_with_validation_budget(roots, control, limits, on_progress, 512 * 1024 * 1024)
}

fn scan_with_validation_budget(
    roots: &[ApprovedRoot],
    control: &ScanControl,
    limits: &ScanLimits,
    on_progress: impl FnMut(ScanProgress),
    validation_budget: u64,
) -> Result<ScanSnapshot, String> {
    if limits.max_entries == 0
        || limits.max_entries > 1_000_000
        || limits.max_candidates == 0
        || limits.max_candidates > 50_000
        || limits.max_depth > 128
        || limits.max_issues > 2000
    {
        return Err("Limites de scan fora do orçamento seguro.".into());
    }
    if roots.is_empty() {
        return Err("Nenhum escopo aprovado.".into());
    }
    let mut unique = HashSet::new();
    for root in roots {
        root.revalidate()?;
        if !unique.insert((root.dev, root.ino)) {
            return Err("Escopo duplicado.".into());
        }
    }
    for (i, a) in roots.iter().enumerate() {
        for b in roots.iter().skip(i + 1) {
            if (a.path.starts_with(&b.path)
                && !b.is_excluded(a.path.strip_prefix(&b.path).unwrap()))
                || (b.path.starts_with(&a.path)
                    && !a.is_excluded(b.path.strip_prefix(&a.path).unwrap()))
            {
                return Err("Escopos sobrepostos; escolha uma única root para cada árvore.".into());
            }
        }
    }
    let now = Instant::now();
    let mut walk = Walker {
        out: ScanSnapshot {
            id: scan_id(),
            candidates: Vec::new(),
            issues: Vec::new(),
            stats: ScanStats::default(),
            complete: true,
            cancelled: false,
            volumes: Vec::new(),
            targets: HashMap::new(),
        },
        control,
        limits,
        start: now,
        last: now,
        on_progress,
        identities: HashSet::new(),
        git_paths: HashSet::new(),
        validation_bytes_left: validation_budget,
    };
    for root in roots {
        match root.volume() {
            Ok(v) => walk.out.volumes.push(v),
            Err(e) => walk.issue(&root.path, "volume", &e),
        };
        walk.walk_root(root);
    }
    if walk.out.stats.visited >= limits.max_entries
        || walk.out.candidates.len() >= limits.max_candidates
    {
        walk.out.complete = false;
        walk.issue(
            &roots[0].path,
            "entry_limit",
            "Limite de entradas/candidatos alcançado.",
        );
    }
    walk.out.stats.elapsed_ms = walk.start.elapsed().as_millis().min(u64::MAX as u128) as u64;
    let final_stage = if walk.out.cancelled {
        "Cancelado"
    } else {
        "Concluído"
    }
    .to_owned();
    let event = walk.progress_event(&final_stage);
    (walk.on_progress)(event);
    let mut out = walk.out;
    // Display paths without `.` components (`/home/u/./.cache` -> `/home/u/.cache`).
    // The plan compares the same normalized form, so identity is unchanged.
    for candidate in &mut out.candidates {
        candidate.display_path = tidy_display_path(&candidate.display_path);
    }
    for issue in &mut out.issues {
        issue.display_path = tidy_display_path(&issue.display_path);
    }
    Ok(out)
}

#[cfg(test)]
mod pytest_budget_tests {
    use super::*;
    use std::fs;

    #[test]
    fn insufficient_pytest_validation_budget_emits_no_review_candidate() {
        let fixture = tempfile::tempdir().unwrap();
        let cache = fixture.path().join("project/.pytest_cache");
        let values = cache.join("v/cache");
        fs::create_dir_all(&values).unwrap();
        fs::write(cache.join("README.md"), b"# pytest cache directory #\n\nThis directory contains data from the pytest's cache plugin,\nwhich provides the `--lf` and `--ff` options, as well as the `cache` fixture.\n\n**Do not** commit this to version control.\nSee [the docs](https://docs.pytest.org/en/stable/how-to/cache.html) for more information.\n").unwrap();
        fs::write(
            cache.join(".gitignore"),
            b"# Created by pytest automatically.\n*\n",
        )
        .unwrap();
        fs::write(cache.join("CACHEDIR.TAG"), b"Signature: 8a477f597d28d172789f06886806bc55\n# This file is a cache directory tag created by pytest.\n# For information about cache directory tags, see:\n#\thttps://bford.info/cachedir/spec.html\n").unwrap();
        fs::write(values.join("nodeids"), br#"["tests/test_a.py::test_a"]"#).unwrap();

        let approved = ApprovedRoot::open(fixture.path(), ScopeKind::Projects).unwrap();
        let budget = crate::cache_rules::pytest_nodeids_max_validation_bytes() - 1;
        let mut stages = Vec::new();
        let snapshot = scan_with_validation_budget(
            &[approved],
            &ScanControl::default(),
            &ScanLimits::default(),
            |event| stages.push(event.stage),
            budget,
        )
        .unwrap();

        assert!(snapshot.complete);
        assert!(snapshot
            .candidates
            .iter()
            .all(|candidate| candidate.detector_id != "python.pytest-nodeids"));
        assert!(snapshot.candidates.iter().any(|candidate| {
            candidate.detector_id == "python.toolcache" && candidate.risk == Risk::Protected
        }));
        assert!(!stages
            .iter()
            .any(|stage| stage == "Validando índice de testes pytest"));
    }
}

#[cfg(test)]
mod os_error_tests {
    #[test]
    fn exdev_and_common_errnos_have_portuguese_text() {
        let text = super::os_error_pt("Invalid cross-device link (os error 18)");
        assert!(text.contains("outro sistema de arquivos"), "{text}");
        for code in [
            4, 6, 9, 11, 12, 16, 19, 22, 24, 26, 27, 31, 36, 61, 75, 95, 107, 110, 121, 122,
        ] {
            let raw = format!("X (os error {code})");
            assert!(!super::os_error_pt(&raw).contains("os error"), "{code}");
        }
    }
}
#[cfg(test)]
mod display_path_tests {
    #[test]
    fn dot_components_are_dropped_for_display_only() {
        assert_eq!(
            super::tidy_display_path("/home/u/./.cache/./pip"),
            "/home/u/.cache/pip"
        );
        assert_eq!(super::tidy_display_path("/home/u/../x"), "/home/u/../x");
    }
}
#[cfg(test)]
mod uv_cache_tests {
    use super::*;
    use std::fs;

    #[test]
    fn pycache_under_the_uv_cache_is_protected() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("xdg-cache");
        let project = cache.join("uv/simple");
        fs::create_dir_all(project.join("__pycache__")).unwrap();
        fs::write(project.join("mod.py"), b"x = 1\n").unwrap();
        fs::write(project.join("__pycache__/mod.cpython-313.pyc"), b"bytecode").unwrap();
        struct Clear;
        impl Drop for Clear {
            fn drop(&mut self) {
                CACHE_HOME_OVERRIDE.with(|slot| *slot.borrow_mut() = None);
            }
        }
        let _clear = Clear;
        CACHE_HOME_OVERRIDE.with(|slot| *slot.borrow_mut() = Some(cache));
        let snapshot = scan(
            &[ApprovedRoot::open(&project, ScopeKind::Projects).unwrap()],
            &ScanControl::default(),
            &ScanLimits::default(),
            |_| {},
        )
        .unwrap();
        let candidate = snapshot
            .candidates
            .iter()
            .find(|c| c.detector_id == "python.pycache")
            .unwrap();
        assert_eq!(candidate.risk, Risk::Protected);
        assert!(candidate.reason.contains("use `uv cache clean`"));
        assert_eq!(candidate.rebuild_hint, "use `uv cache clean`");
        assert!(matches!(candidate.supported_action, SupportedAction::None));
    }
}

#[cfg(test)]
mod trash_sensitivity_tests {
    use super::*;
    use std::fs;

    #[test]
    fn trash_keeps_agent_git_and_credential_entries_with_their_trashinfo() {
        let root = tempfile::tempdir().unwrap();
        let share = root.path().join("share");
        let trash = share.join("Trash");
        let files = trash.join("files");
        let info = trash.join("info");
        fs::create_dir_all(&files).unwrap();
        fs::create_dir_all(&info).unwrap();
        let cases = [
            ("codex", ".codex/session.json"),
            ("claude", ".claude/projects/a.json"),
            ("opencode", ".local/share/opencode/session.json"),
            ("cursor", ".cursor/argv.json"),
            ("repo", "src/.git/HEAD"),
            ("gitfile", ".git"),
            ("aws", ".aws/credentials"),
            ("kube", ".kube/config"),
            ("docker", ".docker/config.json"),
            ("netrc", ".netrc"),
            ("npmrc", ".npmrc"),
            ("pypirc", ".pypirc"),
            ("git-credentials", ".git-credentials"),
            ("rsa", "id_rsa"),
            ("rsa-pub", "id_rsa.pub"),
            ("ed25519", "id_ed25519"),
            ("ed25519-pub", "id_ed25519.pub"),
            ("key", "server.key"),
            ("p12", "client.p12"),
            ("pfx", "client.pfx"),
            ("kdbx", "vault.kdbx"),
        ];
        for (entry, relative) in cases {
            let path = files.join(entry).join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, b"keep").unwrap();
            fs::write(info.join(format!("{entry}.trashinfo")), b"[Trash Info]\n").unwrap();
        }
        fs::create_dir_all(files.join("notes")).unwrap();
        fs::write(files.join("notes/readme.txt"), b"plain").unwrap();
        fs::write(info.join("notes.trashinfo"), b"[Trash Info]\nPath=/notes\n").unwrap();

        struct Clear;
        impl Drop for Clear {
            fn drop(&mut self) {
                TRASH_DIR_OVERRIDE.with(|slot| *slot.borrow_mut() = None);
            }
        }
        let _clear = Clear;
        TRASH_DIR_OVERRIDE.with(|slot| *slot.borrow_mut() = Some(trash.clone()));
        let snapshot = scan(
            &[ApprovedRoot::open(&share, ScopeKind::Projects).unwrap()],
            &ScanControl::default(),
            &ScanLimits::default(),
            |_| {},
        )
        .unwrap();

        for (entry, _) in cases {
            let protected = snapshot.candidates.iter().find(|candidate| {
                candidate.detector_id == "protected.state"
                    && candidate.display_path.ends_with(&format!("/files/{entry}"))
            });
            let protected = protected.unwrap_or_else(|| panic!("missing protected entry {entry}"));
            assert_eq!(protected.risk, Risk::Protected, "{entry}");
            assert!(
                matches!(protected.supported_action, SupportedAction::None),
                "{entry}"
            );
            assert!(protected.reason.contains("somente análise"), "{entry}");
            assert!(
                !snapshot.candidates.iter().any(|candidate| {
                    candidate.detector_id == "trash.entry"
                        && candidate.display_path.ends_with(&format!("/files/{entry}"))
                }),
                "{entry} must not be deletable"
            );
            assert!(
                !snapshot.targets.values().any(|target| {
                    target
                        .files
                        .iter()
                        .any(|(path, _)| path.ends_with(format!("{entry}.trashinfo")))
                }),
                "{entry}.trashinfo must stay out of every deletion manifest"
            );
            assert_eq!(
                fs::read(info.join(format!("{entry}.trashinfo"))).unwrap(),
                b"[Trash Info]\n"
            );
        }
        let notes = snapshot
            .candidates
            .iter()
            .find(|candidate| {
                candidate.detector_id == "trash.entry"
                    && candidate.display_path.ends_with("/files/notes")
            })
            .unwrap();
        let target = snapshot.targets.get(&notes.id).unwrap();
        assert!(target
            .files
            .iter()
            .any(|(path, _)| path.ends_with("notes.trashinfo")));
    }
}

#[cfg(test)]
mod credential_cache_regressions {
    use super::*;
    #[test]
    fn exact_credential_names_do_not_match_source_or_templates() {
        for name in [
            "id_ecdsa",
            "id_ed25519_sk",
            "id_rsa.old",
            ".netrc",
            ".env.local",
            ".pgpass",
            "token.json",
            "backup.p12",
            "vault.kdbx",
            "credentials",
        ] {
            assert!(credential_name(std::ffi::OsStr::new(name)), "{name}");
        }
        for name in [
            "AGENTS.md",
            "CLAUDE.md",
            "service.key",
            "token.rs",
            "tokenizer.json",
            "tokens.json",
            "credentials.rs",
            "id_generator.js",
            "id_map.json",
            "id_ed25519.pub",
            "id_rsa-cert.pub",
            ".env.example",
            ".env.sample",
            ".env.template",
        ] {
            assert!(!protected_name(std::ffi::OsStr::new(name)), "{name}");
        }
        assert!(protected_name(std::ffi::OsStr::new("state.tfstate")));
    }
    #[test]
    fn signature_probe_never_follows_symlinks() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("ordinary"), "fixture").unwrap();
        std::os::unix::fs::symlink("ordinary", temp.path().join("link")).unwrap();
        let root = ApprovedRoot::open(temp.path(), ScopeKind::Projects).unwrap();
        assert!(sensitive_content(&root, Path::new("link")));
        assert!(!sensitive_content(&root, Path::new("ordinary")));
    }
}
