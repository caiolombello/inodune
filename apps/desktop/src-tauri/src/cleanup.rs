//! Cleanup commands accept opaque IDs only. Final consent belongs to a native dialog.
use crate::backend::Backend;
use inodune_clean::{
    executor::{self, ExecutionOptions, ExecutionProgress, ExecutionReport},
    plan::{Action, PlanSession, PlanView},
};
use inodune_core::scan::ScanControl;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

#[derive(Default)]
pub struct CleanupState {
    plans: Mutex<HashMap<String, PreparedCleanup>>,
    job: Mutex<Option<Arc<Mutex<CleanupJob>>>>,
    sequence: AtomicU64,
}
#[derive(Clone, Copy, Default, Deserialize)]
pub enum PromptLocale {
    #[default]
    #[serde(rename = "pt-BR")]
    PtBr,
    #[serde(rename = "en-US")]
    EnUs,
}
struct PreparedCleanup {
    session: PlanSession,
    locale: PromptLocale,
}
struct CleanupJob {
    id: String,
    control: Arc<ScanControl>,
    progress: ExecutionProgress,
    report: Option<ExecutionReport>,
    error: Option<String>,
}
impl CleanupState {
    pub fn active(&self) -> Result<bool, String> {
        let slot = self.job.lock().map_err(|_| "Estado indisponível")?;
        if let Some(j) = slot.as_ref() {
            let j = j.lock().map_err(|_| "Estado indisponível")?;
            Ok(j.report.is_none() && j.error.is_none())
        } else {
            Ok(false)
        }
    }
    pub fn cancel_active(&self) {
        if let Ok(slot) = self.job.lock() {
            if let Some(job) = slot.as_ref() {
                if let Ok(job) = job.lock() {
                    job.control.cancel();
                }
            }
        }
    }
    pub fn invalidate(&self) -> Result<(), String> {
        self.plans
            .lock()
            .map_err(|_| "Estado indisponível")?
            .clear();
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestedAction {
    Trash,
    Permanent,
}
#[derive(Serialize)]
pub struct CleanupPoll {
    status: &'static str,
    progress: ExecutionProgress,
    report: Option<ExecutionReport>,
    error: Option<String>,
}
#[tauri::command]
pub fn prepare_cleanup(
    state: State<'_, Backend>,
    scan_id: String,
    candidate_ids: Vec<String>,
    action: RequestedAction,
    locale: Option<PromptLocale>,
) -> Result<PlanView, String> {
    let _gate = state
        .operation_gate
        .lock()
        .map_err(|_| "Estado indisponível")?;
    if state.cleanup.active()? {
        return Err("Já existe uma limpeza em andamento.".into());
    }
    let snapshot = state.snapshot(&scan_id)?;
    state.check_approved(&snapshot, &candidate_ids)?;
    let prepare = if state.is_automatic_scan(&scan_id)? {
        PlanSession::prepare_verified_items
    } else {
        PlanSession::prepare
    };
    let requested = match action {
        RequestedAction::Trash => Action::Trash,
        RequestedAction::Permanent => Action::Permanent,
    };
    executor::preflight(&executor::ExecutionOptions::local()?, requested)?;
    let plan = prepare(snapshot, &candidate_ids, requested)?;
    let view = plan.view();
    let mut plans = state
        .cleanup
        .plans
        .lock()
        .map_err(|_| "Estado indisponível")?;
    plans.clear();
    plans.insert(
        view.id.clone(),
        PreparedCleanup {
            session: plan,
            locale: locale.unwrap_or_default(),
        },
    );
    Ok(view)
}
fn safe_candidate(candidate: &inodune_core::Candidate) -> bool {
    candidate.risk != inodune_core::Risk::Protected
        && !matches!(
            candidate.supported_action,
            inodune_core::SupportedAction::None
        )
        && matches!(
            candidate.detector_id.as_str(),
            "python.pycache"
                | "thumbnails"
                | "python.pip-cache"
                | "npm.cache"
                | "next.cache"
                | "python.mypy-cache"
                | "python.ruff-cache"
                | "uv.cache"
                | "trash.entry"
                | "rust.target"
                | "cargo.fingerprint-json"
                | "terraform.provider-package"
        )
}
#[tauri::command]
pub fn prepare_safe_cleanup(
    state: State<'_, Backend>,
    scan_id: String,
    action: RequestedAction,
    locale: Option<PromptLocale>,
) -> Result<PlanView, String> {
    let snapshot = state.snapshot(&scan_id)?;
    let ids = snapshot
        .candidates
        .iter()
        .filter(|candidate| safe_candidate(candidate))
        .map(|candidate| candidate.id.clone())
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Err("Nenhum item seguro para limpar.".into());
    }
    prepare_cleanup(state, scan_id, ids, action, locale)
}
#[tauri::command]
pub async fn begin_cleanup(
    app: AppHandle,
    state: State<'_, Backend>,
    plan_id: String,
    hash: String,
) -> Result<String, String> {
    // Reserve the single execution slot before displaying a native prompt.
    let job = {
        let _gate = state
            .operation_gate
            .lock()
            .map_err(|_| "Estado indisponível")?;
        let mut slot = state
            .cleanup
            .job
            .lock()
            .map_err(|_| "Estado indisponível")?;
        if let Some(j) = slot.as_ref() {
            let j = j.lock().map_err(|_| "Estado indisponível")?;
            if j.report.is_none() && j.error.is_none() {
                return Err("Já existe uma limpeza em andamento.".into());
            }
        }
        let id = format!(
            "cleanup-{}",
            state.cleanup.sequence.fetch_add(1, Ordering::Relaxed)
        );
        let job = Arc::new(Mutex::new(CleanupJob {
            id,
            control: Arc::new(ScanControl::default()),
            progress: ExecutionProgress {
                completed_files: 0,
                total_files: 0,
                completed_items: 0,
                total_items: 0,
            },
            report: None,
            error: None,
        }));
        *slot = Some(job.clone());
        job
    };
    let result = begin_authorized(app, &state, &plan_id, &hash, &job).await;
    if let Err(e) = &result {
        if let Ok(mut j) = job.lock() {
            j.error = Some(e.clone());
        }
    }
    result
}
async fn begin_authorized(
    app: AppHandle,
    state: &Backend,
    plan_id: &str,
    hash: &str,
    job: &Arc<Mutex<CleanupJob>>,
) -> Result<String, String> {
    let plan = state
        .cleanup
        .plans
        .lock()
        .map_err(|_| "Estado indisponível")?
        .remove(plan_id)
        .ok_or("Plano expirado, consumido ou desconhecido")?;
    let PreparedCleanup {
        session: plan,
        locale,
    } = plan;
    let english = matches!(locale, PromptLocale::EnUs);
    let view = plan.view();
    let ids = view
        .items
        .iter()
        .map(|i| i.candidate_id.clone())
        .collect::<Vec<_>>();
    let snapshot = state.snapshot(&view.snapshot_id)?;
    state.check_approved(&snapshot, &ids)?;
    let paths=view.items.iter().take(12).map(|i|{
        if english {
            let detector=snapshot.candidates.iter().find(|c|c.id==i.candidate_id).map(|c|c.detector_id.as_str()).unwrap_or("");
            let impact=match detector {
                "python.pycache"=>"Python can regenerate bytecode from the preserved source. The first import may be slower.",
                "thumbnails"=>"Applications can regenerate thumbnails. The first view may be slower.",
                "cargo.fingerprint-json"=>"Cargo may rebuild after diagnostic metadata is removed. Manifests, hashes and final binaries are preserved.",
                "terraform.provider-package"=>"No relevant lock selects this version. Terraform may need to download it again. State, backend and locks are preserved. Current use, publisher signature and future availability are unverified.",
                _=>"Rebuild cost is unmeasured; preserve uncertain data.",
            };
            format!("• {}\n  Risk: {}\n  {impact}",i.display_path,if i.risk==inodune_core::Risk::Review{"review"}else{"low"})
        } else {format!("• {}\n  Risco: {}\n  {}\n  {}",i.display_path,if i.risk==inodune_core::Risk::Review{"revisar"}else{"baixo"},i.impact,i.rebuild_hint)}
    }).collect::<Vec<_>>().join("\n");
    let mut direct = 0usize;
    let mut trashed = 0usize;
    let mut groups: std::collections::BTreeMap<String, (usize, u64)> =
        std::collections::BTreeMap::new();
    for item in &view.items {
        let candidate = snapshot
            .candidates
            .iter()
            .find(|c| c.id == item.candidate_id);
        let detector = candidate.map(|c| c.detector_id.as_str()).unwrap_or("");
        let title = candidate.map(|c| c.title.as_str()).unwrap_or("Item");
        let deletes_now =
            view.action == Action::Permanent || inodune_clean::tools::direct_delete(detector);
        if deletes_now {
            direct += 1;
        } else {
            trashed += 1;
        }
        let entry = groups.entry(title.to_owned()).or_insert((0, 0));
        entry.0 += 1;
        entry.1 = entry.1.saturating_add(item.logical_bytes);
    }
    let summary = groups
        .iter()
        .map(|(title, (count, bytes))| format!("• {title}: {count} · {bytes}"))
        .collect::<Vec<_>>()
        .join("\n");
    let action = if english {
        format!("Recreatable caches and Trash entries are deleted immediately ({direct}). Other selected items move to Trash and keep using space ({trashed}). Terraform providers referenced by a lock are downloaded again on init. There is no secure-erasure guarantee.\n{summary}")
    } else {
        format!("Caches recriáveis e entradas da Lixeira são apagados direto ({direct}). O restante da seleção vai para a Lixeira e continua ocupando espaço ({trashed}). Providers Terraform citados no lock voltam no terraform init. Não há garantia de apagamento seguro.\n{summary}")
    };
    let coverage = if view.inventory_complete {
        ""
    } else if english {
        "\nPartial computer analysis: inaccessible areas were not examined. Only the selected, fully validated manifests are authorized.\n"
    } else {
        "\nAnálise parcial do computador: áreas inacessíveis não foram examinadas. Somente os manifestos selecionados e inteiramente validados estão autorizados.\n"
    };
    let prompt = if english {
        format!("{action}\n{coverage}\n{} items · {} estimated logical bytes\n{paths}\nPlan: {}\n\nChanged files will be preserved. Folders, source code and state will not be removed.",view.items.len(),view.logical_bytes_estimate,view.hash)
    } else {
        format!("{action}\n{coverage}\n{} itens · {} bytes lógicos estimados\n{paths}\nPlano: {}\n\nArquivos alterados serão preservados. Pastas, código e estado não serão removidos.",view.items.len(),view.logical_bytes_estimate,view.hash)
    };
    // Test-only consent is restricted to roots generated by this process's fixture factory.
    let fixture_consent = state.fixture_consent(&snapshot, &ids);
    let confirmed = if fixture_consent {
        true
    } else {
        tauri::async_runtime::spawn_blocking(move || {
            app.dialog()
                .message(prompt)
                .title(if english {
                    "Inodune · Confirm cleanup plan"
                } else {
                    "Inodune · Confirmar plano de limpeza"
                })
                .kind(MessageDialogKind::Warning)
                .buttons(MessageDialogButtons::OkCancel)
                .blocking_show()
        })
        .await
        .map_err(|e| e.to_string())?
    };
    if !confirmed {
        return Err("Limpeza cancelada na confirmação nativa. Nenhuma operação iniciada.".into());
    }
    state.check_approved(&snapshot, &ids)?;
    let authorized = plan.consume(hash)?; // TTL is checked after human confirmation.
    let options = ExecutionOptions::local()?;
    let id = job.lock().map_err(|_| "Estado indisponível")?.id.clone();
    let job = job.clone();
    std::thread::spawn(move || {
        let control = match job.lock() {
            Ok(j) => j.control.clone(),
            Err(_) => return,
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            executor::execute(authorized, &options, &control, |p| {
                if let Ok(mut j) = job.lock() {
                    j.progress = p;
                }
            })
        }));
        if let Ok(mut j) = job.lock() {
            match result{Ok(report)=>j.report=Some(report),Err(_)=>j.error=Some("Execução interrompida por falha interna. Pode haver operações concluídas e arquivos na área de recuperação; refaça o inventário.".into())}
        }
    });
    Ok(id)
}
#[tauri::command]
pub fn poll_cleanup(state: State<'_, Backend>, job_id: String) -> Result<CleanupPoll, String> {
    let slot = state
        .cleanup
        .job
        .lock()
        .map_err(|_| "Estado indisponível")?;
    let job = slot.as_ref().ok_or("Execução inexistente")?;
    let j = job.lock().map_err(|_| "Estado indisponível")?;
    if j.id != job_id {
        return Err("Execução desconhecida".into());
    }
    Ok(CleanupPoll {
        status: if j.error.is_some() {
            "error"
        } else if j.report.is_some() {
            "done"
        } else {
            "running"
        },
        progress: j.progress.clone(),
        report: j.report.clone(),
        error: j.error.clone(),
    })
}
#[tauri::command]
pub fn cancel_cleanup(state: State<'_, Backend>, job_id: String) -> Result<(), String> {
    let slot = state
        .cleanup
        .job
        .lock()
        .map_err(|_| "Estado indisponível")?;
    let job = slot.as_ref().ok_or("Execução inexistente")?;
    let j = job.lock().map_err(|_| "Estado indisponível")?;
    if j.id != job_id {
        return Err("Execução desconhecida".into());
    }
    j.control.cancel();
    Ok(())
}
