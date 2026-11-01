//! Native authority: backend discovery or an advanced native picker creates root capabilities.
#[path = "protection_store.rs"]
mod protection_store;
use inodune_core::{
    scan::{
        self, is_protected_agent_path, ApprovedRoot, ScanControl, ScanIssue, ScanLimits,
        ScanProgress, ScanSnapshot, ScanStats, ScopeKind, VolumeInfo,
    },
    Candidate,
};
use protection_store::{PersistedExclusion, Policy, ProtectionStore};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use tauri::{AppHandle, Manager, State};
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
pub struct Backend {
    roots: Mutex<HashMap<String, RootEntry>>,
    pub operation_gate: Mutex<()>,
    automatic_exclusions: Mutex<Vec<std::path::PathBuf>>,
    persisted_policy: Mutex<Policy>,
    protection_store: Option<ProtectionStore>,
    protection_error: Mutex<Option<String>>,
    pub cleanup: crate::cleanup::CleanupState,
    job: Mutex<Option<Arc<Mutex<Job>>>>,
    sequence: AtomicU64,
    #[cfg(feature = "fixture-tests")]
    fixture_guard: Option<tempfile::TempDir>,
}

impl Default for Backend {
    fn default() -> Self {
        #[cfg(test)]
        let (protection_store, persisted_policy, protection_error) = (None, Policy::new(), None);
        #[cfg(not(test))]
        let (protection_store, persisted_policy, protection_error) =
            match ProtectionStore::from_env() {
                Ok(store) => match store.load() {
                    Ok(policy) => (Some(store), policy, None),
                    Err(error) => (Some(store), Policy::new(), Some(error)),
                },
                Err(error) => (None, Policy::new(), Some(error)),
            };
        Self {
            roots: Mutex::new(HashMap::new()),
            operation_gate: Mutex::new(()),
            automatic_exclusions: Mutex::new(persisted_policy.revoked_roots.clone()),
            persisted_policy: Mutex::new(persisted_policy),
            protection_store,
            protection_error: Mutex::new(protection_error),
            cleanup: crate::cleanup::CleanupState::default(),
            job: Mutex::new(None),
            sequence: AtomicU64::new(1),
            #[cfg(feature = "fixture-tests")]
            fixture_guard: None,
        }
    }
}
struct Job {
    id: String,
    automatic: bool,
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
    state.ensure_policy_healthy()?;
    let mut roots = state.roots.lock().map_err(|_| "Estado indisponível")?;
    let existing = roots
        .iter()
        .find(|(_, r)| r.info.display_path == root.display_path())
        .map(|(id, entry)| {
            (
                id.clone(),
                entry.info.clone(),
                entry.root.revalidate().is_ok(),
            )
        });
    let mut policy = state.policy_snapshot()?;
    let previous = policy.clone();
    policy.revoked_roots.retain(|path| path != root.path());
    policy.protected_roots.retain(|path| path != root.path());
    if kind == ScopeKind::Protected {
        policy.protected_roots.push(root.path().to_path_buf());
        policy.protected_roots.sort();
        policy.protected_roots.dedup();
    }
    if policy != previous {
        state.save_policy(policy.clone())?;
        *state
            .automatic_exclusions
            .lock()
            .map_err(|_| "Estado indisponível")? = policy.revoked_roots.clone();
    }
    if let Some((existing_id, previous_info, same_kind_and_root)) = existing {
        if previous_info.kind == kind && same_kind_and_root {
            return Ok(Some(previous_info));
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
    let removed_path = state
        .roots
        .lock()
        .map_err(|_| "Estado indisponível")?
        .get(&scope_id)
        .ok_or("Escopo não aprovado")?
        .root
        .path()
        .to_path_buf();
    let mut policy = state.policy_snapshot()?;
    policy.revoked_roots.push(removed_path.clone());
    policy.revoked_roots.sort();
    policy.revoked_roots.dedup();
    policy.protected_roots.retain(|path| path != &removed_path);
    policy.exclusions.retain(|item| item.root != removed_path);
    state.save_policy(policy.clone())?;
    state
        .automatic_exclusions
        .lock()
        .map_err(|_| "Estado indisponível")?
        .clone_from(&policy.revoked_roots);
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
    let mut policy = state.policy_snapshot()?;
    let root_path = entry.root.path().to_path_buf();
    policy.exclusions.retain(|item| item.root != root_path);
    if !root.exclusions().is_empty() {
        policy.exclusions.push(PersistedExclusion {
            root: root_path,
            relative_paths: root.exclusions().to_vec(),
        });
        policy
            .exclusions
            .sort_by(|left, right| left.root.cmp(&right.root));
    }
    state.save_policy(policy)?;
    entry.info.exclusions = values;
    entry.root = root;
    state.cleanup.invalidate()?;
    *job_slot = None;
    Ok(entry.info.clone())
}
#[tauri::command]
pub fn begin_automatic_scan(app: AppHandle, state: State<'_, Backend>) -> Result<String, String> {
    let _gate = state
        .operation_gate
        .lock()
        .map_err(|_| "Estado indisponível")?;
    if state.cleanup.active()? {
        return Err("Aguarde a limpeza em andamento.".into());
    }
    state.ensure_policy_healthy()?;
    let mut slot = state.job.lock().map_err(|_| "Estado indisponível")?;
    if slot.as_ref().is_some_and(|job| {
        job.lock()
            .map(|j| j.snapshot.is_none() && j.error.is_none())
            .unwrap_or(true)
    }) {
        return Err("Já existe uma análise em andamento.".into());
    }
    state.cleanup.invalidate()?;
    let id = format!("job-{}", state.sequence.fetch_add(1, Ordering::Relaxed));
    let control = Arc::new(ScanControl::default());
    let job = Arc::new(Mutex::new(Job {
        id: id.clone(),
        automatic: true,
        control: control.clone(),
        progress: ScanProgress {
            visited: 0,
            candidates: 0,
            stage: "Descobrindo home e unidades locais".into(),
            recent_findings: Vec::new(),
        },
        snapshot: None,
        error: None,
    }));
    *slot = Some(job.clone());
    std::thread::spawn(move || {
        let state = app.state::<Backend>();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let limits = ScanLimits::default();
            #[cfg(feature = "fixture-tests")]
            let mut discovered = if let Some(dir) = state.fixture_guard.as_ref() {
                // This path is generated by this test process, never supplied by JS/environment.
                inodune_core::discovery::discover_from(dir.path(), "", &[], &control, &limits)?
            } else {
                inodune_core::discovery::discover(&control, &limits)?
            };
            #[cfg(not(feature = "fixture-tests"))]
            let mut discovered = inodune_core::discovery::discover(&control, &limits)?;
            if !control.is_cancelled() {
                state.install_discovered_roots(&mut discovered, &control)?;
            }
            inodune_core::discovery::scan_discovered(discovered, &control, &limits, |progress| {
                if let Ok(mut j) = job.lock() {
                    j.progress = progress;
                }
            })
        }));
        if let Ok(mut j) = job.lock() {
            match result {
                Ok(Ok(snapshot)) => j.snapshot = Some(Arc::new(snapshot)),
                Ok(Err(error)) => j.error = Some(error),
                Err(_) => {
                    j.error = Some(
                        "A análise automática foi interrompida. Nenhuma limpeza executada.".into(),
                    )
                }
            }
        };
    });
    Ok(id)
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
    state.ensure_policy_healthy()?;
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
        automatic: false,
        control: control.clone(),
        progress: ScanProgress {
            visited: 0,
            candidates: 0,
            stage: "Iniciando análise somente leitura".into(),
            recent_findings: Vec::new(),
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
    let _gate = state
        .operation_gate
        .lock()
        .map_err(|_| "Estado indisponível")?;
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

impl Backend {
    fn ensure_policy_healthy(&self) -> Result<(), String> {
        if let Some(error) = self
            .protection_error
            .lock()
            .map_err(|_| "Estado indisponível")?
            .as_ref()
        {
            return Err(format!(
                "A análise está bloqueada porque as proteções salvas não puderam ser validadas. Corrija o armazenamento local e tente novamente. {error}"
            ));
        }
        Ok(())
    }

    fn save_policy(&self, policy: Policy) -> Result<(), String> {
        self.ensure_policy_healthy()?;
        if let Some(store) = &self.protection_store {
            if let Err(error) = store.save(&policy) {
                *self
                    .protection_error
                    .lock()
                    .map_err(|_| "Estado indisponível")? = Some(error.clone());
                return Err(error);
            }
        }
        *self
            .persisted_policy
            .lock()
            .map_err(|_| "Estado indisponível")? = policy;
        Ok(())
    }

    fn policy_snapshot(&self) -> Result<Policy, String> {
        self.ensure_policy_healthy()?;
        self.persisted_policy
            .lock()
            .map(|policy| policy.clone())
            .map_err(|_| "Estado indisponível".into())
    }
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
        if std::env::var("INODUNE_FIXTURE_AUTO_PARTIAL").as_deref() == Ok("1") {
            std::os::unix::fs::symlink(&thumbnails, projects.join("fixture-link"))
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
    fn install_discovered_roots(
        &self,
        discovery: &mut inodune_core::discovery::DiscoveryResult,
        control: &ScanControl,
    ) -> Result<(), String> {
        let _gate = self
            .operation_gate
            .lock()
            .map_err(|_| "Estado indisponível")?;
        if control.is_cancelled() {
            discovery.roots.clear();
            discovery.complete = false;
            return Ok(());
        }
        let policy = self.policy_snapshot()?;
        let blocked = self
            .automatic_exclusions
            .lock()
            .map_err(|_| "Estado indisponível")?;
        let mut entries = self.roots.lock().map_err(|_| "Estado indisponível")?;
        let mut next = HashMap::new();
        let mut roots = Vec::new();
        for discovered in &discovery.roots {
            let mut exclusions = discovered.exclusions().to_vec();
            for path in blocked.iter() {
                if let Ok(relative) = path.strip_prefix(discovered.path()) {
                    exclusions.push(if relative.as_os_str().is_empty() {
                        std::path::PathBuf::from(".")
                    } else {
                        relative.to_path_buf()
                    });
                }
            }
            let mut kind = discovered.kind;
            for protected in &policy.protected_roots {
                if let Ok(relative) = protected.strip_prefix(discovered.path()) {
                    if relative.as_os_str().is_empty() {
                        kind = ScopeKind::Protected;
                    } else {
                        exclusions.push(relative.to_path_buf());
                    }
                }
            }
            for saved in &policy.exclusions {
                if let Ok(prefix) = saved.root.strip_prefix(discovered.path()) {
                    exclusions.extend(
                        saved
                            .relative_paths
                            .iter()
                            .map(|relative| prefix.join(relative)),
                    );
                }
            }
            let mut preserved_id = None;
            for (id, old) in entries.iter() {
                if let Ok(prefix) = old.root.path().strip_prefix(discovered.path()) {
                    if prefix.as_os_str().is_empty() {
                        kind = old.root.kind;
                        preserved_id = Some(id.clone());
                    } else if old.root.kind == ScopeKind::Protected {
                        exclusions.push(prefix.to_path_buf());
                    }
                    exclusions.extend(old.root.exclusions().iter().map(|e| prefix.join(e)));
                }
            }
            exclusions.sort();
            exclusions.dedup();
            let mut root = discovered.with_exclusions(exclusions)?;
            root.kind = kind;
            let id = preserved_id.unwrap_or_else(|| {
                format!("scope-{}", self.sequence.fetch_add(1, Ordering::Relaxed))
            });
            let info = ScopeInfo {
                id: id.clone(),
                label: root
                    .path()
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                display_path: root.display_path(),
                kind,
                exclusions: root
                    .exclusions()
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect(),
            };
            roots.push(root.clone());
            next.insert(id, RootEntry { info, root });
        }
        *entries = next;
        discovery.roots = roots;
        Ok(())
    }
    pub fn is_automatic_scan(&self, id: &str) -> Result<bool, String> {
        let slot = self.job.lock().map_err(|_| "Estado indisponível")?;
        let job = slot
            .as_ref()
            .ok_or("Análise inexistente")?
            .lock()
            .map_err(|_| "Estado indisponível")?;
        if job.snapshot.as_ref().is_none_or(|s| s.id != id) {
            return Err("Análise expirada ou desconhecida".into());
        }
        Ok(job.automatic)
    }
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
        self.ensure_policy_healthy()?;
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

#[cfg(all(test, feature = "fixture-tests"))]
mod automatic_policy_tests {
    use super::*;
    impl Backend {
        fn with_policy_store(store: ProtectionStore) -> Result<Self, String> {
            let policy = store.load()?;
            Ok(Self {
                automatic_exclusions: Mutex::new(policy.revoked_roots.clone()),
                persisted_policy: Mutex::new(policy),
                protection_store: Some(store),
                ..Backend::default()
            })
        }
    }

    #[test]
    fn rejected_policy_change_preserves_disk_state_and_blocks_future_scans() {
        let temporary = tempfile::tempdir().unwrap();
        let config = temporary.path().join("config");
        std::fs::create_dir(&config).unwrap();
        let store = ProtectionStore::at(config).unwrap();
        let backend = Backend::with_policy_store(
            ProtectionStore::at(temporary.path().join("config")).unwrap(),
        )
        .unwrap();
        let original = store.load().unwrap();
        let mut invalid = original.clone();
        invalid
            .revoked_roots
            .push(std::path::PathBuf::from("/home/user/../../etc"));

        assert!(backend.save_policy(invalid).is_err());
        assert_eq!(store.load().unwrap(), original);
        assert!(backend.ensure_policy_healthy().is_err());
    }

    #[test]
    fn automatic_discovery_restores_persisted_protections_exclusions_and_revocations() {
        let temporary = tempfile::tempdir().unwrap();
        let home = temporary.path().join("home");
        std::fs::create_dir_all(home.join("private")).unwrap();
        std::fs::create_dir_all(home.join("revoked")).unwrap();
        std::fs::create_dir_all(home.join("project/local-data")).unwrap();
        let config = temporary.path().join("config");
        std::fs::create_dir(&config).unwrap();
        let store = ProtectionStore::at(config.clone()).unwrap();
        let mut policy = Policy::new();
        policy.protected_roots.push(home.join("private"));
        policy.revoked_roots.push(home.join("revoked"));
        policy.exclusions.push(PersistedExclusion {
            root: home.join("project"),
            relative_paths: vec![std::path::PathBuf::from("local-data")],
        });
        store.save(&policy).unwrap();

        let backend = Backend::with_policy_store(store).unwrap();
        let mut discovered = inodune_core::discovery::discover_from(
            &home,
            "",
            &[],
            &ScanControl::default(),
            &ScanLimits::default(),
        )
        .unwrap();
        backend
            .install_discovered_roots(&mut discovered, &ScanControl::default())
            .unwrap();

        let root = &discovered.roots[0];
        assert!(root.is_excluded(std::path::Path::new("private")));
        assert!(root.is_excluded(std::path::Path::new("revoked")));
        assert!(root.is_excluded(std::path::Path::new("project/local-data")));

        let reopened = Backend::with_policy_store(ProtectionStore::at(config).unwrap()).unwrap();
        let mut next_scan = inodune_core::discovery::discover_from(
            &home,
            "",
            &[],
            &ScanControl::default(),
            &ScanLimits::default(),
        )
        .unwrap();
        reopened
            .install_discovered_roots(&mut next_scan, &ScanControl::default())
            .unwrap();
        let root = &next_scan.roots[0];
        assert!(root.is_excluded(std::path::Path::new("private")));
        assert!(root.is_excluded(std::path::Path::new("revoked")));
        assert!(root.is_excluded(std::path::Path::new("project/local-data")));
    }

    #[test]
    fn rediscovery_preserves_nested_protections_exclusions_and_revocations() {
        let temporary = tempfile::tempdir().unwrap();
        let home = temporary.path().join("home");
        let private = home.join("private");
        std::fs::create_dir_all(&private).unwrap();
        let mut backend = Backend::default();
        let root = ApprovedRoot::open(&private, ScopeKind::Protected).unwrap();
        backend.roots.get_mut().unwrap().insert(
            "protected".into(),
            RootEntry {
                info: ScopeInfo {
                    id: "protected".into(),
                    label: "private".into(),
                    display_path: root.display_path(),
                    kind: ScopeKind::Protected,
                    exclusions: vec![],
                },
                root,
            },
        );
        backend
            .automatic_exclusions
            .get_mut()
            .unwrap()
            .push(home.join("revoked"));
        let mut discovered = inodune_core::discovery::discover_from(
            &home,
            "",
            &[],
            &ScanControl::default(),
            &ScanLimits::default(),
        )
        .unwrap();
        backend
            .install_discovered_roots(&mut discovered, &ScanControl::default())
            .unwrap();
        assert!(discovered.roots[0].is_excluded(std::path::Path::new("private")));
        assert!(discovered.roots[0].is_excluded(std::path::Path::new("revoked")));
        let previous = backend.roots.lock().unwrap().len();
        let control = ScanControl::default();
        control.cancel();
        backend
            .install_discovered_roots(&mut discovered, &control)
            .unwrap();
        assert!(discovered.roots.is_empty());
        assert_eq!(backend.roots.lock().unwrap().len(), previous);
    }
}
