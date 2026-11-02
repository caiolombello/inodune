//! O renderer recebe uma vista. A autoridade continua no helper de uso único.
use inodune_apps::uninstall::{self, HelperChild, Request, View};
use std::{
    io::Write,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
#[derive(Default)]
pub struct UninstallState {
    pending: Mutex<Option<Prepared>>,
    active: AtomicBool,
}
struct Prepared {
    child: HelperChild,
    view: View,
}
struct Active<'a>(&'a AtomicBool);
impl Drop for Active<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
impl UninstallState {
    fn reserve(&self) -> Result<Active<'_>, String> {
        self.active
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| "Já existe uma desinstalação em andamento")?;
        Ok(Active(&self.active))
    }
}
#[tauri::command]
pub async fn uninstall_prepare(
    state: tauri::State<'_, UninstallState>,
    source: inodune_apps::Source,
    id: String,
    ids: Option<Vec<String>>,
) -> Result<View, String> {
    let _active = state.reserve()?;
    uninstall::refuse_root()?;
    uninstall::validate_id(source, &id)?;
    let ids = ids.unwrap_or_default();
    let request = Request {
        source,
        id: if ids.is_empty() { id } else { ids[0].clone() },
        ids,
    };
    let prepared = tauri::async_runtime::spawn_blocking(move || {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let helper = exe
            .parent()
            .ok_or("Pasta do aplicativo indisponível")?
            .join("inodune-uninstall-helper");
        let child = Command::new(helper)
            .env_remove("LD_PRELOAD")
            .env_remove("LD_LIBRARY_PATH")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "Helper de desinstalação indisponível")?;
        let mut prepared = Prepared {
            child: HelperChild::new(child),
            view: View {
                id: String::new(),
                items: vec![],
                expires_in_seconds: 0,
                steam_handoff: false,
            },
        };
        writeln!(
            prepared.child.stdin.as_mut().ok_or("Canal indisponível")?,
            "{}",
            serde_json::to_string(&request).map_err(|e| e.to_string())?
        )
        .map_err(|e| e.to_string())?;
        let text = uninstall::read_message(
            prepared
                .child
                .stdout
                .as_mut()
                .ok_or("Resposta indisponível")?,
            Duration::from_secs(900),
        )?;
        let value: serde_json::Value =
            serde_json::from_str(&text).map_err(|_| "Resposta inválida do helper")?;
        if let Some(error) = value["error"].as_str() {
            return Err(error.to_owned());
        }
        prepared.view = serde_json::from_value(value).map_err(|_| "Plano inválido do helper")?;
        Ok::<_, String>(prepared)
    })
    .await
    .map_err(|e| e.to_string())??;
    let view = prepared.view.clone();
    *state.pending.lock().map_err(|_| "Estado indisponível")? = Some(prepared);
    Ok(view)
}
#[tauri::command]
pub async fn uninstall_execute(
    app: tauri::AppHandle,
    state: tauri::State<'_, UninstallState>,
    plan_id: String,
) -> Result<String, String> {
    let _active = state.reserve()?;
    let mut prepared = state
        .pending
        .lock()
        .map_err(|_| "Estado indisponível")?
        .take()
        .ok_or("Plano ausente ou já consumido")?;
    if prepared.view.id != plan_id {
        return Err("ID de plano inválido".into());
    }
    tauri::async_runtime::spawn_blocking(move|| {
        let list=prepared.view.items.iter().map(|i|format!("{} · {}",i.id,i.version)).collect::<Vec<_>>().join("\n");
        let message=if prepared.view.steam_handoff {format!("Abrir a confirmação de desinstalação no Steam?\n\n{list}")}else{format!("Desinstalar os pacotes abaixo, inclusive dependências que saem junto? Não há autoremove nem purge. O sistema pode pedir autorização.\n\n{list}")};
        let accepted=app.dialog().message(message).title("Confirmar desinstalação").kind(MessageDialogKind::Warning).buttons(MessageDialogButtons::OkCancel).blocking_show();
        if !accepted {return Err("Desinstalação cancelada; plano consumido".into());}
        prepared.child.mark_execution_sent();
        writeln!(prepared.child.stdin.as_mut().ok_or("Canal indisponível")?,"{}",serde_json::to_string(&plan_id).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        let result=uninstall::read_message(prepared.child.stdout.as_mut().ok_or("Canal indisponível")?,Duration::from_secs(900)).map_err(|_|"Resultado desconhecido; consulte o inventário. O helper não foi encerrado.".to_owned())?;
        let value:serde_json::Value=serde_json::from_str(&result).map_err(|_|"Resposta inválida; resultado desconhecido")?;
        if value["status"]!="completed" {return Err(value["error"].as_str().unwrap_or("Resultado desconhecido; não tente novamente sem consultar o inventário").to_owned());}
        Ok(if prepared.view.steam_handoff {"Confirmação aberta no Steam. O inodune não confirma a remoção."}else{"Transação concluída pelo gerenciador de pacotes."}.to_owned())
    }).await.map_err(|e|e.to_string())?
}
