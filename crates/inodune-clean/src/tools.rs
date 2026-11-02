//! Official cache commands. Fixed argv, no shell, binaries only from fixed directories.
use inodune_core::scan::{Fingerprint, ScannedTarget};
use rustix::fs::{self, FileType, Mode, OFlags};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const BIN_DIRS: &[&str] = &["/usr/bin", "/bin", "/usr/local/bin"];
const TIMEOUT: Duration = Duration::from_secs(120);

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

/// Runs the official purge when a trusted binary exists.
/// `Ok(false)` means the binary is absent and the caller should delete internally.
pub fn purge(detector: &str, target: &ScannedTarget) -> Result<bool, String> {
    let (tool, args) = match detector {
        "python.pip-cache" => ("pip3", vec!["cache".into(), "purge".into()]),
        "npm.cache" => (
            "npm",
            vec!["cache".into(), "clean".into(), "--force".into()],
        ),
        "uv.cache" => ("uv", vec!["cache".into(), "clean".into()]),
        "rust.target" => {
            let manifest = cargo_manifest(target)?;
            (
                "cargo",
                vec!["clean".into(), format!("--manifest-path={manifest}")],
            )
        }
        _ => return Ok(false),
    };
    let Some(bin) = resolve(tool).or_else(|| if tool == "pip3" { resolve("pip") } else { None })
    else {
        return Ok(false);
    };
    run(&bin, &args.iter().map(String::as_str).collect::<Vec<_>>()).map(|()| true)
}

fn cargo_manifest(target: &ScannedTarget) -> Result<String, String> {
    let (relative, expected) = target
        .witnesses
        .iter()
        .find(|(path, _)| path.file_name().is_some_and(|name| name == "Cargo.toml"))
        .ok_or("Manifesto Cargo ausente")?;
    let fd = target
        .root
        .open_relative(relative, OFlags::PATH)
        .map_err(|_| "Manifesto Cargo inacessível")?;
    if Fingerprint::from_stat(&fs::fstat(&fd).map_err(|e| e.to_string())?) != *expected {
        return Err("Manifesto Cargo mudou".into());
    }
    let absolute = target.root.path().join(relative);
    if !absolute.is_absolute()
        || absolute
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("Manifesto Cargo ambíguo".into());
    }
    absolute
        .to_str()
        .filter(|value| !value.chars().any(|c| c.is_control()))
        .map(str::to_owned)
        .ok_or_else(|| "Manifesto Cargo não é Unicode".into())
}

fn resolve(name: &str) -> Option<PathBuf> {
    if !matches!(name, "pip" | "pip3" | "npm" | "uv" | "cargo") {
        return None;
    }
    for dir in BIN_DIRS {
        let path = Path::new(dir).join(name);
        if trusted_binary(&path) {
            return Some(path);
        }
    }
    None
}

fn trusted_binary(path: &Path) -> bool {
    let fd = fs::openat2(
        fs::CWD,
        path,
        OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
        fs::ResolveFlags::NO_SYMLINKS | fs::ResolveFlags::NO_MAGICLINKS,
    );
    let Ok(fd) = fd else {
        return false;
    };
    let Ok(st) = fs::fstat(&fd) else {
        return false;
    };
    FileType::from_raw_mode(st.st_mode) == FileType::RegularFile
        && st.st_uid == 0
        && st.st_mode & 0o022 == 0
        && st.st_mode & 0o111 != 0
}

fn run(bin: &Path, args: &[&str]) -> Result<(), String> {
    if args.iter().any(|arg| arg.chars().any(char::is_control)) {
        return Err("Argumento de comando recusado".into());
    }
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Não foi possível iniciar o comando oficial: {error}"))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => return Err(format!("Comando oficial encerrou com {status}")),
            Ok(None) if started.elapsed() > TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Comando oficial excedeu 120s e foi interrompido".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(error) => return Err(error.to_string()),
        }
    }
}
