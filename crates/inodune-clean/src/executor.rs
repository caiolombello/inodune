//! Authorized deletion of a scanned manifest. No shell. Symlinks are unlinked, never followed.
//!
//! Linux has no compare-inode-and-unlink syscall. We capture a leaf into a private
//! directory, check its identity again, then act there. This resists ordinary cache
//! writers and namespace swaps; it is not a sandbox against hostile same-UID code.
use crate::{
    plan::{Action, AuthorizedPlan},
    storage::{self, Trash},
};
use inodune_core::scan::{ApprovedRoot, Fingerprint, ScanControl, ScannedTarget};
use rustix::{
    fd::{AsFd, OwnedFd},
    fs::{self, AtFlags, FileType, Mode, OFlags, RenameFlags},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Trashed,
    Removed,
    SkippedChanged,
    Failed,
    Cancelled,
    Quarantined,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileOutcome {
    pub display_path: String,
    pub status: FileStatus,
    pub logical_bytes: u64,
    pub message: String,
    pub preserved_at: Option<String>,
    #[serde(default)]
    pub durability_confirmed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemResult {
    pub candidate_id: String,
    pub files: Vec<FileOutcome>,
    pub complete: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VolumeDelta {
    pub device: u64,
    pub available_before: u64,
    pub available_after: Option<u64>,
    pub observed_delta: Option<i128>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub schema_version: u32,
    pub plan_id: String,
    pub action: Action,
    pub items: Vec<ItemResult>,
    pub volumes: Vec<VolumeDelta>,
    pub complete: bool,
    pub cancelled: bool,
    pub logical_bytes_processed: u64,
    /// Always null: free-space variation cannot be attributed solely to this app.
    pub attributed_freed_bytes: Option<u64>,
    pub measurement_note: String,
    pub warnings: Vec<String>,
    pub history_path: Option<String>,
    #[serde(default)]
    pub journal_path: Option<String>,
}
#[derive(Clone, Serialize)]
pub struct ExecutionProgress {
    pub completed_files: usize,
    pub total_files: usize,
    pub completed_items: usize,
    pub total_items: usize,
}
/// Trusted backend configuration, never accepted through IPC.
pub struct ExecutionOptions {
    pub trash_directory: Option<PathBuf>,
    pub history_directory: Option<PathBuf>,
}
impl ExecutionOptions {
    pub fn local() -> Result<Self, String> {
        Ok(Self {
            trash_directory: Some(
                storage::xdg_home("XDG_DATA_HOME", ".local/share")?.join("Trash"),
            ),
            history_directory: Some(
                storage::xdg_home("XDG_STATE_HOME", ".local/state")?.join("inodune/reports"),
            ),
        })
    }
}
#[derive(Clone, Copy)]
enum CacheKind {
    Python,
    Thumbnails,
    CargoMetadata,
    TerraformProvider,
    PytestNodeIds,
    Rebuildable,
}
impl CacheKind {
    fn detector(id: &str) -> Option<Self> {
        Some(match id {
            "python.pycache" => Self::Python,
            "thumbnails" => Self::Thumbnails,
            "cargo.fingerprint-json" => Self::CargoMetadata,
            "terraform.provider-package" => Self::TerraformProvider,
            "python.pytest-nodeids" => Self::PytestNodeIds,
            "rust.target" | "python.pip-cache" | "npm.cache" | "next.cache"
            | "python.mypy-cache" | "python.ruff-cache" | "trash.entry" | "node.modules"
            | "uv.cache" => Self::Rebuildable,
            _ => return None,
        })
    }
}
#[derive(Clone, Copy)]
enum Phase {
    Pinned,
    Captured,
}

pub fn preflight(options: &ExecutionOptions, action: Action) -> Result<(), String> {
    let history = options
        .history_directory
        .as_ref()
        .ok_or("O registro da operação não está configurado.")?;
    storage::probe_reports(history)
        .map_err(|e| format!("O registro da operação não está utilizável: {e}"))?;
    if action == Action::Trash {
        let trash = options
            .trash_directory
            .as_ref()
            .ok_or("A Lixeira não está configurada.")?;
        storage::Trash::open(trash).map_err(|e| format!("A Lixeira não está utilizável: {e}"))?;
    }
    Ok(())
}
pub fn execute(
    plan: AuthorizedPlan,
    options: &ExecutionOptions,
    control: &ScanControl,
    on_progress: impl FnMut(ExecutionProgress),
) -> ExecutionReport {
    execute_inner(plan, options, control, on_progress, |_, _| {})
}
fn execute_inner(
    mut plan: AuthorizedPlan,
    options: &ExecutionOptions,
    control: &ScanControl,
    mut on_progress: impl FnMut(ExecutionProgress),
    mut hook: impl FnMut(Phase, &Path),
) -> ExecutionReport {
    let total_files = plan
        .ids
        .iter()
        .filter_map(|id| plan.snapshot.targets.get(id))
        .map(|t| t.files.len())
        .sum();
    let mut progress = ExecutionProgress {
        completed_files: 0,
        total_files,
        completed_items: 0,
        total_items: plan.ids.len(),
    };
    let mut report=ExecutionReport{schema_version:1,plan_id:plan.view.id.clone(),action:plan.action,items:vec![],volumes:vec![],complete:true,cancelled:false,logical_bytes_processed:0,attributed_freed_bytes:None,measurement_note:if plan.action==Action::Trash{"Arquivos movidos para a Lixeira continuam ocupando espaço. Variações observadas no volume não são ganho atribuído à limpeza."}else{"Variação observada no filesystem; arquivos abertos, CoW, snapshots e atividade externa podem adiar ou alterar a liberação. Sem garantia de apagamento seguro."}.into(),warnings:vec![],history_path:None,journal_path:None};
    let mut journal = options
        .history_directory
        .as_ref()
        .ok_or("Histórico local não configurado".to_string())
        .and_then(|p| storage::Journal::create(p));
    let mut journal_error = None;
    match &mut journal {
        Ok(j) => {
            report.journal_path = Some(j.path.to_string_lossy().into());
            if let Err(e) = j
                .record(&serde_json::json!({"phase":"plan","plan":plan.view,"state":"in_progress"}))
            {
                journal_error = Some(e);
            }
        }
        Err(e) => journal_error = Some(e.clone()),
    }
    if let Some(e) = &journal_error {
        report.warnings.push(format!(
            "Histórico local não salvo; nenhuma operação iniciada: {e}"
        ));
        report.complete = false;
    }
    let trash = if plan.action == Action::Trash {
        options
            .trash_directory
            .as_ref()
            .ok_or("Lixeira não configurada".into())
            .and_then(|p| Trash::open(p))
            .map(Some)
    } else {
        Ok(None)
    };
    let mut devices = HashSet::new();
    for id in &plan.ids {
        if let Some(t) = plan.snapshot.targets.get(id) {
            if devices.insert(t.fingerprint.dev) {
                match t.root.volume() {
                    Ok(v) => report.volumes.push(VolumeDelta {
                        device: v.device,
                        available_before: v.available_bytes,
                        available_after: None,
                        observed_delta: None,
                    }),
                    Err(e) => report.warnings.push(e),
                }
            }
        }
    }
    for id in std::mem::take(&mut plan.ids) {
        let Some(target) = plan.snapshot.targets.get(&id) else {
            report.complete = false;
            report.warnings.push("Manifesto ausente".into());
            continue;
        };
        let detector_id = plan
            .snapshot
            .candidates
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.detector_id.clone())
            .unwrap_or_default();
        let kind = CacheKind::detector(&detector_id);
        let action = plan.action;
        let mut item = ItemResult {
            candidate_id: id,
            files: vec![],
            complete: true,
        };
        let preflight = if let Some(e) = &journal_error {
            Err(format!("Journal indisponível: {e}"))
        } else if control.is_cancelled() {
            Err("Cancelado antes do item".into())
        } else {
            validate_target(target)
                .and_then(|_| validate_policy(target, kind, control))
                .and_then(|_| {
                    if action != Action::Trash {
                        return Ok(());
                    }
                    trash.as_ref().map_err(Clone::clone).and_then(|t| {
                        if let Some(t) = t {
                            t.revalidate(target.fingerprint.dev)
                        } else {
                            Ok(())
                        }
                    })
                })
        };
        match preflight {
            Err(e) => {
                item.complete = false;
                item.files.push(outcome(
                    &target.root.path().join(&target.relative),
                    if control.is_cancelled() {
                        FileStatus::Cancelled
                    } else {
                        FileStatus::SkippedChanged
                    },
                    0,
                    &e,
                    None,
                ));
            }
            Ok(()) => {
                match Stage::create(&target.root) {
                    Err(e) => {
                        item.complete = false;
                        item.files.push(outcome(
                            &target.root.path().join(&target.relative),
                            FileStatus::Failed,
                            0,
                            &e,
                            None,
                        ));
                    }
                    Ok(stage) => {
                        for (relative, expected) in &target.files {
                            let path = target.root.path().join(relative);
                            if journal_error.is_none() {
                                if let Ok(j) = &mut journal {
                                    if let Err(e)=j.record(&serde_json::json!({"phase":"intent","path":path,"candidate_id":item.candidate_id})){journal_error=Some(e);}
                                }
                            }
                            let result = if let Some(e) = &journal_error {
                                outcome(
                                    &path,
                                    FileStatus::Failed,
                                    0,
                                    &format!("Journal indisponível; operação não iniciada: {e}"),
                                    None,
                                )
                            } else if control.is_cancelled() {
                                outcome(
                                    &path,
                                    FileStatus::Cancelled,
                                    0,
                                    "Cancelado antes desta operação",
                                    None,
                                )
                            } else if let (Some(kind), Ok(journal)) = (kind, &mut journal) {
                                process_file(
                                    Leaf {
                                        target,
                                        relative,
                                        expected,
                                        kind,
                                    },
                                    &stage,
                                    trash.as_ref().ok().and_then(|t| t.as_ref()),
                                    action,
                                    &mut hook,
                                    journal,
                                )
                            } else {
                                outcome(&path, FileStatus::Failed, 0, "Journal indisponível", None)
                            };
                            if !result.durability_confirmed {
                                item.complete = false;
                            }
                            if matches!(result.status, FileStatus::Removed | FileStatus::Trashed) {
                                report.logical_bytes_processed = report
                                    .logical_bytes_processed
                                    .saturating_add(result.logical_bytes);
                            } else {
                                item.complete = false;
                            }
                            if let Ok(j) = &mut journal {
                                if let Err(e) = j
                                    .record(&serde_json::json!({"phase":"outcome","result":result}))
                                {
                                    journal_error = Some(e.clone());
                                    report.warnings.push(format!(
                                        "Resultado aplicado mas journal não sincronizado: {e}"
                                    ));
                                    report.complete = false;
                                }
                            }
                            item.files.push(result);
                            progress.completed_files += 1;
                            on_progress(progress.clone());
                        }
                        remove_empty_dirs(target);
                        match remaining_cache_entries(target) {
                            Ok(true) => {
                                item.complete = false;
                                report.warnings.push(format!("Conteúdo novo ou remanescente preservado em {}. Refaça a análise.",target.root.path().join(&target.relative).display()));
                            }
                            Err(e) => {
                                item.complete = false;
                                report.warnings.push(e);
                            }
                            Ok(false) => {}
                        }
                        if let Err(e) = stage.finish(&target.root) {
                            item.complete = false;
                            report.warnings.push(e);
                        }
                    }
                }
            }
        }
        report.complete &= item.complete;
        report.items.push(item);
        progress.completed_items += 1;
        on_progress(progress.clone());
    }
    report.cancelled = control.is_cancelled();
    report.complete &= !report.cancelled;
    for delta in &mut report.volumes {
        if let Some(t) = plan
            .snapshot
            .targets
            .values()
            .find(|t| t.fingerprint.dev == delta.device)
        {
            match t.root.volume() {
                Ok(v) => {
                    delta.available_after = Some(v.available_bytes);
                    delta.observed_delta =
                        Some(i128::from(v.available_bytes) - i128::from(delta.available_before));
                }
                Err(e) => report.warnings.push(e),
            }
        }
    }
    if let Ok(j) = &mut journal {
        if let Err(e) = j.record(&serde_json::json!({"phase":"finished","report":report})) {
            report.complete = false;
            report
                .warnings
                .push(format!("Journal final não sincronizado: {e}"));
        }
    }
    if let Some(path) = &options.history_directory {
        match storage::save_report(path, &report) {
            Ok(p) => report.history_path = Some(p.to_string_lossy().into()),
            Err(e) => {
                report
                    .warnings
                    .push(format!("Histórico local não salvo: {e}"));
                report.complete = false;
            }
        }
    }
    report
}
fn remaining_cache_entries(target: &ScannedTarget) -> Result<bool, String> {
    let mut pending = vec![target.relative.clone()];
    let mut walked = 0usize;
    while let Some(path) = pending.pop() {
        if target.preserved.iter().any(|keep| path.starts_with(keep)) {
            continue;
        }
        walked += 1;
        if walked > 100_000 {
            return Err("Verificação de remanescentes excedeu limite".into());
        }
        let st = match target.root.lstat_relative(&path) {
            Ok(st) => st,
            Err(rustix::io::Errno::NOENT) => continue,
            Err(e) => return Err(e.to_string()),
        };
        if FileType::from_raw_mode(st.st_mode) != FileType::Directory {
            return Ok(true);
        }
        let fd = target
            .root
            .open_relative(&path, OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOATIME)
            .map_err(|e| e.to_string())?;
        let mut buffer = [std::mem::MaybeUninit::uninit(); 8192];
        let mut entries = fs::RawDir::new(&fd, &mut buffer);
        while let Some(entry) = entries.next() {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_bytes();
            if name != b"." && name != b".." {
                if walked.saturating_add(pending.len()) >= 100_000 {
                    return Err("Verificação de remanescentes excedeu limite".into());
                }
                pending.push(path.join(std::ffi::OsStr::from_bytes(name)));
            }
        }
    }
    Ok(false)
}
fn validate_target(target: &ScannedTarget) -> Result<(), String> {
    target.root.revalidate()?;
    let leaf_target = FileType::from_raw_mode(target.fingerprint.mode) == FileType::RegularFile;
    validate_ancestors(
        target,
        if leaf_target {
            target.relative.parent().unwrap_or(Path::new("."))
        } else {
            &target.relative
        },
    )?;
    let fd = target
        .root
        .open_relative(
            &target.relative,
            if leaf_target {
                OFlags::PATH
            } else {
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOATIME
            },
        )
        .map_err(|e| e.to_string())?;
    let fp = Fingerprint::from_stat(&fs::fstat(&fd).map_err(|e| e.to_string())?);
    if !target.complete || fp != target.fingerprint {
        return Err("Cache alterado desde a análise; refaça o scan.".into());
    }
    if target.files.iter().any(|(p, _)| {
        if leaf_target {
            target.files.len() != 1 || p != &target.relative
        } else {
            let inside = p.starts_with(&target.relative) && p != &target.relative;
            let trashinfo = inodune_core::scan::paired_trashinfo(&target.relative)
                .is_some_and(|info| p == &info);
            !inside && !trashinfo
        }
    }) {
        return Err("O manifesto contém um caminho fora do item aprovado.".into());
    }
    if target
        .files
        .iter()
        .any(|(path, _)| target.preserved.iter().any(|keep| path.starts_with(keep)))
    {
        return Err("Manifesto sobrepõe uma exclusão protegida".into());
    }
    validate_witnesses(target)
}
fn validate_witnesses(target: &ScannedTarget) -> Result<(), String> {
    if inodune_core::terraform::is_provider_package_path(&target.relative)
        && !inodune_core::terraform::provider_locks_unchanged(
            &target.root,
            &target.relative,
            &target.witnesses,
        )
    {
        return Err("Locks relevantes mudaram; refaça a análise.".into());
    }

    for (path, expected) in &target.witnesses {
        if expected.mode & 0o022 != 0 {
            return Err(
                "Evidência gravável por terceiros; limpeza recusada sem alterar permissões.".into(),
            );
        }
        let fd = target
            .root
            .open_relative(path, OFlags::PATH)
            .map_err(|e| format!("Evidência desapareceu: {e}"))?;
        if Fingerprint::from_stat(&fs::fstat(fd).map_err(|e| e.to_string())?) != *expected {
            return Err("Manifest, fonte ou lockfile mudou desde a análise; preserve o cache e analise novamente.".into());
        }
    }
    Ok(())
}
fn validate_policy(
    target: &ScannedTarget,
    kind: Option<CacheKind>,
    control: &ScanControl,
) -> Result<(), String> {
    match kind.ok_or("Detector sem ação aprovada")? {
        CacheKind::CargoMetadata => {
            if inodune_core::cache_rules::validate_leaf(&target.root, &target.relative).as_ref()
                != Some(&target.fingerprint)
            {
                return Err("Metadado Cargo não confere mais com o formato aprovado.".into());
            }
        }
        CacheKind::TerraformProvider => {
            let check = inodune_core::terraform::assess_provider(
                &target.root,
                &target.relative,
                &target.files,
            );
            if !check.eligible {
                return Err(check.reason);
            }
            let _ = control;
        }
        CacheKind::PytestNodeIds => {
            if inodune_core::cache_rules::validate_pytest_nodeids_leaf(
                &target.root,
                &target.relative,
            )
            .as_ref()
                != Some(&target.fingerprint)
            {
                return Err("Índice pytest ou marcadores mudaram desde a análise.".into());
            }
        }
        _ => {}
    }
    validate_witnesses(target)
}
struct Stage {
    fd: OwnedFd,
    name: String,
    path: PathBuf,
    identity: (u64, u64),
}
impl Stage {
    fn create(root: &ApprovedRoot) -> Result<Self, String> {
        root.revalidate()?;
        let name = storage::random_name(".inodune-stage-")?;
        fs::mkdirat(root.directory_fd(), &name, Mode::from_raw_mode(0o700))
            .map_err(|e| e.to_string())?;
        let fd = storage::open_child(root.directory_fd(), Path::new(&name))
            .map_err(|e| e.to_string())?;
        storage::check_private(&fd)?;
        let st = fs::fstat(&fd).map_err(|e| e.to_string())?;
        Ok(Self {
            fd,
            name: name.clone(),
            path: root.path().join(name),
            identity: (st.st_dev, st.st_ino),
        })
    }
    fn revalidate(&self, root: &ApprovedRoot) -> Result<(), String> {
        root.revalidate()?;
        let fd = storage::open_child(root.directory_fd(), Path::new(&self.name))
            .map_err(|e| e.to_string())?;
        storage::check_private(&fd)?;
        let st = fs::fstat(&fd).map_err(|e| e.to_string())?;
        if (st.st_dev, st.st_ino) != self.identity {
            return Err("Área privada mudou; conteúdo preservado.".into());
        }
        Ok(())
    }
    fn finish(self, root: &ApprovedRoot) -> Result<(), String> {
        self.revalidate(root)?;
        // rmdir succeeds only when empty; unknown/recovery files are never removed.
        fs::unlinkat(root.directory_fd(), &self.name, AtFlags::REMOVEDIR).map_err(|e| {
            format!(
                "Área de recuperação preservada em {}: {e}",
                self.path.display()
            )
        })
    }
}
fn validate_ancestors(target: &ScannedTarget, path: &Path) -> Result<(), String> {
    inodune_core::scan::cleanup_ancestors_allowed(&target.root, path)
}
fn parent_stable(target: &ScannedTarget, parent: &OwnedFd, rel: &Path) -> Result<(), String> {
    target.root.revalidate()?;
    validate_ancestors(target, rel)?;
    let current = target
        .root
        .open_relative(rel, OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOATIME)
        .map_err(|e| e.to_string())?;
    let a = fs::fstat(parent).map_err(|e| e.to_string())?;
    let b = fs::fstat(current).map_err(|e| e.to_string())?;
    if (a.st_dev, a.st_ino) != (b.st_dev, b.st_ino) {
        return Err("Pasta do cache foi substituída.".into());
    }
    Ok(())
}
fn leaf_fingerprint(fd: impl AsFd, name: &std::ffi::OsStr) -> Result<Fingerprint, String> {
    let file = fs::openat2(
        fd,
        name,
        OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
        fs::ResolveFlags::BENEATH
            | fs::ResolveFlags::NO_SYMLINKS
            | fs::ResolveFlags::NO_MAGICLINKS
            | fs::ResolveFlags::NO_XDEV,
    )
    .map_err(|e| e.to_string())?;
    Ok(Fingerprint::from_stat(
        &fs::fstat(file).map_err(|e| e.to_string())?,
    ))
}
fn same_after_rename(a: &Fingerprint, b: &Fingerprint) -> bool {
    // rename updates ctime. All other measured identity/content metadata must match.
    a.dev == b.dev
        && a.ino == b.ino
        && a.size == b.size
        && a.blocks == b.blocks
        && a.mode == b.mode
        && a.nlink == b.nlink
        && a.uid == b.uid
        && a.mtime == b.mtime
        && a.mtime_nsec == b.mtime_nsec
}
struct Leaf<'a> {
    target: &'a ScannedTarget,
    relative: &'a Path,
    expected: &'a Fingerprint,
    kind: CacheKind,
}

fn lstat_child(parent: &OwnedFd, name: &std::ffi::OsStr) -> Result<rustix::fs::Stat, String> {
    let file = fs::openat2(
        parent,
        name,
        OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
        fs::ResolveFlags::BENEATH | fs::ResolveFlags::NO_MAGICLINKS | fs::ResolveFlags::NO_XDEV,
    )
    .map_err(|e| e.to_string())?;
    fs::fstat(file).map_err(|e| e.to_string())
}
fn links_inside(target: &ScannedTarget, dev: u64, ino: u64, nlink: u64) -> bool {
    count_inode(&target.root, &target.relative, dev, ino, &target.preserved)
        .is_ok_and(|count| count == nlink)
}
fn count_inode(
    root: &ApprovedRoot,
    relative: &Path,
    dev: u64,
    ino: u64,
    preserved: &[PathBuf],
) -> Result<u64, String> {
    let mut pending = vec![relative.to_path_buf()];
    let mut seen = 0u64;
    let mut walked = 0usize;
    while let Some(path) = pending.pop() {
        if preserved.iter().any(|keep| path.starts_with(keep)) {
            continue;
        }
        walked += 1;
        if walked > 100_000 {
            return Err("contagem de hardlink excedeu o limite".into());
        }
        let st = root.lstat_relative(&path).map_err(|e| e.to_string())?;
        if st.st_dev == dev && st.st_ino == ino {
            seen += 1;
        }
        if FileType::from_raw_mode(st.st_mode) != FileType::Directory {
            continue;
        }
        let fd = root
            .open_relative(&path, OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOATIME)
            .map_err(|e| e.to_string())?;
        let mut buffer = [std::mem::MaybeUninit::uninit(); 8192];
        let mut entries = fs::RawDir::new(&fd, &mut buffer);
        while let Some(entry) = entries.next() {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_bytes();
            if name == b"." || name == b".." {
                continue;
            }
            pending.push(path.join(std::ffi::OsStr::from_bytes(name)));
        }
    }
    Ok(seen)
}
fn remove_empty_dirs(target: &ScannedTarget) {
    let mut dirs: Vec<std::path::PathBuf> = target
        .files
        .iter()
        .filter_map(|(path, _)| path.parent().map(|parent| parent.to_path_buf()))
        .filter(|path| path.starts_with(&target.relative) && path != &target.relative)
        .collect();
    dirs.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    dirs.dedup();
    for dir in dirs {
        if target
            .preserved
            .iter()
            .any(|keep| keep.starts_with(&dir) || dir.starts_with(keep))
        {
            continue;
        }
        let Some(name) = dir.file_name() else {
            continue;
        };
        let parent = dir.parent().unwrap_or(std::path::Path::new("."));
        let Ok(parent_fd) = target
            .root
            .open_relative(parent, OFlags::RDONLY | OFlags::DIRECTORY)
        else {
            continue;
        };
        let _ = fs::unlinkat(&parent_fd, name, AtFlags::REMOVEDIR);
    }
}
fn process_file(
    leaf: Leaf<'_>,
    stage: &Stage,
    trash: Option<&Trash>,
    action: Action,
    hook: &mut impl FnMut(Phase, &Path),
    journal: &mut storage::Journal,
) -> FileOutcome {
    let Leaf {
        target,
        relative,
        expected,
        kind,
    } = leaf;
    // Same file without `.` components: used in reports and Trash `Path=`.
    let original = inodune_core::scan::tidy_path(&target.root.path().join(relative));
    let fail = |e: &str| outcome(&original, FileStatus::SkippedChanged, 0, e, None);
    let parent_rel = relative.parent().unwrap_or(Path::new("."));
    let Some(name) = relative.file_name() else {
        return fail("Nome de arquivo inválido");
    };
    let parent = match target.root.open_relative(
        parent_rel,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOATIME,
    ) {
        Ok(fd) => fd,
        Err(e) => return fail(&e.to_string()),
    };
    if let Ok(st) = lstat_child(&parent, name) {
        if FileType::from_raw_mode(st.st_mode) == FileType::Symlink {
            let fp = Fingerprint::from_stat(&st);
            let uid = rustix::process::getuid().as_raw();
            if fp.dev != expected.dev
                || fp.ino != expected.ino
                || fp.uid != uid
                || fp.mtime != expected.mtime
            {
                return fail("Link mudou; preservado.");
            }
            if let Err(e) = parent_stable(target, &parent, parent_rel) {
                return fail(&e);
            }
            return match action {
                Action::Permanent => match fs::unlinkat(&parent, name, AtFlags::empty()) {
                    Ok(()) => outcome(
                        &original,
                        FileStatus::Removed,
                        expected.size,
                        "Link removido sem seguir o alvo.",
                        None,
                    ),
                    Err(e) => fail(&e.to_string()),
                },
                Action::Trash => {
                    let Some(trash) = trash else {
                        return fail("Lixeira indisponível");
                    };
                    match trash.reserve(&original).and_then(|reservation| {
                        fs::renameat_with(
                            &parent,
                            name,
                            &trash.files,
                            &reservation,
                            RenameFlags::NOREPLACE,
                        )
                        .map_err(|e| e.to_string())?;
                        Ok(trash.destination(&reservation))
                    }) {
                        Ok(dest) => outcome(
                            &original,
                            FileStatus::Trashed,
                            expected.size,
                            "Link movido à Lixeira sem seguir o alvo.",
                            Some(dest.to_string_lossy().into()),
                        ),
                        Err(e) => fail(&e),
                    }
                }
            };
        }
    }
    let pinned = match target.root.open_relative(relative, OFlags::PATH) {
        Ok(fd) => fd,
        Err(e) => return fail(&e.to_string()),
    };
    let (fp, leaf_gid) = match fs::fstat(&pinned) {
        Ok(st) => (Fingerprint::from_stat(&st), st.st_gid),
        Err(e) => return fail(&e.to_string()),
    };
    let uid = rustix::process::getuid().as_raw();
    let same_file = fp.dev == expected.dev
        && fp.ino == expected.ino
        && fp.size == expected.size
        && fp.mode == expected.mode
        && fp.uid == expected.uid
        && fp.mtime == expected.mtime
        && fp.mtime_nsec == expected.mtime_nsec
        && fp.nlink >= 1
        && fp.nlink <= expected.nlink;
    if !same_file || fp.uid != uid || FileType::from_raw_mode(fp.mode) != FileType::RegularFile {
        return fail("Arquivo mudou, trocou de dono ou não é regular.");
    }
    if relative
        .components()
        .any(|part| inodune_core::scan::protected_name(part.as_os_str()))
        || inodune_core::scan::sensitive_content(&target.root, relative)
    {
        return fail("Credencial, chave privada ou leitura insegura; arquivo protegido.");
    }
    if fp.nlink > 1 && !links_inside(target, fp.dev, fp.ino, fp.nlink) {
        return fail("Hardlink com elo fora do item; preservado.");
    }
    let python = matches!(kind, CacheKind::Python);
    let valid = match kind {
        CacheKind::Python => {
            inodune_core::scan::cache_header_valid(&target.root, relative, true)
                && inodune_core::scan::matching_source(&target.root, relative)
        }
        CacheKind::Thumbnails => {
            inodune_core::scan::cache_header_valid(&target.root, relative, false)
        }
        CacheKind::CargoMetadata => {
            inodune_core::cache_rules::validate_leaf(&target.root, relative).as_ref()
                == Some(expected)
        }
        CacheKind::TerraformProvider | CacheKind::Rebuildable => true, // Política e manifesto revalidados antes do staging.
        CacheKind::PytestNodeIds => {
            inodune_core::cache_rules::validate_pytest_nodeids_leaf(&target.root, relative).as_ref()
                == Some(expected)
        }
    };
    if let Err(refusal) = inodune_core::access::stat_write_allowed(
        inodune_core::access::WriteRole::CacheLeaf,
        fp.uid,
        leaf_gid,
        fp.mode,
        &inodune_core::access::SystemGroups,
    ) {
        return fail(&format!(
            "Formato, fonte ou permissões do cache mudaram; preservado. {}",
            inodune_core::access::describe_write_refusal(refusal)
        ));
    }
    if !valid {
        return fail("Formato, fonte ou permissões do cache mudaram; preservado.");
    }
    if let Err(e) = validate_witnesses(target) {
        return fail(&e);
    }
    hook(Phase::Pinned, &original);
    if let Err(e) =
        parent_stable(target, &parent, parent_rel).and_then(|_| stage.revalidate(&target.root))
    {
        return fail(&e);
    }
    let captured = match storage::random_name("leaf-") {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };
    if let Err(e)=journal.record(&serde_json::json!({"phase":"capture_intent","original":original,"recovery_path":stage.path.join(&captured),"fingerprint":expected})) {return fail(&format!("Journal não sincronizado: {e}"));}
    if let Err(e) = fs::renameat_with(&parent, name, &stage.fd, &captured, RenameFlags::NOREPLACE) {
        return fail(&format!("Captura recusada: {e}"));
    }
    hook(Phase::Captured, &original);
    let verification: Result<Fingerprint, String> = (|| {
        parent_stable(target, &parent, parent_rel)?;
        stage.revalidate(&target.root)?;
        validate_witnesses(target)?;
        if python && !inodune_core::scan::matching_source(&target.root, relative) {
            return Err("A fonte Python desapareceu; bytecode preservado.".into());
        }
        fs::fsync(&parent)
            .and_then(|_| fs::fsync(&stage.fd))
            .map_err(|e| format!("Captura não sincronizada: {e}"))?;
        let after = leaf_fingerprint(&stage.fd, std::ffi::OsStr::new(&captured))?;
        let held = Fingerprint::from_stat(&fs::fstat(&pinned).map_err(|e| e.to_string())?);
        if !same_after_rename(expected, &after) || after != held {
            return Err("Arquivo mudou durante captura; preservado.".into());
        }
        Ok(after)
    })();
    let after = match verification {
        Ok(fp) => fp,
        Err(e) => {
            return restore_or_preserve(
                target, &parent, parent_rel, name, stage, &captured, &original, &e,
            )
        }
    };
    let result = (|| -> Result<Option<PathBuf>, String> {
        stage.revalidate(&target.root)?;
        parent_stable(target, &parent, parent_rel)?;
        validate_witnesses(target)?;
        if leaf_fingerprint(&stage.fd, std::ffi::OsStr::new(&captured))? != after {
            return Err("Arquivo alterado após a captura.".into());
        }
        match action {
            Action::Permanent => {
                fs::unlinkat(&stage.fd, &captured, AtFlags::empty()).map_err(|e| e.to_string())?;
                Ok(None)
            }
            Action::Trash => {
                let trash = trash.ok_or("Lixeira indisponível")?;
                trash.revalidate(expected.dev)?;
                let reservation = trash.reserve(&original)?;
                if let Err(e) = fs::renameat_with(
                    &stage.fd,
                    &captured,
                    &trash.files,
                    &reservation,
                    RenameFlags::NOREPLACE,
                ) {
                    let cleanup = trash
                        .abandon(&reservation)
                        .err()
                        .map(|e| format!("; metadata órfã: {e}"))
                        .unwrap_or_default();
                    return Err(format!("Falha ao mover à Lixeira: {e}{cleanup}"));
                }
                Ok(Some(trash.destination(&reservation)))
            }
        }
    })();
    match result {
        Ok(destination) => {
            let sync = fs::fsync(&stage.fd)
                .and_then(|_| fs::fsync(&parent))
                .map_err(|e| e.to_string())
                .and_then(|_| {
                    if let Some(t) = trash {
                        t.sync()
                    } else {
                        Ok(())
                    }
                });
            let mut result = outcome(
                &original,
                if action == Action::Trash {
                    FileStatus::Trashed
                } else {
                    FileStatus::Removed
                },
                expected.size,
                if action == Action::Trash {
                    "Movido à Lixeira; ainda ocupa espaço."
                } else {
                    "Removido; verifique variação observada do volume."
                },
                destination.map(|p| p.to_string_lossy().into()),
            );
            if let Err(e) = sync {
                result.durability_confirmed = false;
                result.message.push_str(&format!(
                    " Sincronização falhou; durabilidade após queda não confirmada: {e}"
                ));
            }
            result
        }
        Err(e) => restore_or_preserve(
            target, &parent, parent_rel, name, stage, &captured, &original, &e,
        ),
    }
}
#[allow(clippy::too_many_arguments)]
fn restore_or_preserve(
    target: &ScannedTarget,
    parent: &OwnedFd,
    parent_rel: &Path,
    name: &std::ffi::OsStr,
    stage: &Stage,
    captured: &str,
    original: &Path,
    message: &str,
) -> FileOutcome {
    if parent_stable(target, parent, parent_rel).is_ok()
        && fs::renameat_with(&stage.fd, captured, parent, name, RenameFlags::NOREPLACE).is_ok()
    {
        outcome(
            original,
            FileStatus::SkippedChanged,
            0,
            &format!("{message} Arquivo preservado na origem."),
            None,
        )
    } else {
        outcome(
            original,
            FileStatus::Quarantined,
            0,
            &format!(
                "{message} Não foi possível restaurar sem sobrescrever; recuperação necessária."
            ),
            Some(stage.path.join(captured).to_string_lossy().into()),
        )
    }
}
fn outcome(
    path: &Path,
    status: FileStatus,
    logical_bytes: u64,
    message: &str,
    preserved_at: Option<String>,
) -> FileOutcome {
    FileOutcome {
        display_path: path.to_string_lossy().into(),
        status,
        logical_bytes,
        message: message.into(),
        preserved_at,
        durability_confirmed: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::PlanSession;
    use inodune_core::scan::{scan, ScanLimits, ScopeKind};
    use std::{
        fs,
        os::unix::{
            fs::{symlink, PermissionsExt},
            io::AsRawFd,
        },
        sync::Arc,
    };
    struct Fixture {
        dir: tempfile::TempDir,
        snapshot: Arc<inodune_core::scan::ScanSnapshot>,
        path: PathBuf,
    }
    impl Fixture {
        fn new(count: usize) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("projects");
            fs::create_dir_all(root.join("app/__pycache__")).unwrap();
            for n in 0..count {
                fs::write(root.join(format!("app/code{n}.py")), b"pass").unwrap();
                let mut pyc = vec![0u8; 64];
                pyc[0..4].copy_from_slice(&[0xa7, 0x0d, 13, 10]);
                pyc[16] = b'c';
                fs::write(
                    root.join(format!("app/__pycache__/code{n}.cpython-311.pyc")),
                    pyc,
                )
                .unwrap();
            }
            let approved = ApprovedRoot::open(&root, ScopeKind::Projects).unwrap();
            let snapshot = Arc::new(
                scan(
                    &[approved],
                    &ScanControl::default(),
                    &ScanLimits::default(),
                    |_| {},
                )
                .unwrap(),
            );
            assert!(snapshot.complete);
            assert_eq!(snapshot.candidates.len(), 1);
            assert_eq!(snapshot.candidates[0].risk, inodune_core::Risk::Low);
            let path = root.join("app/__pycache__/code0.cpython-311.pyc");
            Self {
                dir,
                snapshot,
                path,
            }
        }
        fn plan(&self, action: Action) -> AuthorizedPlan {
            let p = PlanSession::prepare(
                self.snapshot.clone(),
                &[self.snapshot.candidates[0].id.clone()],
                action,
            )
            .unwrap();
            let hash = p.view().hash.clone();
            p.consume(&hash).unwrap()
        }
        fn options(&self) -> ExecutionOptions {
            ExecutionOptions {
                trash_directory: Some(self.dir.path().join("data/Trash")),
                history_directory: Some(self.dir.path().join("state/reports")),
            }
        }
        fn status(report: &ExecutionReport) -> FileStatus {
            report.items[0].files[0].status.clone()
        }
    }
    #[test]
    fn permanent_removes_only_manifest_leaves_and_preserves_sources_and_parent() {
        let f = Fixture::new(1);
        let file = fs::File::open(&f.path).unwrap();
        let report = execute(
            f.plan(Action::Permanent),
            &f.options(),
            &ScanControl::default(),
            |_| {},
        );
        assert!(report.complete, "{report:?}");
        assert!(!f.path.exists());
        assert!(f.path.parent().unwrap().exists());
        assert!(f
            .path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("code0.py")
            .exists());
        // An open handle still references data; logical bytes are not physical gain.
        assert!(file.as_raw_fd() >= 0);
        assert_eq!(file.metadata().unwrap().len(), 64);
        assert_eq!(report.logical_bytes_processed, 64);
        assert!(report.attributed_freed_bytes.is_none());
        assert!(report.history_path.is_some());
    }
    #[test]
    fn trash_writes_info_before_move_and_does_not_claim_reclaimed_space() {
        let f = Fixture::new(1);
        let report = execute(
            f.plan(Action::Trash),
            &f.options(),
            &ScanControl::default(),
            |_| {},
        );
        assert!(report.complete, "{report:?}");
        assert_eq!(Fixture::status(&report), FileStatus::Trashed);
        assert!(!f.path.exists());
        assert!(report.attributed_freed_bytes.is_none());
        let trash = f.options().trash_directory.unwrap();
        let files = fs::read_dir(trash.join("files"))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(fs::read(files[0].path()).unwrap().len(), 64);
        let info = fs::read_to_string(trash.join("info").join(format!(
            "{}.trashinfo",
            files[0].file_name().to_str().unwrap()
        )))
        .unwrap();
        assert!(info.starts_with("[Trash Info]\nPath=/"));
        assert!(info.contains("DeletionDate="));
        assert!(info.contains("code0.cpython-311.pyc"));
    }
    #[test]
    fn changed_file_and_new_hardlink_are_skipped() {
        for hardlink in [false, true] {
            let f = Fixture::new(1);
            let plan = f.plan(Action::Permanent);
            if hardlink {
                fs::hard_link(&f.path, f.dir.path().join("link")).unwrap();
            } else {
                fs::write(&f.path, b"new personal data").unwrap();
            }
            let report = execute(plan, &f.options(), &ScanControl::default(), |_| {});
            assert!(!report.complete);
            assert_eq!(Fixture::status(&report), FileStatus::SkippedChanged);
            assert!(f.path.exists());
        }
    }
    #[test]
    fn directory_symlink_swap_before_execution_never_touches_external_sentinel() {
        let f = Fixture::new(1);
        let external = tempfile::tempdir().unwrap();
        fs::write(
            external.path().join(f.path.file_name().unwrap()),
            b"sentinel",
        )
        .unwrap();
        let plan = f.plan(Action::Permanent);
        let parent = f.path.parent().unwrap();
        fs::rename(parent, parent.with_extension("saved")).unwrap();
        symlink(external.path(), parent).unwrap();
        let report = execute(plan, &f.options(), &ScanControl::default(), |_| {});
        assert!(!report.complete);
        assert_eq!(
            fs::read(external.path().join(f.path.file_name().unwrap())).unwrap(),
            b"sentinel"
        );
    }
    #[test]
    fn source_name_swap_after_pin_is_restored_without_deleting_either_inode() {
        let f = Fixture::new(1);
        let saved = f.path.with_extension("saved");
        let mut swapped = false;
        let report = execute_inner(
            f.plan(Action::Permanent),
            &f.options(),
            &ScanControl::default(),
            |_| {},
            |phase, path| {
                if matches!(phase, Phase::Pinned) && !swapped {
                    fs::rename(path, &saved).unwrap();
                    fs::write(path, b"replacement personal data").unwrap();
                    swapped = true;
                }
            },
        );
        assert!(!report.complete);
        assert_eq!(fs::read(&f.path).unwrap(), b"replacement personal data");
        assert_eq!(fs::metadata(saved).unwrap().len(), 64);
    }
    #[test]
    fn leaf_symlink_swap_after_pin_never_follows_or_removes_external_file() {
        let f = Fixture::new(1);
        let external = tempfile::tempdir().unwrap();
        let sentinel = external.path().join("private");
        fs::write(&sentinel, b"keep private").unwrap();
        let saved = f.path.with_extension("saved");
        let mut changed = false;
        let report = execute_inner(
            f.plan(Action::Permanent),
            &f.options(),
            &ScanControl::default(),
            |_| {},
            |phase, path| {
                if matches!(phase, Phase::Pinned) && !changed {
                    fs::rename(path, &saved).unwrap();
                    symlink(&sentinel, path).unwrap();
                    changed = true;
                }
            },
        );
        assert!(!report.complete);
        assert_eq!(fs::read(sentinel).unwrap(), b"keep private");
        assert_eq!(fs::metadata(saved).unwrap().len(), 64);
        assert!(fs::symlink_metadata(&f.path)
            .unwrap()
            .file_type()
            .is_symlink());
    }
    #[test]
    fn parent_swap_after_capture_quarantines_and_never_follows_link() {
        let f = Fixture::new(1);
        let external = tempfile::tempdir().unwrap();
        fs::write(
            external.path().join(f.path.file_name().unwrap()),
            b"sentinel",
        )
        .unwrap();
        let mut swapped = false;
        let report = execute_inner(
            f.plan(Action::Permanent),
            &f.options(),
            &ScanControl::default(),
            |_| {},
            |phase, path| {
                if matches!(phase, Phase::Captured) && !swapped {
                    let parent = path.parent().unwrap();
                    fs::rename(parent, parent.with_extension("saved")).unwrap();
                    symlink(external.path(), parent).unwrap();
                    swapped = true;
                }
            },
        );
        assert!(!report.complete);
        assert_eq!(Fixture::status(&report), FileStatus::Quarantined);
        let preserved = report.items[0].files[0].preserved_at.as_ref().unwrap();
        assert_eq!(fs::metadata(preserved).unwrap().len(), 64);
        assert_eq!(
            fs::read(external.path().join(f.path.file_name().unwrap())).unwrap(),
            b"sentinel"
        );
    }
    #[test]
    fn concurrent_writer_after_capture_is_preserved_and_new_source_is_not_overwritten() {
        let f = Fixture::new(1);
        let mut held = fs::OpenOptions::new().write(true).open(&f.path).unwrap();
        let mut modified = false;
        let report = execute_inner(
            f.plan(Action::Permanent),
            &f.options(),
            &ScanControl::default(),
            |_| {},
            |phase, path| {
                if matches!(phase, Phase::Captured) && !modified {
                    use std::io::Write;
                    held.write_all(b"changed").unwrap();
                    fs::write(path, b"new cache writer").unwrap();
                    modified = true;
                }
            },
        );
        assert_eq!(Fixture::status(&report), FileStatus::Quarantined);
        assert_eq!(fs::read(&f.path).unwrap(), b"new cache writer");
        assert!(report.items[0].files[0]
            .preserved_at
            .as_ref()
            .is_some_and(|p| Path::new(p).exists()));
    }
    #[test]
    fn cancellation_keeps_completed_result_and_preserves_remaining_files() {
        let f = Fixture::new(3);
        let control = ScanControl::default();
        let report = execute(f.plan(Action::Permanent), &f.options(), &control, |p| {
            if p.completed_files == 1 {
                control.cancel();
            }
        });
        assert!(!report.complete);
        assert!(report.cancelled);
        let statuses = &report.items[0].files;
        assert_eq!(
            statuses
                .iter()
                .filter(|r| r.status == FileStatus::Removed)
                .count(),
            1
        );
        assert_eq!(
            statuses
                .iter()
                .filter(|r| r.status == FileStatus::Cancelled)
                .count(),
            2
        );
        assert_eq!(fs::read_dir(f.path.parent().unwrap()).unwrap().count(), 2);
    }
    #[test]
    fn preflight_reports_an_unusable_registry_or_trash_before_execution() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let reports = dir.path().join("reports");
        fs::create_dir(&reports).unwrap();
        fs::set_permissions(&reports, fs::Permissions::from_mode(0o777)).unwrap();
        let trash = dir.path().join("Trash");
        let options = ExecutionOptions {
            trash_directory: Some(trash.clone()),
            history_directory: Some(reports.clone()),
        };
        let err = preflight(&options, Action::Trash).unwrap_err();
        assert!(
            err.contains("registro da operação não está utilizável"),
            "{err}"
        );
        assert!(!trash.exists());
        fs::set_permissions(&reports, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(&trash).unwrap();
        fs::set_permissions(&trash, fs::Permissions::from_mode(0o777)).unwrap();
        let err = preflight(&options, Action::Trash).unwrap_err();
        assert!(err.contains("Lixeira não está utilizável"), "{err}");
        assert_eq!(
            fs::metadata(&trash).unwrap().permissions().mode() & 0o777,
            0o777
        );
        fs::remove_dir(&trash).unwrap();
        assert!(preflight(&options, Action::Permanent).is_ok());
        let created = dir.path().join("fresh-trash");
        let options = ExecutionOptions {
            trash_directory: Some(created.clone()),
            history_directory: Some(reports),
        };
        assert!(preflight(&options, Action::Trash).is_ok());
        assert_eq!(
            fs::metadata(&created).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
    #[test]
    fn unsafe_or_unavailable_trash_has_no_permanent_fallback() {
        let f = Fixture::new(1);
        let options = f.options();
        let trash = options.trash_directory.as_ref().unwrap();
        fs::create_dir_all(trash).unwrap();
        fs::set_permissions(trash, fs::Permissions::from_mode(0o777)).unwrap();
        let report = execute(
            f.plan(Action::Trash),
            &options,
            &ScanControl::default(),
            |_| {},
        );
        assert!(!report.complete);
        assert!(f.path.exists());
        assert_eq!(Fixture::status(&report), FileStatus::SkippedChanged);
    }
    #[test]
    fn new_unknown_file_during_cleanup_is_preserved_and_report_is_partial() {
        let f = Fixture::new(1);
        let new = f.path.parent().unwrap().join("personal.txt");
        let report = execute(
            f.plan(Action::Permanent),
            &f.options(),
            &ScanControl::default(),
            |p| {
                if p.completed_files == 1 {
                    fs::write(&new, b"personal").unwrap();
                }
            },
        );
        assert!(!report.complete);
        assert_eq!(fs::read(new).unwrap(), b"personal");
        assert_eq!(report.logical_bytes_processed, 64);
    }
    #[test]
    fn source_removed_after_scan_is_not_mistaken_for_regenerable_bytecode() {
        let f = Fixture::new(1);
        let plan = f.plan(Action::Permanent);
        fs::remove_file(f.path.parent().unwrap().parent().unwrap().join("code0.py")).unwrap();
        let report = execute(plan, &f.options(), &ScanControl::default(), |_| {});
        assert!(!report.complete);
        assert!(f.path.exists());
    }
    #[test]
    fn denied_rename_after_pin_is_reported_without_removal() {
        let f = Fixture::new(1);
        let parent = f.path.parent().unwrap();
        let report = execute_inner(
            f.plan(Action::Permanent),
            &f.options(),
            &ScanControl::default(),
            |_| {},
            |phase, _| {
                if matches!(phase, Phase::Pinned) {
                    fs::set_permissions(parent, fs::Permissions::from_mode(0o500)).unwrap();
                }
            },
        );
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(!report.complete);
        assert!(f.path.exists());
    }
    #[test]
    #[ignore = "run only in a private user+mount namespace; binds task-created fixtures"]
    fn bind_mount_replacement_is_not_traversed() {
        let f = Fixture::new(1);
        let outside = tempfile::tempdir().unwrap();
        let sentinel = outside.path().join("sentinel");
        fs::write(&sentinel, b"external fixture").unwrap();
        let plan = f.plan(Action::Permanent);
        assert!(std::process::Command::new("mount")
            .arg("--bind")
            .arg(&sentinel)
            .arg(&f.path)
            .status()
            .unwrap()
            .success());
        let report = execute(plan, &f.options(), &ScanControl::default(), |_| {});
        assert!(std::process::Command::new("umount")
            .arg(&f.path)
            .status()
            .unwrap()
            .success());
        assert!(!report.complete);
        assert_eq!(fs::read(sentinel).unwrap(), b"external fixture");
        assert!(f.path.exists());
    }
    #[test]
    fn history_rejects_symlinks_without_modifying_the_destination() {
        let f = Fixture::new(1);
        let external = tempfile::tempdir().unwrap();
        let mut options = f.options();
        let state = f.dir.path().join("state");
        symlink(external.path(), &state).unwrap();
        options.history_directory = Some(state.join("reports"));
        let report = execute(
            f.plan(Action::Permanent),
            &options,
            &ScanControl::default(),
            |_| {},
        );
        assert!(!report.complete);
        assert!(report.warnings.iter().any(|w| w.contains("Histórico")));
        assert_eq!(fs::read_dir(external.path()).unwrap().count(), 0);
    }
}
