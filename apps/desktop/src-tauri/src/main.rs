mod backend;
fn main() {
    #[cfg(feature = "fixture-tests")]
    let backend = backend::Backend::fixtures().expect("failed to create isolated fixture roots");
    #[cfg(not(feature = "fixture-tests"))]
    let backend = backend::Backend::default();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(backend)
        .invoke_handler(tauri::generate_handler![
            backend::choose_scope,
            backend::list_scopes,
            backend::forget_scope,
            backend::begin_scan,
            backend::poll_scan,
            backend::control_scan
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Inodune desktop");
}
