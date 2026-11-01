fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "choose_scope",
            "list_scopes",
            "forget_scope",
            "begin_scan",
            "poll_scan",
            "control_scan",
        ]),
    ))
    .expect("failed to build native command permissions");
}
