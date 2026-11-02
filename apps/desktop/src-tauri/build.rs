fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "choose_scope",
            "list_scopes",
            "forget_scope",
            "set_scope_exclusions",
            "list_history",
            "get_history_report",
            "export_history_report",
            "begin_scan",
            "begin_automatic_scan",
            "poll_scan",
            "control_scan",
            "prepare_cleanup",
            "prepare_safe_cleanup",
            "begin_cleanup",
            "poll_cleanup",
            "cancel_cleanup",
            "list_installed_apps",
            "uninstall_prepare",
            "uninstall_execute",
        ]),
    ))
    .expect("failed to build native command permissions");
}
