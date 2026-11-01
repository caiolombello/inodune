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
    plans: Mutex<HashMap<String, PlanSession>>,
    job: Mutex<Option<Arc<Mutex<CleanupJob>>>>,
    sequence: AtomicU64,
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
) -> Result<PlanView, String> {
    if state.cleanup.active()? {
        return Err("Já existe uma limpeza em andamento.".into());
    }
    let snapshot = state.snapshot(&scan_id)?;
    state.check_approved(&snapshot, &candidate_ids)?;
    let plan = PlanSession::prepare(
        snapshot,
        &candidate_ids,
        match action {
            RequestedAction::Trash => Action::Trash,
            RequestedAction::Permanent => Action::Permanent,
        },
    )?;
    let view = plan.view();
    let mut plans = state
        .cleanup
        .plans
        .lock()
        .map_err(|_| "Estado indisponível")?;
    plans.clear();
    plans.insert(view.id.clone(), plan);
    Ok(view)
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
    let view = plan.view();
    let ids = view
        .items
        .iter()
        .map(|i| i.candidate_id.clone())
        .collect::<Vec<_>>();
    let snapshot = state.snapshot(&view.snapshot_id)?;
    state.check_approved(&snapshot, &ids)?;
    let paths = view
        .items
        .iter()
        .take(12)
        .map(|i| {
            format!(
                "• {}\n  Risco: {}\n  {}\n  {}",
                i.display_path,
                if i.risk == inodune_core::Risk::Review {
                    "revisão individual"
                } else {
                    "baixo"
                },
                i.impact,
                i.rebuild_hint
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let action = if view.action == Action::Trash {
        "Mover arquivos de cache à Lixeira. Eles continuarão ocupando espaço."
    } else {
        "REMOVER PERMANENTEMENTE os arquivos de cache selecionados. Não há desfazer nem garantia de apagamento seguro."
    };
    let prompt=format!("{action}\n\n{} itens · {} bytes lógicos estimados\n{}\n{}\nPlano: {}\n\nArquivos alterados serão preservados. Pastas, código e estado não serão removidos.",view.items.len(),view.logical_bytes_estimate,paths,if view.items.len()>12{"Demais itens estão na revisão do aplicativo."}else{""},view.hash);
    // Test-only consent is restricted to roots generated by this process's fixture factory.
    let fixture_consent = state.fixture_consent(&snapshot, &ids);
    let confirmed = if fixture_consent {
        true
    } else {
        tauri::async_runtime::spawn_blocking(move || {
            app.dialog()
                .message(prompt)
                .title("Inodune · Confirmar plano de limpeza")
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
