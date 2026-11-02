#[cfg(all(feature = "fixture-tests", not(debug_assertions)))]
compile_error!("fixture-tests is forbidden in release builds");
mod backend;
mod cleanup;
mod graphics_env;
mod history;
mod uninstall;
use tauri::Manager;
#[tauri::command]
async fn list_installed_apps() -> Result<inodune_apps::Inventory, String> {
    tauri::async_runtime::spawn_blocking(inodune_apps::inventory)
        .await
        .map_err(|e| e.to_string())?
}
fn main() {
    // First statement: the process is still single-threaded here (see
    // graphics_env::apply_before_threads for the safety argument).
    graphics_env::apply_before_threads();
    inodune_clean::diagnostics::install_local_panic_reporter();
    #[cfg(feature = "fixture-tests")]
    let backend = backend::Backend::fixtures().expect("failed to create isolated fixture roots");
    #[cfg(not(feature = "fixture-tests"))]
    let backend = backend::Backend::default();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(backend)
        .manage(uninstall::UninstallState::default())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<backend::Backend>();
                if state.cleanup.active().unwrap_or(true) {
                    api.prevent_close();
                    state.cleanup.cancel_active();
                    // Keep the report visible. A second close after completion is allowed.
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_installed_apps,
            uninstall::uninstall_prepare,
            uninstall::uninstall_execute,
            backend::choose_scope,
            backend::list_scopes,
            backend::forget_scope,
            backend::set_scope_exclusions,
            backend::begin_scan,
            backend::begin_automatic_scan,
            backend::poll_scan,
            backend::control_scan,
            cleanup::prepare_cleanup,
            cleanup::prepare_safe_cleanup,
            cleanup::begin_cleanup,
            cleanup::poll_cleanup,
            cleanup::cancel_cleanup,
            history::list_history,
            history::get_history_report,
            history::export_history_report
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Inodune desktop");
}
