fn main() {
    // No application invoke handlers or plugins: the prototype has no FS authority.
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run Inodune desktop");
}
