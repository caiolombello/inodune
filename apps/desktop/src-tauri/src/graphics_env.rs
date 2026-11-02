//! WebKitGTK rendering default for environments without DRI3/DMA-BUF.
//!
//! QA saw the desktop crash with SIGSEGV in the first second in 5 of 14
//! launches in a container without DRI3 ("libEGL warning: DRI3 error"); with
//! WEBKIT_DISABLE_DMABUF_RENDERER=1 it was 0 of 6. The variable is set only
//! when the user has not defined it (any value, even empty, is respected), so
//! `WEBKIT_DISABLE_DMABUF_RENDERER=0` restores WebKit's DMA-BUF renderer.
use std::ffi::OsStr;

pub const DMABUF_VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";

/// Value to set for [`DMABUF_VAR`], or `None` to leave the environment as is.
pub fn dmabuf_default(current: Option<&OsStr>) -> Option<&'static str> {
    match current {
        None => Some("1"),
        Some(_) => None,
    }
}

/// Must run first in `main`, before any thread, GTK or WebKit exists.
/// Safety: setenv is not thread-safe (and `set_var` is `unsafe` in Rust
/// 2024); at this point the process is still single-threaded, so no other
/// thread can read or write the environment concurrently.
pub fn apply_before_threads() {
    if let Some(value) = dmabuf_default(std::env::var_os(DMABUF_VAR).as_deref()) {
        std::env::set_var(DMABUF_VAR, value);
    }
}

#[cfg(test)]
mod tests {
    use super::dmabuf_default;
    use std::ffi::OsStr;

    #[test]
    fn sets_only_when_absent_and_respects_any_user_value() {
        assert_eq!(dmabuf_default(None), Some("1"));
        assert_eq!(dmabuf_default(Some(OsStr::new(""))), None);
        assert_eq!(dmabuf_default(Some(OsStr::new("0"))), None);
        assert_eq!(dmabuf_default(Some(OsStr::new("1"))), None);
    }
}
