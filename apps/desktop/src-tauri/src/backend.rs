//! Native authority: roots can only be approved through a Rust-owned picker.
use inodune_core::{
    scan::{
        self, is_protected_agent_path, ApprovedRoot, ScanControl, ScanIssue, ScanLimits,
        ScanProgress, ScanSnapshot, ScanStats, ScopeKind, VolumeInfo,
    },
    Candidate,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Clone, Serialize)]
pub struct ScopeInfo {
    id: String,
    label: String,
    display_path: String,
    kind: ScopeKind,
    exclusions: Vec<String>,
}
struct RootEntry {
    info: ScopeInfo,
    root: ApprovedRoot,
}
#[derive(Default)]
pub struct Backend {
    roots: Mutex<HashMap<String, RootEntry>>,
    pub operation_gate: Mutex<()>,
    pub cleanup: crate::cleanup::CleanupState,
    job: Mutex<Option<Arc<Mutex<Job>>>>,
    sequence: AtomicU64,
    #[cfg(feature = "fixture-tests")]
    fixture_guard: Option<tempfile::TempDir>,
}
struct Job {
    id: String,
    control: Arc<ScanControl>,
    progress: ScanProgress,
    snapshot: Option<Arc<ScanSnapshot>>,
    error: Option<String>,
}
#[derive(Serialize)]
pub struct ScanResult {
    scan_id: String,
    candidates: Vec<Candidate>,
    issues: Vec<ScanIssue>,
    stats: ScanStats,
    complete: bool,
    cancelled: bool,
    volumes: Vec<VolumeInfo>,
}
#[derive(Serialize)]
pub struct ScanPoll {
    job_id: String,
    status: &'static str,
    progress: ScanProgress,
    result: Option<ScanResult>,
    error: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlAction {
    Pause,
    Resume,
    Cancel,
}

#[tauri::command]
pub async fn choose_scope(
    app: AppHandle,
    state: State<'_, Backend>,
    kind: ScopeKind,
    locale: Option<crate::cleanup::PromptLocale>,
) -> Result<Option<ScopeInfo>, String> {
    if state.cleanup.active()? {
        return Err("Aguarde a limpeza em andamento.".into());
    }
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title(
                match (
                    kind,
                    matches!(locale, Some(crate::cleanup::PromptLocale::EnUs)),
                ) {
                    (ScopeKind::Protected, true) => {
                        "Inodune · Choose protected inventory content (read only)"
                    }
                    (ScopeKind::Thumbnails, true) => {
                        "Inodune · Choose the XDG thumbnails folder (read only)"
                    }
                    (_, true) => "Inodune · Choose a project folder to analyze (read only)",
                    (ScopeKind::Protected, false) => {
                        "Inodune · Escolha conteúdo para inventário protegido (somente leitura)"
                    }
                    (ScopeKind::Thumbnails, false) => {
                        "Inodune · Escolha a pasta XDG thumbnails (somente leitura)"
                    }
                    _ => "Inodune · Escolha uma pasta de projetos para analisar (somente leitura)",
                },
            )
            .blocking_pick_folder()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    if kind == ScopeKind::Thumbnails && path.file_name().is_none_or(|n| n != "thumbnails") {
        return Err("Para miniaturas, selecione a pasta XDG chamada thumbnails. Nenhum outro diretório de imagens é tratado como cache.".into());
    }
    let root = ApprovedRoot::open(&path, kind)?;
    let _gate = state
        .operation_gate
        .lock()
        .map_err(|_| "Estado indisponível")?;
    let mut job_slot = state.job.lock().map_err(|_| "Estado indisponível")?;
    if job_slot.as_ref().is_some_and(|job| {
        job.lock()
            .map(|j| j.snapshot.is_none() && j.error.is_none())
            .unwrap_or(true)
    }) {
        return Err("Aguarde a análise em andamento.".into());
    }
    if state.cleanup.active()? {
        return Err("Aguarde a limpeza em andamento.".into());
    }
    let mut roots = state.roots.lock().map_err(|_| "Estado indisponível")?;
    if let Some((existing_id, existing)) = roots
        .iter()
        .find(|(_, r)| r.info.display_path == root.display_path())
    {
        if existing.info.kind == kind && existing.root.revalidate().is_ok() {
            return Ok(Some(existing.info.clone()));
        }
        let id = existing_id.clone();
        let info = ScopeInfo {
            id: id.clone(),
            label: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
            display_path: root.display_path(),
            kind,
            exclusions: Vec::new(),
        };
        roots.insert(
            id,
            RootEntry {
                info: info.clone(),
                root,
            },
        );
        state.cleanup.invalidate()?;
        *job_slot = None;
        return Ok(Some(info));
    }
    if roots.len() >= 32 {
        return Err("Limite de 32 escopos aprovados; remova um antes de adicionar outro.".into());
    }
    let id = format!("scope-{}", state.sequence.fetch_add(1, Ordering::Relaxed));
    let info = ScopeInfo {
        id: id.clone(),
        label: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        display_path: root.display_path(),
        kind,
        exclusions: Vec::new(),
    };
    roots.insert(
        id,
        RootEntry {
            info: info.clone(),
            root,
        },
    );
    Ok(Some(info))
}
#[tauri::command]
pub fn list_scopes(state: State<'_, Backend>) -> Result<Vec<ScopeInfo>, String> {
    let mut scopes = state
        .roots
        .lock()
        .map_err(|_| "Estado indisponível")?
        .values()
        .map(|r| r.info.clone())
        .collect::<Vec<_>>();
    scopes.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(scopes)
}
#[tauri::command]
pub fn forget_scope(state: State<'_, Backend>, scope_id: String) -> Result<(), String> {
    let _gate = state
        .operation_gate
        .lock()
        .map_err(|_| "Estado indisponível")?;
    let mut job_slot = state.job.lock().map_err(|_| "Estado indisponível")?;
    if job_slot.as_ref().is_some_and(|job| {
        job.lock()
            .map(|j| j.snapshot.is_none() && j.error.is_none())
            .unwrap_or(true)
    }) {
        return Err("Aguarde a análise em andamento.".into());
    }
    if state.cleanup.active()? {
        return Err("Aguarde a limpeza em andamento.".into());
    }
    if !state
        .roots
        .lock()
        .map_err(|_| "Estado indisponível")?
        .contains_key(&scope_id)
    {
        return Err("Escopo não aprovado".into());
    }
    state.cleanup.invalidate()?;
    *job_slot = None;
    state
        .roots
        .lock()
        .map_err(|_| "Estado indisponível")?
        .remove(&scope_id);
    Ok(())
}
#[tauri::command]
pub fn set_scope_exclusions(
    state: State<'_, Backend>,
    scope_id: String,
    relative_paths: Vec<String>,
) -> Result<ScopeInfo, String> {
    let _gate = state
        .operation_gate
        .lock()
        .map_err(|_| "Estado indisponível")?;
    let mut job_slot = state.job.lock().map_err(|_| "Estado indisponível")?;
    if job_slot.as_ref().is_some_and(|job| {
        job.lock()
            .map(|j| j.snapshot.is_none() && j.error.is_none())
            .unwrap_or(true)
    }) {
        return Err("Aguarde a análise em andamento.".into());
    }
    if state.cleanup.active()? {
        return Err("Aguarde a limpeza em andamento.".into());
    }
    let exclusions = relative_paths
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect();
    let mut roots = state.roots.lock().map_err(|_| "Estado indisponível")?;
    let entry = roots.get_mut(&scope_id).ok_or("Escopo não aprovado")?;
    let root = entry.root.with_exclusions(exclusions)?;
    let values = root
        .exclusions()
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    entry.info.exclusions = values;
    entry.root = root;
    state.cleanup.invalidate()?;
    *job_slot = None;
    Ok(entry.info.clone())
}
#[tauri::command]
pub fn begin_scan(state: State<'_, Backend>, scope_ids: Vec<String>) -> Result<String, String> {
    let _gate = state
        .operation_gate
        .lock()
        .map_err(|_| "Estado indisponível")?;
    if state.cleanup.active()? {
        return Err("Aguarde a limpeza em andamento.".into());
    }
    if scope_ids.is_empty() || scope_ids.len() > 32 {
        return Err("Escolha entre 1 e 32 escopos aprovados.".into());
    }
    let mut job_slot = state.job.lock().map_err(|_| "Estado indisponível")?;
    if let Some(job) = job_slot.as_ref() {
        let j = job.lock().map_err(|_| "Estado indisponível")?;
        if j.snapshot.is_none() && j.error.is_none() {
            return Err("Já existe uma análise em andamento.".into());
        }
    }
    let roots = {
        let entries = state.roots.lock().map_err(|_| "Estado indisponível")?;
        scope_ids
            .iter()
            .map(|id| {
                entries
                    .get(id)
                    .map(|r| r.root.clone())
                    .ok_or("Escopo não aprovado".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    state.cleanup.invalidate()?;
    let id = format!("job-{}", state.sequence.fetch_add(1, Ordering::Relaxed));
    let control = Arc::new(ScanControl::default());
    let job = Arc::new(Mutex::new(Job {
        id: id.clone(),
        control: control.clone(),
        progress: ScanProgress {
            visited: 0,
            candidates: 0,
            stage: "Iniciando análise somente leitura".into(),
        },
        snapshot: None,
        error: None,
    }));
    *job_slot = Some(job.clone());
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            scan::scan(&roots, &control, &ScanLimits::default(), |p| {
                if let Ok(mut j) = job.lock() {
                    j.progress = p;
                }
            })
        }));
        if let Ok(mut j) = job.lock() {
            match result {
                Ok(Ok(snapshot)) => j.snapshot = Some(Arc::new(snapshot)),
                Ok(Err(e)) => j.error = Some(e),
                Err(_) => j.error = Some(
                    "A análise foi interrompida por uma falha interna. Nenhuma limpeza executada."
                        .into(),
                ),
            }
        }
    });
    Ok(id)
}
#[tauri::command]
pub async fn poll_scan(state: State<'_, Backend>, job_id: String) -> Result<ScanPoll, String> {
    let slot = state.job.lock().map_err(|_| "Estado indisponível")?;
    let job = slot.as_ref().ok_or("Análise inexistente")?;
    let j = job.lock().map_err(|_| "Estado indisponível")?;
    if j.id != job_id {
        return Err("Análise expirada ou desconhecida".into());
    }
    Ok(ScanPoll {
        job_id: j.id.clone(),
        status: if j.error.is_some() {
            "error"
        } else if j.snapshot.is_some() {
            "done"
        } else {
            "running"
        },
        progress: j.progress.clone(),
        error: j.error.clone(),
        result: j.snapshot.as_ref().map(|s| ScanResult {
            scan_id: s.id.clone(),
            candidates: s.candidates.clone(),
            issues: s.issues.clone(),
            stats: s.stats.clone(),
            complete: s.complete,
            cancelled: s.cancelled,
            volumes: s.volumes.clone(),
        }),
    })
}
#[tauri::command]
pub fn control_scan(
    state: State<'_, Backend>,
    job_id: String,
    action: ControlAction,
) -> Result<(), String> {
    let slot = state.job.lock().map_err(|_| "Estado indisponível")?;
    let job = slot.as_ref().ok_or("Análise inexistente")?;
    let j = job.lock().map_err(|_| "Estado indisponível")?;
    if j.id != job_id {
        return Err("Análise expirada ou desconhecida".into());
    }
    match action {
        ControlAction::Pause => j.control.pause(),
        ControlAction::Resume => j.control.resume(),
        ControlAction::Cancel => j.control.cancel(),
    };
    Ok(())
}

