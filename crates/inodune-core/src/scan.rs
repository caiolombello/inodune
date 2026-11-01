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
pub fn xdg_thumbnail_root() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .filter(|p| !p.is_empty() && Path::new(p).is_absolute())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|p| Path::new(p).is_absolute())
                .map(|p| PathBuf::from(p).join(".cache"))
        })
        .map(|p| p.join("thumbnails"))
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
#[derive(Clone)]
pub struct ScannedTarget {
    pub root: ApprovedRoot,
    pub relative: PathBuf,
    pub fingerprint: Fingerprint,
    pub files: Vec<(PathBuf, Fingerprint)>,
    /// Non-deletable evidence (manifest/lock/source) bound to an eventual plan.
    pub witnesses: Vec<(PathBuf, Fingerprint)>,
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
    fn issue(&mut self, path: &Path, code: &str, message: &str) {
        self.out.complete = false;
        self.out.stats.skipped += 1;
        if self.out.issues.len() < self.limits.max_issues {
            self.out.issues.push(ScanIssue {
                display_path: path.to_string_lossy().into(),
                code: code.into(),
                message: message.into(),
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
                self.out.stats.skipped += 1;
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
    ) -> (u64, u64, bool, Vec<(PathBuf, Fingerprint)>) {
        let mut pending = vec![(rel.to_path_buf(), 0usize)];
        let mut logical = 0u64;
        let mut allocated = 0u64;
        let mut complete = true;
        let mut files = Vec::new();
        let mut seen = HashSet::new();
        while let Some((path, depth)) = pending.pop() {
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
            let Some(st) = self.metadata(root, &path) else {
                complete = false;
                continue;
            };
            match FileType::from_raw_mode(st.st_mode) {
                FileType::Directory => {
                    if let Some(names) = self.names(root, &path) {
                        for name in names {
                            pending.push((path.join(name), depth + 1));
                        }
                    } else {
                        complete = false;
                    }
                }
                FileType::RegularFile => {
                    if st.st_nlink != 1 {
                        self.issue(
                            &root.path.join(&path),
                            "hardlink",
                            "Hardlink: ganho e propriedade do conteúdo incertos.",
                        );
                        complete = false;
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
                _ => {
                    self.issue(
                        &root.path.join(&path),
                        "symlink_or_special",
                        "Link ou arquivo especial preservado; conteúdo não seguido.",
                    );
                    complete = false;
                }
            }
        }
        (logical, allocated, complete, files)
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
        let (logical, allocated, mut complete, files) = self.measure(root, rel);
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
        let risk = if complete { risk } else { Risk::Protected };
        let id = format!("{}-{}", self.out.id, self.out.candidates.len());
        self.out.targets.insert(
            id.clone(),
            ScannedTarget {
                root: root.clone(),
                relative: rel.to_path_buf(),
                fingerprint: Fingerprint::from_stat(&st),
                files,
                witnesses: Vec::new(),
                complete,
            },
        );
        self.out.candidates.push(Candidate{
            id, title:title.into(),display_path:root.path.join(rel).to_string_lossy().into(),detector_id:detector.into(),detector_version:"1".into(),scope_id:root.display_path(),category:category.into(),project:project.into(),
            selected_by_default:risk==Risk::Low,risk,logical_bytes:logical,allocated_bytes_estimate:Some(allocated),identity:Some(Identity{dev:st.st_dev,ino:st.st_ino,mtime_unix:st.st_mtime}),coverage_key:(coverage_complete && stable).then(||format!("{}:{}",st.st_dev,st.st_ino)),
            reason:if complete {reason.into()}else{format!("{reason} Cobertura incompleta, links, conteúdo desconhecido ou hardlinks: protegido.")},
            impact:if py||thumbs {"O aplicativo recriará o cache; a primeira abertura pode ser mais lenta.".into()}else{"Pode conter dados de trabalho ou exigir reconstrução/download. Inventário não autoriza remoção.".into()},
            rebuild_hint:if py {"Bytecode recriado ao importar o módulo.".into()}else if thumbs{"Miniaturas recriadas pelo aplicativo.".into()}else{"Custo não medido; revisão necessária.".into()},
            preserved:"Código, estado, configuração, credenciais, sessões e memória nunca entram em remoção genérica.".into(),evidence:vec!["root-approved".into(),"openat2-no-symlinks-no-xdev".into(),if complete{"coverage-complete"}else{"coverage-incomplete"}.into()],activity:format!("mtime Unix {} (metadado; não comprova desuso)",st.st_mtime),supported_action:if complete && (py||thumbs){SupportedAction::CacheContents}else{SupportedAction::None},
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
                let bytes = files
                    .iter()
                    .try_fold(0u64, |n, (_, fp)| n.checked_add(fp.size))
                    .unwrap_or(u64::MAX);
                if bytes > self.validation_bytes_left {
                    continue;
                }
                self.validation_bytes_left -= bytes;
                let event = self.progress_event("Verificando checksum de provider local");
                (self.on_progress)(event);
                let check = crate::terraform::verify_provider(root, &path, &files, self.control);
                if !check.eligible {
                    continue;
                }
                let Some(st) = self.metadata(root, &path) else {
                    continue;
                };
                let witnesses = check
                    .lock_relative
                    .zip(check.lock_fingerprint)
                    .into_iter()
                    .collect();
                proposals.push((
                    path,
                    Fingerprint::from_stat(&st),
                    files,
                    witnesses,
                    "terraform.provider-package",
                    "Provider Terraform verificado",
                    check.evidence,
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
                ));
            }
        }
        for (path, fp, files, witnesses, detector, title, evidence) in proposals {
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
                    complete: true,
                },
            );
            self.out.candidates.push(Candidate{id,title:title.into(),display_path:root.path.join(path).to_string_lossy().into(),detector_id:detector.into(),detector_version:"f3-v1".into(),scope_id:root.display_path(),category:"Desenvolvimento".into(),project:project.into(),risk:Risk::Review,logical_bytes:bytes,allocated_bytes_estimate:Some(allocated),identity:Some(Identity{dev:fp.dev,ino:fp.ino,mtime_unix:fp.mtime}),coverage_key:Some(format!("{}:{}",fp.dev,fp.ino)),selected_by_default:false,reason:if providers{"Pacote local confere com checksum h1 do lockfile e layout conhecido. Isso não prova assinatura de publicação; revisão individual obrigatória."}else{"Somente JSON diagnóstico Cargo 1.93 reconhecido. Hashes, binários, fontes e demais artefatos permanecem protegidos."}.into(),impact:if providers{"A próxima inicialização Terraform poderá precisar baixar o provider novamente. Não executamos terraform init."}else{"Cargo pode reconstruir artefatos ao perder o metadado de fingerprint; custo não medido."}.into(),rebuild_hint:if providers{"Reinstalação pelo Terraform, conforme lockfile preservado; sem estimativa de tempo/rede."}else{"Cargo regenera metadados; uma recompilação pode ser necessária."}.into(),preserved:"Pastas, manifests, lockfiles, scripts, estado, configuração e conteúdo desconhecido permanecem.".into(),evidence,activity:"Revisão por item; idade não comprova desuso.".into(),supported_action:SupportedAction::ReviewedCacheContents});
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
                parent.reason="Conteúdo remanescente protegido; subconjuntos verificados aparecem separadamente, sem dupla contagem.".into();
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
    fn walk_root(&mut self, root: &ApprovedRoot) {
        if root.is_excluded(Path::new(".")) {
            self.out.stats.skipped += 1;
            if self.out.issues.len() < self.limits.max_issues {
                self.out.issues.push(ScanIssue {
                    display_path: root.display_path(),
                    code: "policy_excluded".into(),
                    message: "Escopo excluído por política; conteúdo não acessado.".into(),
                });
            }
            return;
        }
        if root.kind == ScopeKind::Protected || is_protected_agent_path(root.path()) {
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
                let Some(st) = self.metadata(root, &path) else {
                    continue;
                };
                let kind = FileType::from_raw_mode(st.st_mode);
                if kind == FileType::Symlink {
                    self.issue(
                        &root.path.join(&path),
                        "symlink",
                        "Link preservado; alvo não acessado.",
                    );
                    continue;
                }
                if name == ".git" {
                    self.git_inventory(root, &rel, &project);
                    continue;
                }
                if kind == FileType::RegularFile {
                    if protected_name(name) {
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
                let n = name.to_string_lossy();
                if ["node_modules", ".venv", "venv", ".ssh", ".gnupg"].contains(&n.as_ref()) {
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
                        ("next.cache", "Cache Next.js (protegido)")
                    } else if n == "_cacache" {
                        ("npm.cache", "Possível cache npm (protegido)")
                    } else {
                        ("python.pip-cache", "Cache pip (protegido)")
                    };
                    self.candidate(root,&path,&project,CandidatePolicy{detector,title,category:"Desenvolvimento",risk:Risk::Protected,reason:"Localização documentada; formato/conteúdo por versão e uso atual não verificados. Inventário sem ação de remoção."});
                    continue;
                }
                if n == "__pycache__" {
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
                    self.candidate(root,&path,&project,CandidatePolicy{detector:"python.toolcache",title:"Cache de ferramenta Python",category:"Desenvolvimento",risk:Risk::Protected,reason:"Formato específico da ferramenta não validado para remoção; inventário."});
                    continue;
                }
                if n == "target" && has("Cargo.toml") {
                    self.candidate(root,&path,&project,CandidatePolicy{detector:"rust.target",title:"Artefatos Cargo",category:"Desenvolvimento",risk:Risk::Protected,reason:"target junto de Cargo.toml; pode conter binários finais e exige revisão."});
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
fn protected_name(name: &std::ffi::OsStr) -> bool {
    let n = name.to_string_lossy();
    n.contains(".tfstate")
        || n.ends_with(".tfvars")
        || n.ends_with(".tfplan")
        || n == ".terraform.lock.hcl"
        || n == ".env"
        || ["AGENTS.md", "CLAUDE.md", "auth.json", "credentials.json"].contains(&n.as_ref())
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
    Ok(walk.out)
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
