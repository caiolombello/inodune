//! Disposition of a cleanup item.
//!
//! Official tool commands are not used. Their config files and environment can
//! redirect deletion away from the scanned path (`CARGO_TARGET_DIR`,
//! `npm_config_cache`, `.cargo/config.toml`, `.npmrc`). The internal unlink
//! path revalidates the manifest instead.
pub fn direct_delete(detector: &str) -> bool {
    matches!(
        detector,
        "python.pip-cache"
            | "npm.cache"
            | "next.cache"
            | "python.mypy-cache"
            | "python.ruff-cache"
            | "uv.cache"
            | "trash.entry"
    )
}
