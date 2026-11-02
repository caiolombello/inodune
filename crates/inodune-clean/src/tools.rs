//! Disposition of a cleanup item.
//!
//! Official tool commands are not used. Their config files and environment can
//! redirect deletion away from the scanned path (`CARGO_TARGET_DIR`,
//! `npm_config_cache`, `.cargo/config.toml`, `.npmrc`). The internal unlink
//! path revalidates the manifest instead.
//!
//! Every file follows the plan's [`Action`](crate::plan::Action): no detector
//! is deleted directly when the plan says Trash. User-facing wording comes from
//! [`Action::label`](crate::plan::Action::label) and
//! [`Action::consequence`](crate::plan::Action::consequence).
