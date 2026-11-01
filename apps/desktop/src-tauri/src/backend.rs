//! Native authority: roots can only be approved through a Rust-owned picker.
use inodune_core::{
    scan::{
        self, ApprovedRoot, ScanControl, ScanIssue, ScanLimits, ScanProgress, ScanSnapshot,
        ScanStats, ScopeKind, VolumeInfo,
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
}
struct RootEntry {
    info: ScopeInfo,
    root: ApprovedRoot,
}
#[derive(Default)]
pub struct Backend {
    roots: Mutex<HashMap<String, RootEntry>>,
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
) -> Result<Option<ScopeInfo>, String> {
    if state.cleanup.active()? {
        return Err("Aguarde a limpeza em andamento.".into());
    }
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Inodune · Escolha uma pasta para analisar (somente leitura)")
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
    if state.cleanup.active()? {
        return Err("Aguarde a limpeza em andamento.".into());
    }
    let root = ApprovedRoot::open(&path, kind)?;
    let mut roots = state.roots.lock().map_err(|_| "Estado indisponível")?;
    if let Some(existing) = roots
        .values()
        .find(|r| r.info.display_path == root.display_path())
    {
        return Ok(Some(existing.info.clone()));
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
    if state.cleanup.active()? {
        return Err("Aguarde a limpeza em andamento.".into());
    }
    state.cleanup.invalidate()?;
    state
        .roots
        .lock()
        .map_err(|_| "Estado indisponível")?
        .remove(&scope_id);
    Ok(())
}
#[tauri::command]
pub fn begin_scan(state: State<'_, Backend>, scope_ids: Vec<String>) -> Result<String, String> {
    if state.cleanup.active()? {
        return Err("Aguarde a limpeza em andamento.".into());
    }
    state.cleanup.invalidate()?;
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
            if !roots.values().any(|r| r.root.path() == target.root.path()) {
                return Err("A aprovação da pasta foi revogada.".into());
            }
            target.root.revalidate()?;
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
