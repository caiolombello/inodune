use rustix::fs::{fcntl_setfl, OFlags};
use std::{
    io::Read,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const LIMIT: usize = 8 * 1024 * 1024;
/// Closed command grammar. No caller-selected executable, options or environment.
pub(crate) enum Query<'a> {
    Dnf,
    Rpm(&'a [String]),
    Flatpak(bool),
    AptManual,
    Dpkg,
    CoreOwners(bool),
    Snap,
}
impl Query<'_> {
    fn spec(&self) -> Result<(&'static str, Vec<String>), String> {
        let (bin, args): (_, Vec<String>) = match self {
            Self::Dnf => ("/usr/bin/dnf", vec!["--cacheonly", "--no-plugins", "--config=/dev/null", "--setopt=reposdir=/nonexistent", "repoquery", "--userinstalled", "--qf=%{name}\n"].into_iter().map(str::to_owned).collect()),
            Self::Rpm(ids) => {
                if ids.is_empty() || ids.len() > 128 || ids.iter().any(|s| !super::valid_id(s)) { return Err("IDs RPM inválidos".into()); }
                let mut args = vec!["-q".into(), "--qf".into(), "%{NAME}\t%{VERSION}-%{RELEASE}\t%{SIZE}\t%{INSTALLTIME}\n".into(), "--".into()];
                args.extend(ids.iter().cloned()); ("/usr/bin/rpm", args)
            },
            Self::Flatpak(user) => ("/usr/bin/flatpak", vec!["list", "--app", if *user {"--user"} else {"--system"}, "--columns=application,name,version,size,branch,arch"].into_iter().map(str::to_owned).collect()),
            Self::AptManual => ("/usr/bin/apt-mark", vec!["showmanual".into()]),
            Self::CoreOwners(apt) => {
                let mut paths = Vec::new();
                for name in ["ls", "cat", "cp", "mv", "rm", "sh", "bash", "env", "chmod", "chown", "mkdir", "rmdir", "ln", "readlink", "stat", "touch", "id", "true", "false", "sleep", "sort", "cut", "head", "tail", "wc", "find", "grep", "sed", "awk"] {
                    let path = std::path::PathBuf::from(format!("/usr/bin/{name}"));
                    if path.exists() {
                        let resolved = std::fs::canonicalize(&path).map_err(|_| "Binário básico não verificável")?;
                        let resolved = resolved.to_string_lossy();
                        // dpkg databases may retain /bin paths on merged-/usr systems.
                        paths.push(if *apt { format!("*/{}", resolved.strip_prefix("/usr/").unwrap_or(resolved.trim_start_matches('/'))) } else { resolved.into_owned() });
                    }
                }
                paths.sort(); paths.dedup();
                if paths.is_empty() { return Err("Nenhum binário básico verificável".into()); }
                let mut args: Vec<String> = if *apt { vec!["-S".into(), "--".into()] } else { vec!["-qf".into(), "--qf=%{NAME}\n".into(), "--".into()] };
                args.extend(paths);
                (if *apt { "/usr/bin/dpkg-query" } else { "/usr/bin/rpm" }, args)
            },
            Self::Dpkg => ("/usr/bin/dpkg-query", vec!["-W".into(), "-f=${binary:Package}\t${Version}\t${Installed-Size}\t${Status}\t${Essential}\t${Priority}\n".into()]),
            Self::Snap => ("/usr/bin/snap", vec!["list".into(), "--all".into()]),
        };
        Ok((bin, args))
    }
}
pub(crate) fn run(query: Query<'_>) -> Result<String, String> {
    let (bin, args) = query.spec()?;
    let mut command = Command::new(bin);
    // DNF5 initializes log/state paths even for queries. Isolate its scratch
    // files; never point logdir/cache at active user state.
    let scratch = if matches!(query, Query::Dnf) {
        Some(tempfile::tempdir().map_err(|e| e.to_string())?)
    } else {
        None
    };
    if let Some(dir) = &scratch {
        command.arg(format!("--setopt=logdir={}", dir.path().display()));
        command.arg(format!("--setopt=cachedir={}", dir.path().display()));
    }
    command
        .args(args)
        .env_clear()
        .env("LC_ALL", "C")
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", "/nonexistent")
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = &scratch {
        command.env("HOME", dir.path());
    }
    // Flatpak needs the user's installation path, but no session bus or inherited loader variables.
    if matches!(query, Query::Flatpak(_)) {
        if let Some(home) = std::env::var_os("HOME") {
            command.env("HOME", home);
        }
        if let Some(data) = std::env::var_os("XDG_DATA_HOME") {
            command.env("XDG_DATA_HOME", data);
        }
    }
    capture(command, LIMIT, Duration::from_secs(20))
}
fn capture(mut command: Command, limit: usize, timeout: Duration) -> Result<String, String> {
    let bin = "consulta de inventário";
    let mut child = command
        .spawn()
        .map_err(|_| format!("{bin}: indisponível"))?;
    let result = (|| {
        let mut out = child.stdout.take().ok_or("stdout ausente")?;
        let mut err = child.stderr.take().ok_or("stderr ausente")?;
        fcntl_setfl(&out, OFlags::NONBLOCK).map_err(|e| e.to_string())?;
        fcntl_setfl(&err, OFlags::NONBLOCK).map_err(|e| e.to_string())?;
        let start = Instant::now();
        let mut output = vec![];
        let mut total = 0;
        loop {
            for (stream, keep) in [
                (&mut out as &mut dyn Read, true),
                (&mut err as &mut dyn Read, false),
            ] {
                let mut buf = [0; 16384];
                match stream.read(&mut buf) {
                    Ok(n) => {
                        total += n;
                        if total > limit {
                            return Err("Saída excede 8 MiB".into());
                        }
                        if keep {
                            output.extend_from_slice(&buf[..n]);
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e.to_string()),
                }
            }
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                // Drain remaining buffered bytes, still bounded.
                let mut tail = vec![];
                out.take((limit - total + 1) as u64)
                    .read_to_end(&mut tail)
                    .map_err(|e| e.to_string())?;
                total += tail.len();
                output.extend(tail);
                let mut discarded = vec![];
                err.take((limit.saturating_sub(total) + 1) as u64)
                    .read_to_end(&mut discarded)
                    .map_err(|e| e.to_string())?;
                total += discarded.len();
                if total > limit {
                    return Err("Saída excede limite".into());
                }
                if !status.success() {
                    return Err(format!("{bin}: consulta falhou ({status})"));
                }
                return String::from_utf8(output).map_err(|_| "Saída não UTF-8".into());
            }
            if start.elapsed() > timeout {
                return Err(format!("{bin}: timeout de 20s"));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    })();
    // Only this task's child, never another process. Always reap on timeout/error.
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_grammar_rejects_hostile_rpm_arguments() {
        for id in ["-x", "a b", "x;y", "$(id)", "../escape"] {
            assert!(Query::Rpm(&[id.into()]).spec().is_err());
        }
        let (bin, args) = Query::Dnf.spec().unwrap();
        assert_eq!(bin, "/usr/bin/dnf");
        assert!(args.contains(&"--qf=%{name}\n".into()));
        assert!(!args.contains(&"--installed".into()));
        let (_, args) = Query::Rpm(&["editor".into()]).spec().unwrap();
        assert_eq!(args[3], "--");
    }
}
#[cfg(test)]
mod process_tests {
    use super::*;
    #[test]
    fn fixture_child() {
        match std::env::var("INODUNE_PROCESS_FIXTURE_MODE").as_deref() {
            Ok("timeout") => std::thread::sleep(Duration::from_secs(10)),
            Ok("output") => {
                use std::io::Write;
                std::io::stdout().write_all(&vec![b'x'; 100000]).unwrap();
            }
            _ => {}
        }
    }
    #[test]
    fn bounds_output_and_terminates_only_owned_child() {
        for mode in ["timeout", "output"] {
            let mut command = Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "command::process_tests::fixture_child",
                    "--nocapture",
                ])
                .env_clear()
                .env("INODUNE_PROCESS_FIXTURE_MODE", mode)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            let e = capture(command, 4096, Duration::from_millis(100)).unwrap_err();
            assert!(
                e.contains(if mode == "timeout" {
                    "timeout"
                } else {
                    "excede"
                }),
                "{e}"
            );
        }
    }
}
