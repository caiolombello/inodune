//! Native IPC for reports. Saved JSON is presentation data, never cleanup authority.
use crate::cleanup::PromptLocale;
use inodune_clean::{executor::ExecutionOptions, history::HistoryEntry};
use tauri_plugin_dialog::DialogExt;

fn history_directory() -> Result<std::path::PathBuf, String> {
    ExecutionOptions::local()?
        .history_directory
        .ok_or_else(|| "Histórico local não configurado.".to_owned())
}

#[tauri::command]
pub async fn list_history() -> Result<Vec<HistoryEntry>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        inodune_clean::history::list_history(&history_directory()?)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_history_report(
    report_id: String,
) -> Result<inodune_clean::executor::ExecutionReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        inodune_clean::history::get_history_report(&history_directory()?, &report_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn export_history_report(
    app: tauri::AppHandle,
    report_id: String,
    locale: Option<PromptLocale>,
) -> Result<Option<String>, String> {
    let report = get_history_report(report_id).await?;
    let path = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title(if matches!(locale, Some(PromptLocale::EnUs)) {
                "Export local report · includes file paths · choose a NEW file"
            } else {
                "Exportar relatório local · contém caminhos · escolha um NOVO arquivo"
            })
            .set_file_name("inodune-report.json")
            .add_filter("JSON", &["json"])
            .blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(path) = path else {
        return Ok(None);
    };
    let destination = path.into_path().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        inodune_clean::storage::export_report(&destination, &report)?;
        Ok(Some(destination.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|e| e.to_string())?
}
