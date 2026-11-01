//! Bounded local crash markers, without panic payloads, inspected paths or uploads.
use crate::storage;
use rustix::fs::{self, AtFlags, Mode, OFlags};
use std::{io::Write, path::Path};

pub fn install_local_panic_reporter() {
    std::panic::set_hook(Box::new(|info| {
        let source = info
            .location()
            .and_then(|l| Path::new(l.file()).file_name());
        let source = source.and_then(|s| s.to_str()).unwrap_or("unknown");
        let line = info.location().map(|l| l.line()).unwrap_or(0);
        let saved = storage::xdg_home("XDG_STATE_HOME", ".local/state")
            .and_then(|state| save_marker(&state.join("inodune/crashes"), source, line));
        eprintln!(
            "Inodune: unexpected termination. Local diagnostic marker {}. No upload.",
            if saved.is_ok() {
                "saved"
            } else {
                "unavailable"
            }
        );
    }));
}

fn save_marker(directory: &Path, source: &str, line: u32) -> Result<(), String> {
    let dir = storage::open_dir(directory, true, true)?;
    let mut buffer = [std::mem::MaybeUninit::uninit(); 8192];
    let mut entries = fs::RawDir::new(&dir, &mut buffer);
    let (mut count, mut bytes) = (0usize, 0u64);
    while let Some(entry) = entries.next() {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        if name.to_bytes() == b"." || name.to_bytes() == b".." {
            continue;
        }
        count += 1;
        let stat = fs::statat(&dir, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|e| e.to_string())?;
        bytes = bytes.saturating_add(stat.st_size.max(0) as u64);
        if count >= 20 || bytes > 127 * 1024 {
            return Err("Local diagnostic limit reached; archive manually.".into());
        }
    }
    // Only the compiled source basename and line; never panic payload/thread names.
    let record = serde_json::to_vec(&serde_json::json!({
        "schema_version": 1,
        "kind": "panic",
        "unix_seconds": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs(),
        "source": Path::new(source).file_name().and_then(|s| s.to_str()).unwrap_or("unknown"),
        "line": line,
        "application_version": env!("CARGO_PKG_VERSION")
    })).map_err(|e| e.to_string())?;
    if record.len() > 1024 {
        return Err("Diagnostic marker too large.".into());
    }
    let name = storage::random_name("crash-")? + ".json";
    let fd = fs::openat(
        &dir,
        name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|e| e.to_string())?;
    let mut file = std::fs::File::from(fd);
    file.write_all(&record)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    fs::fsync(&dir).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_is_private_bounded_and_has_no_full_source_path() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("crashes");
        for _ in 0..20 {
            save_marker(&path, "/private/build/project/main.rs", 42).unwrap();
        }
        assert!(save_marker(&path, "main.rs", 43).is_err());
        assert_eq!(std::fs::read_dir(&path).unwrap().count(), 20);
        for file in std::fs::read_dir(&path).unwrap() {
            let file = file.unwrap();
            assert_eq!(file.metadata().unwrap().permissions().mode() & 0o777, 0o600);
            let record = std::fs::read_to_string(file.path()).unwrap();
            assert!(record.contains("main.rs"));
            assert!(!record.contains("/private"));
        }
    }
}
