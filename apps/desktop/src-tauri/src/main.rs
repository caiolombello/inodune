#[cfg(all(feature = "fixture-tests", not(debug_assertions)))]
compile_error!("fixture-tests is forbidden in release builds");
mod backend;
mod cleanup;
mod history;
use tauri::Manager;
fn main() {
    inodune_clean::diagnostics::install_local_panic_reporter();
    #[cfg(feature = "fixture-tests")]
    let backend = backend::Backend::fixtures().expect("failed to create isolated fixture roots");
    #[cfg(not(feature = "fixture-tests"))]
    let backend = backend::Backend::default();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(backend)
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
            backend::choose_scope,
            backend::list_scopes,
            backend::forget_scope,
            backend::set_scope_exclusions,
            backend::begin_scan,
            backend::begin_automatic_scan,
            backend::poll_scan,
            backend::control_scan,
            cleanup::prepare_cleanup,
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