#[cfg(feature = "fixture-tests")]
impl Backend {
    pub fn fixtures() -> Result<Self, String> {
        use std::fs;
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let projects = dir.path().join("projects");
        let thumbnails = dir.path().join("cache/thumbnails");
        // fixture-tests runs before Tauri/runtime threads are started. This modifies
        // only this test process, never the user's shell or persistent configuration.
        for (var, relative) in [
            ("XDG_CACHE_HOME", "cache"),
            ("XDG_DATA_HOME", "data"),
            ("XDG_STATE_HOME", "state"),
            ("XDG_CONFIG_HOME", "config"),
        ] {
            fs::create_dir_all(dir.path().join(relative)).map_err(|e| e.to_string())?;
            std::env::set_var(var, dir.path().join(relative));
        }
        for rel in [
            "projects/orbit/target",
            "projects/infra/.terraform/providers",
            "projects/infra/.terraform/modules",
            "projects/.codex/sessions",
            "cache/thumbnails/normal",
        ] {
            fs::create_dir_all(dir.path().join(rel)).map_err(|e| e.to_string())?;
        }
        for (rel, bytes) in [
            (
                "projects/orbit/Cargo.toml",
                &b"[package]\nname=\"fixture\"\nversion=\"0.1.0\""[..],
            ),
            ("projects/orbit/target/binary", &b"build fixture"[..]),
            ("projects/infra/main.tf", &b"terraform {}"[..]),
            ("projects/infra/terraform.tfstate", &b"fixture state"[..]),
            (
                "projects/.codex/sessions/rollout.jsonl",
                &b"private fixture"[..],
            ),
        ] {
            fs::write(dir.path().join(rel), bytes).map_err(|e| e.to_string())?;
        }
        // A tiny PNG fixture, created only in an application-owned temporary tree.
        let png: &[u8] = &[
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 4, 0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 248, 15,
            0, 1, 5, 1, 1, 39, 24, 227, 102, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
        ];
        fs::write(
            thumbnails.join("normal/0123456789abcdef0123456789abcdef.png"),
            png,
        )
        .map_err(|e| e.to_string())?;
        if std::env::var("INODUNE_FIXTURE_REVIEW").as_deref() == Ok("1") {
            let fingerprint =
                projects.join("orbit/target/debug/.fingerprint/fixture-0123456789abcdef");
            fs::create_dir_all(&fingerprint).map_err(|e| e.to_string())?;
            fs::write(fingerprint.join("lib-fixture"), b"0123456789abcdef")
                .map_err(|e| e.to_string())?;
            fs::write(fingerprint.join("lib-fixture.json"),br#"{"rustc":1,"features":"[]","declared_features":"[]","target":2,"profile":3,"path":4,"deps":[],"local":[],"rustflags":[],"config":5,"compile_kind":0}"#).map_err(|e|e.to_string())?;
            let package = projects.join(
                "infra/.terraform/providers/registry.terraform.io/hashicorp/demo/1.2.3/linux_amd64",
            );
            fs::create_dir_all(&package).map_err(|e| e.to_string())?;
            let mut elf = vec![0u8; 64];
            elf[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
            elf[18..20].copy_from_slice(&62u16.to_le_bytes());
            fs::write(package.join("terraform-provider-demo_v1.2.3"), elf)
                .map_err(|e| e.to_string())?;
            fs::write(package.join("LICENSE.txt"), b"license text\n").map_err(|e| e.to_string())?;
            fs::write(projects.join("infra/.terraform.lock.hcl"),b"provider \"registry.terraform.io/hashicorp/demo\" {\n version = \"1.2.3\"\n hashes = [\"h1:sBa6P0Es63rPk400M08oc+n2m1OBbCco8ayvM+ij82c=\"]\n}\n").map_err(|e|e.to_string())?;
            fs::write(
                projects.join("infra/.terraform/terraform.tfstate"),
                b"private backend fixture",
            )
            .map_err(|e| e.to_string())?;
            fs::write(
                projects.join("infra/.terraform/modules/local-script.sh"),
                b"private module fixture",
            )
            .map_err(|e| e.to_string())?;
        }
        let mut backend = Self::default();
        for (id, path, kind) in [
            ("fixture-projects", projects, ScopeKind::Projects),
            ("fixture-thumbnails", thumbnails, ScopeKind::Thumbnails),
        ] {
            let root = ApprovedRoot::open(&path, kind)?;
            let info = ScopeInfo {
                id: id.into(),
                label: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into(),
                display_path: root.display_path(),
                kind,
                exclusions: Vec::new(),
            };
            backend
                .roots
                .get_mut()
                .map_err(|_| "Estado indisponível")?
                .insert(id.into(), RootEntry { info, root });
        }
        backend.fixture_guard = Some(dir);
        Ok(backend)
    }
}

impl Backend {
    pub fn snapshot(&self, id: &str) -> Result<Arc<ScanSnapshot>, String> {
        let slot = self.job.lock().map_err(|_| "Estado indisponível")?;
        let job = slot.as_ref().ok_or("Análise inexistente")?;
        let j = job.lock().map_err(|_| "Estado indisponível")?;
        let snapshot = j.snapshot.as_ref().ok_or("Aguarde a análise terminar")?;
        if snapshot.id != id {
            return Err("Análise expirada ou desconhecida".into());
        }
        Ok(snapshot.clone())
    }
    pub fn check_approved(&self, snapshot: &ScanSnapshot, ids: &[String]) -> Result<(), String> {
        let roots = self.roots.lock().map_err(|_| "Estado indisponível")?;
        for id in ids {
            let target = snapshot.targets.get(id).ok_or("Item não aprovado")?;
            let current = roots
                .values()
                .find(|r| r.root.path() == target.root.path())
                .ok_or("A aprovação da pasta foi revogada.")?;
            if current.root.kind == ScopeKind::Protected
                || is_protected_agent_path(current.root.path())
                || is_protected_agent_path(&target.relative)
                || target
                    .files
                    .iter()
                    .any(|(path, _)| is_protected_agent_path(path))
                || target
                    .witnesses
                    .iter()
                    .any(|(path, _)| is_protected_agent_path(path))
            {
                return Err("Conteúdo protegido não pode ser autorizado para limpeza.".into());
            }
            if current.root.kind != target.root.kind
                || current.info.display_path != target.root.display_path()
            {
                return Err("A identidade do escopo mudou; analise novamente.".into());
            }
            if current.root.is_excluded(&target.relative)
                || target
                    .files
                    .iter()
                    .any(|(p, _)| current.root.is_excluded(p))
                || target
                    .witnesses
                    .iter()
                    .any(|(p, _)| current.root.is_excluded(p))
            {
                return Err("O item foi excluído da política do escopo; analise novamente.".into());
            }
            current.root.revalidate()?;
        }
        Ok(())
    }
    pub fn fixture_consent(&self, snapshot: &ScanSnapshot, ids: &[String]) -> bool {
        #[cfg(feature = "fixture-tests")]
        if std::env::var("INODUNE_FIXTURE_CONFIRM").as_deref() == Ok("1") {
            return self.fixture_guard.as_ref().is_some_and(|dir| {
                !ids.is_empty()
                    && ids.iter().all(|id| {
                        snapshot
                            .targets
                            .get(id)
                            .is_some_and(|t| t.root.path().starts_with(dir.path()))
                    })
            });
        }
        let _ = (snapshot, ids);
        false
    }
}
