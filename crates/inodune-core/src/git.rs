//! Read-only, fail-closed Git worktree inventory.
//!
//! Git repositories and their configuration are untrusted input. Git is only run
//! inside bubblewrap with the approved directory descriptor mounted read-only.
//! If that isolation is unavailable, the inventory is protected and no Git
//! process is started.
use crate::scan::{ApprovedRoot, ScanControl, ScopeKind};
use rustix::{
    fs::{self, FileType, OFlags, RawDir},
    io::Errno,
};
use serde::{Serialize, Serializer};
use std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
    fs::File,
    io::Read,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Component, Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

const GIT: &str = "/usr/bin/git";
const BWRAP: &str = "/usr/bin/bwrap";
const MAX_COMMAND_OUTPUT: usize = 1024 * 1024;
const MAX_REF_ENTRIES: usize = 20_000;
const MAX_REF_DEPTH: usize = 32;
const MAX_WORKTREES: usize = 256;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(4);
const INSPECTION_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Serialize)]
pub struct GitInventory {
    #[serde(serialize_with = "serialize_path_lossy")]
    pub repository: PathBuf,
    pub repository_bytes_hex: Option<String>,
    pub worktrees: Vec<GitWorktree>,
    pub protected: bool,
    pub complete: bool,
    pub cancelled: bool,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GitWorktree {
    #[serde(serialize_with = "serialize_path_lossy")]
    pub path: PathBuf,
    #[serde(serialize_with = "serialize_optional_path_lossy")]
    pub relative_path: Option<PathBuf>,
    pub path_bytes_hex: Option<String>,
    pub relative_path_bytes_hex: Option<String>,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub ahead: Option<u64>,
    pub behind: Option<u64>,
    pub is_main: bool,
    pub detached: bool,
    pub locked: bool,
    pub prunable: bool,
    pub dirty: bool,
    pub untracked: bool,
    pub ignored: bool,
    pub submodule_state_unknown: bool,
    pub protected: bool,
    pub reasons: Vec<String>,
}

/// Inspect a repository root relative to an approved project directory.
///
/// This function never removes, prunes, repairs, locks, fetches, or writes a
/// repository. Anything it cannot establish with bounded read-only inspection
/// is returned as protected.
pub fn inspect(root: &ApprovedRoot, relative: &Path, control: &ScanControl) -> GitInventory {
    inspect_with_tools(root, relative, control, Path::new(BWRAP), Path::new(GIT))
}

fn inspect_with_tools(
    root: &ApprovedRoot,
    relative: &Path,
    control: &ScanControl,
    bwrap: &Path,
    git: &Path,
) -> GitInventory {
    let repository = root.path().join(relative);
    let mut inventory = GitInventory {
        repository_bytes_hex: repository
            .to_str()
            .is_none()
            .then(|| hex_bytes(repository.as_os_str().as_bytes())),
        repository: repository.clone(),
        worktrees: Vec::new(),
        protected: false,
        complete: true,
        cancelled: false,
        reasons: Vec::new(),
    };

    if !control.checkpoint() {
        return fail(&mut inventory, "Inspeção cancelada.", true);
    }
    if root.kind != ScopeKind::Projects || !valid_relative(relative) {
        return fail(
            &mut inventory,
            "Escopo ou caminho relativo não permitido.",
            false,
        );
    }
    if crate::scan::is_protected_agent_path(&repository) {
        return fail(
            &mut inventory,
            "Namespace de agentes protegido; somente metadados serão inventariados.",
            false,
        );
    }
    if root.has_exclusions() {
        return fail(
            &mut inventory,
            "Escopo tem exclusões; Git externo não é executado para preservar o limite de leitura.",
            false,
        );
    }
    if inventory.repository_bytes_hex.is_some() {
        return fail(
            &mut inventory,
            "Caminho do repositório contém bytes inválidos para o formato de evidência.",
            false,
        );
    }
    if let Err(error) = root.revalidate() {
        return fail(
            &mut inventory,
            &format!("Raiz aprovada indisponível: {error}"),
            false,
        );
    }
    if root
        .open_relative(
            relative,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOATIME,
        )
        .is_err()
    {
        return fail(
            &mut inventory,
            "Diretório do repositório inválido ou inacessível.",
            false,
        );
    }

    let Some(gitdir) = resolve_gitdir(root, relative) else {
        return fail(
            &mut inventory,
            "Metadados .git ausentes, externos ou incertos.",
            false,
        );
    };
    if !preflight_metadata(root, &gitdir) {
        return fail(
            &mut inventory,
            "Metadados Git contêm links, mounts ou estruturas incertas.",
            false,
        );
    }
    if let Some(reason) = sandbox_block_reason(
        has_nested_mount(root.path()),
        bwrap.is_file(),
        git.is_file(),
    ) {
        return fail(&mut inventory, reason, false);
    }

    let output = match run_git(
        root,
        &repository,
        &["worktree", "list", "--porcelain", "-z"],
        control,
        bwrap,
        git,
    ) {
        Ok(output) => output,
        Err(reason) => return fail(&mut inventory, &reason, control.is_cancelled()),
    };
    let mut listed = match parse_worktrees(&output) {
        Ok(records) if !records.is_empty() && records.len() <= MAX_WORKTREES => records,
        _ => {
            return fail(
                &mut inventory,
                "Saída Git de worktrees inválida ou acima do limite.",
                false,
            )
        }
    };

    let inspection_started = Instant::now();
    for (index, listed_tree) in listed.drain(..).enumerate() {
        if !control.checkpoint() {
            inventory.cancelled = true;
            inventory.complete = false;
            inventory.protected = true;
            inventory
                .reasons
                .push("Inspeção cancelada antes do próximo worktree.".into());
            break;
        }
        if inspection_started.elapsed() > INSPECTION_TIMEOUT {
            inventory.protected = true;
            inventory.complete = false;
            inventory
                .reasons
                .push("Inspeção excedeu o limite total de tempo.".into());
            break;
        }
        let path = listed_tree.path.expect("parser requires a worktree path");
        let relative_path = path.strip_prefix(root.path()).ok().map(|p| {
            if p.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                p.to_path_buf()
            }
        });
        let path_bytes_hex = path
            .to_str()
            .is_none()
            .then(|| hex_bytes(path.as_os_str().as_bytes()));
        let relative_path_bytes_hex = relative_path
            .as_deref()
            .filter(|path| path.to_str().is_none())
            .map(|path| hex_bytes(path.as_os_str().as_bytes()));
        let mut tree = GitWorktree {
            path,
            relative_path,
            path_bytes_hex,
            relative_path_bytes_hex,
            head: listed_tree.head,
            branch: listed_tree.branch,
            upstream: None,
            ahead: None,
            behind: None,
            is_main: index == 0,
            detached: listed_tree.detached,
            locked: listed_tree.locked,
            prunable: listed_tree.prunable,
            dirty: false,
            untracked: false,
            ignored: false,
            submodule_state_unknown: false,
            protected: false,
            reasons: Vec::new(),
        };
        if tree.path_bytes_hex.is_some() || tree.relative_path_bytes_hex.is_some() {
            inventory.complete = false;
            protect_tree(
                &mut tree,
                "Caminho contém bytes inválidos para o formato de evidência.",
            );
        }
        if tree.is_main {
            protect_tree(&mut tree, "Worktree principal sempre protegida.");
        }
        if tree.detached {
            protect_tree(&mut tree, "HEAD destacado.");
        }
        if tree.locked {
            protect_tree(&mut tree, "Worktree bloqueada.");
        }
        if tree.prunable {
            protect_tree(
                &mut tree,
                "Git marcou metadados como prunable; estado incerto.",
            );
        }
        if tree.path.strip_prefix(root.path()).is_err() {
            inventory.complete = false;
            protect_tree(&mut tree, "Worktree registrada fora da raiz aprovada.");
        } else {
            let relative_tree = match tree.path.strip_prefix(root.path()) {
                Ok(path) if path.as_os_str().is_empty() => Path::new("."),
                Ok(path) if valid_relative(path) => path,
                _ => {
                    inventory.complete = false;
                    protect_tree(&mut tree, "Caminho registrado inválido.");
                    inventory.worktrees.push(tree);
                    continue;
                }
            };
            if root
                .open_relative(
                    relative_tree,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOATIME,
                )
                .is_err()
            {
                inventory.complete = false;
                protect_tree(
                    &mut tree,
                    "Worktree ausente, montada ou não validável sob a raiz.",
                );
            } else if resolve_gitdir(root, relative_tree)
                .is_none_or(|tree_gitdir| !preflight_metadata(root, &tree_gitdir))
            {
                inventory.complete = false;
                protect_tree(
                    &mut tree,
                    "Metadados da worktree contêm links ou estado externo/incerto.",
                );
            } else {
                match submodule_state(root, relative_tree, &tree.path, control, bwrap, git) {
                    Ok(unknown) => {
                        tree.submodule_state_unknown = unknown;
                        if unknown {
                            protect_tree(
                                &mut tree,
                                "Submódulos não foram inspecionados; estado interno desconhecido.",
                            );
                        }
                    }
                    Err(reason) => {
                        tree.submodule_state_unknown = true;
                        inventory.complete = false;
                        protect_tree(
                            &mut tree,
                            &format!("Estado de submódulos incerto: {reason}"),
                        );
                    }
                }
                protect_tree(&mut tree,"Consulta de status exclui namespaces protegidos de agentes; não comprova a árvore inteira.");
                let status = run_git(
                    root,
                    &tree.path,
                    &[
                        "status",
                        "--porcelain=v2",
                        "-z",
                        "--branch",
                        "--untracked-files=normal",
                        "--ignored=matching",
                        "--ignore-submodules=all",
                        "--",
                        ".",
                        ":(top,exclude,glob)**/.codex/**",
                        ":(top,exclude,glob)**/.claude/**",
                        ":(top,exclude,glob)**/opencode/**",
                    ],
                    control,
                    bwrap,
                    git,
                );
                match status.and_then(|bytes| parse_status(&bytes)) {
                    Ok(result) => {
                        tree.upstream = result.upstream;
                        tree.ahead = result.ahead;
                        tree.behind = result.behind;
                        tree.dirty = result.dirty;
                        tree.untracked = result.untracked;
                        tree.ignored = result.ignored;
                        if tree.branch.is_none() || result.branch_head.is_none() {
                            protect_tree(&mut tree, "Branch ou cabeçalho de status ausente.");
                        }
                        if tree.upstream.is_none() {
                            protect_tree(&mut tree, "Branch sem upstream conhecido.");
                        }
                        if tree.ahead.unwrap_or(0) > 0 {
                            protect_tree(
                                &mut tree,
                                "Commits locais ainda não publicados no upstream.",
                            );
                        }
                        protect_tree(
                            &mut tree,
                            "Estado remoto não consultado; upstream local não comprova publicação atual.",
                        );
                        if tree.dirty {
                            protect_tree(&mut tree, "Worktree contém alterações locais.");
                        }
                        if tree.untracked {
                            protect_tree(&mut tree, "Worktree contém arquivos não rastreados.");
                        }
                        if tree.ignored {
                            protect_tree(&mut tree, "Worktree contém arquivos ignorados.");
                        }
                    }
                    Err(reason) => {
                        inventory.complete = false;
                        protect_tree(&mut tree, &format!("Status Git incerto: {reason}"));
                    }
                }
            }
        }
        if tree.protected {
            inventory.protected = true;
        }
        inventory.worktrees.push(tree);
    }
    if inventory.worktrees.is_empty() {
        fail(
            &mut inventory,
            "Nenhuma worktree pôde ser validada.",
            control.is_cancelled(),
        )
    } else {
        if control.is_cancelled() {
            inventory.cancelled = true;
            inventory.complete = false;
            inventory.protected = true;
        }
        inventory
    }
}

fn fail(inventory: &mut GitInventory, reason: &str, cancelled: bool) -> GitInventory {
    inventory.protected = true;
    inventory.complete = false;
    inventory.cancelled = cancelled;
    inventory.reasons.push(reason.to_owned());
    inventory.clone()
}

fn protect_tree(tree: &mut GitWorktree, reason: &str) {
    tree.protected = true;
    tree.reasons.push(reason.to_owned());
}

fn valid_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_) | Component::CurDir))
}

fn serialize_path_lossy<S: Serializer>(path: &Path, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&path.to_string_lossy())
}

fn serialize_optional_path_lossy<S: Serializer>(
    path: &Option<PathBuf>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match path {
        Some(path) => serializer.serialize_some(&path.to_string_lossy()),
        None => serializer.serialize_none(),
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
}

fn resolve_gitdir(root: &ApprovedRoot, relative: &Path) -> Option<PathBuf> {
    let entry = relative.join(".git");
    let fd = root
        .open_relative(&entry, OFlags::RDONLY | OFlags::NOATIME | OFlags::NONBLOCK)
        .ok()?;
    let stat = fs::fstat(&fd).ok()?;
    if FileType::from_raw_mode(stat.st_mode) == FileType::Directory {
        return Some(entry);
    }
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
        return None;
    }
    let mut bytes = Vec::new();
    File::from(fd).take(4097).read_to_end(&mut bytes).ok()?;
    if bytes.len() > 4096 || !bytes.starts_with(b"gitdir: ") {
        return None;
    }
    let value = bytes[8..].strip_suffix(b"\n").unwrap_or(&bytes[8..]);
    if value.is_empty() || value.contains(&b'\n') || value.contains(&0) {
        return None;
    }
    let raw = PathBuf::from(OsString::from_vec(value.to_vec()));
    let absolute = if raw.is_absolute() {
        raw
    } else {
        root.path().join(relative).join(raw)
    };
    let normalized = normalize_absolute(&absolute)?;
    let rel = normalized.strip_prefix(root.path()).ok()?;
    if !valid_relative(rel) {
        return None;
    }
    root.open_relative(rel, OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOATIME)
        .ok()?;
    Some(rel.to_path_buf())
}

fn normalize_absolute(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::RootDir => result.push("/"),
            Component::Normal(part) => result.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                if !result.pop() {
                    return None;
                }
            }
            Component::Prefix(_) => return None,
        }
    }
    Some(result)
}

fn preflight_metadata(root: &ApprovedRoot, gitdir: &Path) -> bool {
    let Some(common) = common_gitdir(root, gitdir) else {
        return false;
    };
    if !regular_file(root, &common.join("config"))
        || !regular_file(root, &gitdir.join("HEAD"))
        || !regular_file_or_missing(root, &gitdir.join("config"))
        || !regular_file_or_missing(root, &gitdir.join("config.worktree"))
        || !regular_file_or_missing(root, &common.join("packed-refs"))
        || !safe_directory_tree(root, &common.join("refs"), 0, &mut 0)
        || !safe_directory_tree(root, &common.join("worktrees"), 0, &mut 0)
    {
        return false;
    }
    true
}

fn submodule_state(
    root: &ApprovedRoot,
    relative_worktree: &Path,
    worktree: &Path,
    control: &ScanControl,
    bwrap: &Path,
    git: &Path,
) -> Result<bool, String> {
    match root.open_relative(&relative_worktree.join(".gitmodules"), OFlags::PATH) {
        Ok(_) => return Ok(true),
        Err(Errno::NOENT) => {}
        Err(error) => return Err(format!(".gitmodules não validável: {error}")),
    }
    let output = run_git(
        root,
        worktree,
        &["ls-files", "--stage", "-z"],
        control,
        bwrap,
        git,
    )?;
    parse_index_for_gitlinks(&output)
}

fn parse_index_for_gitlinks(bytes: &[u8]) -> Result<bool, String> {
    for record in bytes.split(|byte| *byte == 0) {
        if record.is_empty() {
            continue;
        }
        let tab = record
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or("Entrada do index sem separador")?;
        let mut fields = record[..tab].split(|byte| *byte == b' ');
        let mode = fields.next().ok_or("Modo do index ausente")?;
        let object = fields.next().ok_or("Objeto do index ausente")?;
        let stage = fields.next().ok_or("Stage do index ausente")?;
        if fields.next().is_some()
            || !(object.len() == 40 || object.len() == 64)
            || !object.iter().all(u8::is_ascii_hexdigit)
            || !matches!(stage, b"0" | b"1" | b"2" | b"3")
        {
            return Err("Entrada do index malformada".into());
        }
        if mode == b"160000" {
            return Ok(true);
        }
        if !matches!(mode, b"100644" | b"100755" | b"120000") {
            return Err("Modo do index desconhecido".into());
        }
    }
    Ok(false)
}

fn common_gitdir(root: &ApprovedRoot, gitdir: &Path) -> Option<PathBuf> {
    let path = gitdir.join("commondir");
    let fd = match root.open_relative(&path, OFlags::RDONLY | OFlags::NOATIME | OFlags::NONBLOCK) {
        Ok(fd) => fd,
        Err(Errno::NOENT) => return Some(gitdir.to_path_buf()),
        Err(_) => return None,
    };
    let mut bytes = Vec::new();
    File::from(fd).take(4097).read_to_end(&mut bytes).ok()?;
    if bytes.len() > 4096 {
        return None;
    }
    let value = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
    if value.is_empty() || value.contains(&b'\n') || value.contains(&0) {
        return None;
    }
    let raw = PathBuf::from(OsString::from_vec(value.to_vec()));
    let absolute = if raw.is_absolute() {
        raw
    } else {
        root.path().join(gitdir).join(raw)
    };
    let normalized = normalize_absolute(&absolute)?;
    let relative = normalized.strip_prefix(root.path()).ok()?;
    if !valid_relative(relative) {
        return None;
    }
    root.open_relative(
        relative,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOATIME,
    )
    .ok()?;
    Some(relative.to_path_buf())
}

fn regular_file(root: &ApprovedRoot, path: &Path) -> bool {
    match root.open_relative(path, OFlags::RDONLY | OFlags::NOATIME | OFlags::NONBLOCK) {
        Ok(fd) => fs::fstat(&fd)
            .is_ok_and(|stat| FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile),
        Err(_) => false,
    }
}

fn regular_file_or_missing(root: &ApprovedRoot, path: &Path) -> bool {
    match root.open_relative(path, OFlags::RDONLY | OFlags::NOATIME | OFlags::NONBLOCK) {
        Ok(fd) => fs::fstat(&fd)
            .is_ok_and(|stat| FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile),
        Err(Errno::NOENT) => true,
        Err(_) => false,
    }
}

fn safe_directory_tree(root: &ApprovedRoot, path: &Path, depth: usize, count: &mut usize) -> bool {
    if depth > MAX_REF_DEPTH || *count > MAX_REF_ENTRIES {
        return false;
    }
    let Ok(fd) = root.open_relative(path, OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOATIME)
    else {
        return matches!(
            root.open_relative(path, OFlags::RDONLY | OFlags::NOATIME | OFlags::NONBLOCK),
            Err(Errno::NOENT)
        );
    };
    let mut buffer = [std::mem::MaybeUninit::uninit(); 8192];
    let mut entries = RawDir::new(&fd, &mut buffer);
    while let Some(entry) = entries.next() {
        let Ok(entry) = entry else { return false };
        let name = entry.file_name().to_bytes();
        if name == b"." || name == b".." {
            continue;
        }
        *count += 1;
        if *count > MAX_REF_ENTRIES {
            return false;
        }
        let child = path.join(OsStr::from_bytes(name));
        let Ok(child_fd) = root.open_relative(&child, OFlags::PATH) else {
            return false;
        };
        let Ok(stat) = fs::fstat(&child_fd) else {
            return false;
        };
        match FileType::from_raw_mode(stat.st_mode) {
            FileType::Directory => {
                if !safe_directory_tree(root, &child, depth + 1, count) {
                    return false;
                }
            }
            FileType::RegularFile => {}
            _ => return false,
        }
    }
    true
}

fn has_nested_mount(root: &Path) -> bool {
    let Ok(file) = File::open("/proc/self/mountinfo") else {
        return true;
    };
    let mut bytes = Vec::new();
    if file
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() > 4 * 1024 * 1024
    {
        return true;
    }
    mountinfo_has_nested_mount(root, &bytes)
}

fn mountinfo_has_nested_mount(root: &Path, bytes: &[u8]) -> bool {
    for line in bytes.split(|byte| *byte == b'\n') {
        if line.is_empty() {
            continue;
        }
        let Some(mount) = line.split(|byte| *byte == b' ').nth(4) else {
            return true;
        };
        let Some(decoded) = decode_mount_path(mount) else {
            return true;
        };
        let path = PathBuf::from(OsString::from_vec(decoded));
        if path != root && path.starts_with(root) {
            return true;
        }
    }
    false
}

fn sandbox_block_reason(
    nested_mount: bool,
    bubblewrap_available: bool,
    git_available: bool,
) -> Option<&'static str> {
    if nested_mount {
        Some("Há um mount abaixo da raiz aprovada; sandbox recusada.")
    } else if !bubblewrap_available || !git_available {
        Some("Bubblewrap ou Git confiável indisponível.")
    } else {
        None
    }
}

fn decode_mount_path(input: &[u8]) -> Option<Vec<u8>> {
    let mut output = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'\\' {
            if i + 3 >= input.len() {
                return None;
            }
            let octal = std::str::from_utf8(&input[i + 1..i + 4]).ok()?;
            output.push(u8::from_str_radix(octal, 8).ok()?);
            i += 4;
        } else {
            output.push(input[i]);
            i += 1;
        }
    }
    Some(output)
}

/// Specific Portuguese reason for a failed sandboxed Git query. The repository
/// stays protected either way; only the explanation changes.
fn git_failure(code: Option<i32>, stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let first = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_control())
        .take(160)
        .collect::<String>();
    if first.starts_with("bwrap:") {
        return format!(
            "Sandbox somente leitura do Git (bubblewrap) indisponível neste ambiente, por exemplo em contêiner sem permissão para montar /proc; repositório preservado sem inspeção. Detalhe: {}",
            crate::scan::os_error_pt(first.trim_start_matches("bwrap:").trim())
        );
    }
    let code = code.map_or_else(|| "sinal".to_owned(), |c| c.to_string());
    if first.is_empty() {
        format!("Git terminou com erro (código {code}) sem diagnóstico; repositório preservado.")
    } else {
        format!("Git terminou com erro (código {code}): {first}. Repositório preservado.")
    }
}

fn run_git(
    root: &ApprovedRoot,
    cwd: &Path,
    args: &[&str],
    control: &ScanControl,
    bwrap: &Path,
    git: &Path,
) -> Result<Vec<u8>, String> {
    if !control.checkpoint() {
        return Err("Inspeção cancelada.".into());
    }
    let fd = root
        .directory_fd()
        .try_clone_to_owned()
        .map_err(|e| format!("Não foi possível duplicar o descritor aprovado: {e}"))?;
    let mut command = Command::new(bwrap);
    command
        .env_clear()
        .env("PATH", "/usr/bin")
        .env("HOME", "/")
        .env("LC_ALL", "C")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .env("PAGER", "cat")
        .env("GIT_NO_LAZY_FETCH", "1")
        .stdin(Stdio::from(fd))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg("--die-with-parent")
        .arg("--unshare-all")
        .arg("--ro-bind")
        .arg("/usr")
        .arg("/usr")
        .arg("--ro-bind-try")
        .arg("/lib")
        .arg("/lib")
        .arg("--ro-bind-try")
        .arg("/lib64")
        .arg("/lib64")
        .arg("--dev")
        .arg("/dev")
        .arg("--proc")
        .arg("/proc")
        .arg("--ro-bind-fd")
        .arg("0")
        .arg(root.path())
        .arg("--chdir")
        .arg(cwd)
        .arg(git)
        .arg("--no-optional-locks")
        .arg("-c")
        .arg("core.fsmonitor=false")
        .arg("-c")
        .arg("core.untrackedCache=false")
        .arg("-c")
        .arg("core.hooksPath=/dev/null")
        .args(args);
    let mut child = command
        .spawn()
        .map_err(|e| format!("Sandbox Git indisponível: {e}"))?;
    bounded_wait(&mut child, control)
}

fn bounded_wait(child: &mut Child, control: &ScanControl) -> Result<Vec<u8>, String> {
    bounded_wait_with_timeout(child, control, COMMAND_TIMEOUT)
}

fn bounded_wait_with_timeout(
    child: &mut Child,
    control: &ScanControl,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    let stdout = child.stdout.take().ok_or("stdout Git indisponível")?;
    let stderr = child.stderr.take().ok_or("stderr Git indisponível")?;
    let overflow = Arc::new(AtomicBool::new(false));
    let out_overflow = Arc::clone(&overflow);
    let err_overflow = Arc::clone(&overflow);
    let out = thread::spawn(move || read_limited(stdout, out_overflow));
    let err = thread::spawn(move || read_limited(stderr, err_overflow));
    let started = Instant::now();
    let mut cancelled = false;
    loop {
        if control.is_cancelled() {
            cancelled = true;
        }
        if cancelled || overflow.load(Ordering::Acquire) || started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = out.join();
            let _ = err.join();
            return Err(if cancelled {
                "Inspeção cancelada.".into()
            } else if overflow.load(Ordering::Acquire) {
                "Saída Git excedeu o limite.".into()
            } else {
                "Git excedeu o tempo limite.".into()
            });
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = out.join().map_err(|_| "Leitura da saída Git falhou")??;
                let stderr = err.join().map_err(|_| "Leitura do erro Git falhou")??;
                if overflow.load(Ordering::Acquire) {
                    return Err("Saída Git excedeu o limite.".into());
                }
                if !status.success() {
                    return Err(git_failure(status.code(), &stderr));
                }
                if !stderr.is_empty() {
                    return Err("Git emitiu diagnóstico; estado é incerto.".into());
                }
                return Ok(stdout);
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("Não foi possível acompanhar Git: {error}"));
            }
        }
    }
}

fn read_limited(mut reader: impl Read, overflow: Arc<AtomicBool>) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        let room = MAX_COMMAND_OUTPUT.saturating_sub(output.len());
        output.extend_from_slice(&buffer[..count.min(room)]);
        if count > room {
            overflow.store(true, Ordering::Release);
        }
    }
    Ok(output)
}

#[derive(Default)]
struct ListedWorktree {
    path: Option<PathBuf>,
    head: Option<String>,
    branch: Option<String>,
    detached: bool,
    locked: bool,
    prunable: bool,
    seen: HashSet<Vec<u8>>,
}

fn parse_worktrees(bytes: &[u8]) -> Result<Vec<ListedWorktree>, ()> {
    let mut result = Vec::new();
    let mut current = ListedWorktree::default();
    let mut has_fields = false;
    for field in bytes.split(|byte| *byte == 0) {
        if field.is_empty() {
            if has_fields {
                if current.path.is_none() || current.head.is_none() {
                    return Err(());
                }
                result.push(current);
                current = ListedWorktree::default();
                has_fields = false;
                if result.len() > MAX_WORKTREES {
                    return Err(());
                }
            }
            continue;
        }
        has_fields = true;
        let (key, value) = match field.iter().position(|byte| *byte == b' ') {
            Some(i) => (&field[..i], Some(&field[i + 1..])),
            None => (field, None),
        };
        if !current.seen.insert(key.to_vec()) {
            return Err(());
        }
        match key {
            b"worktree" => {
                current.path = Some(PathBuf::from(OsString::from_vec(value.ok_or(())?.to_vec())))
            }
            b"HEAD" => {
                current.head = Some(
                    std::str::from_utf8(value.ok_or(())?)
                        .map_err(|_| ())?
                        .to_owned(),
                )
            }
            b"branch" => {
                current.branch = Some(
                    std::str::from_utf8(value.ok_or(())?)
                        .map_err(|_| ())?
                        .to_owned(),
                )
            }
            b"detached" if value.is_none() => current.detached = true,
            b"locked" => current.locked = true,
            b"prunable" => current.prunable = true,
            b"bare" => return Err(()),
            _ => return Err(()),
        }
    }
    if has_fields {
        return Err(());
    }
    Ok(result)
}

#[derive(Default)]
struct StatusResult {
    branch_head: Option<String>,
    upstream: Option<String>,
    ahead: Option<u64>,
    behind: Option<u64>,
    dirty: bool,
    untracked: bool,
    ignored: bool,
}

fn parse_status(bytes: &[u8]) -> Result<StatusResult, String> {
    let mut result = StatusResult::default();
    let mut known_headers = 0usize;
    let mut seen = HashSet::new();
    for record in bytes.split(|byte| *byte == 0) {
        if record.is_empty() {
            continue;
        }
        if !record.starts_with(b"# ") {
            result.dirty = true;
            if record.starts_with(b"? ") {
                result.untracked = true;
            }
            if record.starts_with(b"! ") {
                result.ignored = true;
            }
            continue;
        }
        let line = record;
        let line = line
            .strip_prefix(b"# ")
            .ok_or("Cabeçalho status inesperado")?;
        if let Some(value) = line.strip_prefix(b"branch.head ") {
            if value.is_empty() || !seen.insert(b"branch.head".to_vec()) {
                return Err("Branch ausente ou duplicada".into());
            }
            result.branch_head = Some(
                std::str::from_utf8(value)
                    .map_err(|_| "Branch inválida")?
                    .to_owned(),
            );
            known_headers += 1;
        } else if let Some(value) = line.strip_prefix(b"branch.upstream ") {
            if value.is_empty() || !seen.insert(b"branch.upstream".to_vec()) {
                return Err("Upstream ausente ou duplicado".into());
            }
            result.upstream = Some(
                std::str::from_utf8(value)
                    .map_err(|_| "Upstream inválido")?
                    .to_owned(),
            );
            known_headers += 1;
        } else if let Some(value) = line.strip_prefix(b"branch.ab ") {
            if !seen.insert(b"branch.ab".to_vec()) {
                return Err("branch.ab duplicado".into());
            }
            let mut fields = value.split(|byte| *byte == b' ');
            let ahead = fields
                .next()
                .and_then(|value| parse_signed_count(value, b'+'))
                .ok_or("Ahead inválido")?;
            let behind = fields
                .next()
                .and_then(|value| parse_signed_count(value, b'-'))
                .ok_or("Behind inválido")?;
            if fields.next().is_some() {
                return Err("branch.ab inválido".into());
            }
            result.ahead = Some(ahead);
            result.behind = Some(behind);
            known_headers += 1;
        } else if line.starts_with(b"branch.oid ") {
            if !seen.insert(b"branch.oid".to_vec()) {
                return Err("branch.oid duplicado".into());
            }
            known_headers += 1;
        } else {
            return Err("Cabeçalho status desconhecido".into());
        }
    }
    if known_headers < 2 {
        return Err("Cabeçalho status incompleto".into());
    }
    Ok(result)
}

fn parse_signed_count(value: &[u8], sign: u8) -> Option<u64> {
    let digits = value.get(1..)?;
    if value.first() != Some(&sign) {
        return None;
    }
    std::str::from_utf8(digits).ok()?.parse().ok()
}

#[cfg(test)]
mod git_failure_tests {
    #[test]
    fn sandbox_and_git_failures_are_explained() {
        let sandbox = super::git_failure(
            Some(1),
            b"bwrap: Can't mount proc on /newroot/proc: Operation not permitted\n",
        );
        assert!(sandbox.contains("bubblewrap"), "{sandbox}");
        assert!(!sandbox.contains("Git retornou erro"));
        let git = super::git_failure(Some(128), b"fatal: not a git repository\n");
        assert!(git.contains("código 128") && git.contains("fatal: not a git repository"));
        assert!(super::git_failure(None, b"").contains("sem diagnóstico"));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::{ApprovedRoot, ScanControl, ScopeKind};
    use std::io::Write;
    use std::process::Command;
    use tempfile::TempDir;

    fn git(directory: &Path, args: &[&str]) -> bool {
        Command::new(GIT)
            .arg("-C")
            .arg(directory)
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin")
            .env("HOME", directory)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .status()
            .is_ok_and(|status| status.success())
    }

    fn commit(directory: &Path) {
        assert!(git(
            directory,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "add",
                "."
            ]
        ));
        assert!(git(
            directory,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-m",
                "fixture"
            ]
        ));
    }

    fn approved(path: &Path) -> ApprovedRoot {
        ApprovedRoot::open(path, ScopeKind::Projects).unwrap()
    }

    fn tracked_tree<'a>(inventory: &'a GitInventory, path: &Path) -> &'a GitWorktree {
        inventory
            .worktrees
            .iter()
            .find(|tree| tree.path == path)
            .unwrap_or_else(|| panic!("worktree ausente: {path:?}; inventário: {inventory:?}"))
    }

    fn append_config(path: &Path, contents: &[u8]) {
        std::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .unwrap()
            .write_all(contents)
            .unwrap();
    }

    #[test]
    fn tracked_agent_payload_is_excluded_from_content_status_checks() {
        use std::os::unix::fs::PermissionsExt;
        if !Path::new(BWRAP).exists() || !Path::new(GIT).exists() {
            return;
        }
        let temp = TempDir::new().unwrap();
        let repo = temp.path().join("repo");
        std::fs::create_dir(&repo).unwrap();
        assert!(git(&repo, &["init", "-q", "-b", "main"]));
        std::fs::create_dir(repo.join(".codex")).unwrap();
        let payload = repo.join(".codex/private.jsonl");
        std::fs::write(&payload, b"fixture session").unwrap();
        commit(&repo);
        std::fs::write(&payload, b"changed private fixture").unwrap();
        std::fs::set_permissions(&payload, std::fs::Permissions::from_mode(0o000)).unwrap();
        let inventory = inspect(
            &approved(temp.path()),
            Path::new("repo"),
            &ScanControl::default(),
        );
        std::fs::set_permissions(&payload, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(inventory.complete, "{inventory:?}");
        assert!(inventory.protected);
        assert!(!inventory.worktrees[0].dirty);
        assert!(inventory.worktrees[0]
            .reasons
            .iter()
            .any(|r| r.contains("namespaces protegidos")));
    }
    #[test]
    fn clean_linked_worktree_is_still_protected_for_no_upstream() {
        if !Path::new(BWRAP).exists() || !Path::new(GIT).exists() {
            return;
        }
        let temp = TempDir::new().unwrap();
        let repository = temp.path().join("repo");
        let linked = temp.path().join("linked");
        std::fs::create_dir(&repository).unwrap();
        assert!(git(&repository, &["init", "-q", "-b", "main"]));
        std::fs::write(repository.join("file.txt"), "tracked\n").unwrap();
        commit(&repository);
        assert!(git(
            &repository,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feature",
                linked.to_str().unwrap()
            ]
        ));

        let root = approved(temp.path());
        let inventory = inspect(&root, Path::new("repo"), &ScanControl::default());
        assert!(inventory.complete, "{inventory:?}");
        assert!(inventory.protected);
        assert_eq!(inventory.worktrees.len(), 2);
        assert!(inventory.worktrees[0].is_main);
        let linked_tree = inventory
            .worktrees
            .iter()
            .find(|tree| tree.path == linked)
            .unwrap();
        assert!(linked_tree.protected);
        assert!(
            linked_tree
                .reasons
                .iter()
                .any(|reason| reason.contains("upstream")),
            "{inventory:?}"
        );
    }

    #[test]
    fn dirty_and_untracked_worktree_is_protected() {
        if !Path::new(BWRAP).exists() || !Path::new(GIT).exists() {
            return;
        }
        let temp = TempDir::new().unwrap();
        let repository = temp.path().join("repo");
        let linked = temp.path().join("linked");
        std::fs::create_dir(&repository).unwrap();
        assert!(git(&repository, &["init", "-q", "-b", "main"]));
        std::fs::write(repository.join("file.txt"), "tracked\n").unwrap();
        std::fs::write(repository.join(".gitignore"), "ignored.txt\n").unwrap();
        commit(&repository);
        assert!(git(
            &repository,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feature",
                linked.to_str().unwrap()
            ]
        ));
        std::fs::write(linked.join("file.txt"), "changed\n").unwrap();
        std::fs::write(linked.join("new.txt"), "untracked\n").unwrap();
        std::fs::write(linked.join("ignored.txt"), "ignored\n").unwrap();

        let root = approved(temp.path());
        let inventory = inspect(&root, Path::new("repo"), &ScanControl::default());
        let linked_tree = inventory
            .worktrees
            .iter()
            .find(|tree| tree.path == linked)
            .unwrap();
        assert!(linked_tree.dirty, "{inventory:?}");
        assert!(linked_tree.untracked);
        assert!(linked_tree.ignored);
        assert!(linked_tree.protected);
    }

    #[test]
    fn table_fixture_reports_main_locked_detached_clean_tracking_and_unpublished() {
        if !Path::new(BWRAP).exists() || !Path::new(GIT).exists() {
            return;
        }
        let temp = TempDir::new().unwrap();
        let bare = temp.path().join("remote.git");
        let repository = temp.path().join("repo");
        let clean = temp.path().join("clean");
        let ahead = temp.path().join("ahead");
        let locked = temp.path().join("locked");
        let detached = temp.path().join("detached");
        std::fs::create_dir(&bare).unwrap();
        std::fs::create_dir(&repository).unwrap();
        assert!(git(
            &bare,
            &["init", "--bare", "-q", "--initial-branch=main"]
        ));
        assert!(git(&repository, &["init", "-q", "-b", "main"]));
        std::fs::write(repository.join("base.txt"), "base\n").unwrap();
        commit(&repository);
        assert!(git(
            &repository,
            &["remote", "add", "origin", bare.to_str().unwrap()]
        ));
        assert!(git(&repository, &["push", "-q", "-u", "origin", "main"]));

        for branch in ["clean-branch", "ahead-branch", "locked-branch"] {
            assert!(git(&repository, &["branch", branch]));
            assert!(git(&repository, &["push", "-q", "-u", "origin", branch]));
        }
        assert!(git(
            &repository,
            &[
                "worktree",
                "add",
                "-q",
                clean.to_str().unwrap(),
                "clean-branch"
            ]
        ));
        assert!(git(
            &repository,
            &[
                "worktree",
                "add",
                "-q",
                ahead.to_str().unwrap(),
                "ahead-branch"
            ]
        ));
        std::fs::write(ahead.join("local-only.txt"), "unpublished\n").unwrap();
        commit(&ahead);
        assert!(git(
            &repository,
            &[
                "worktree",
                "add",
                "-q",
                locked.to_str().unwrap(),
                "locked-branch"
            ]
        ));
        assert!(git(
            &repository,
            &["worktree", "lock", locked.to_str().unwrap()]
        ));
        assert!(git(
            &repository,
            &[
                "worktree",
                "add",
                "-q",
                "--detach",
                detached.to_str().unwrap(),
                "HEAD"
            ]
        ));

        let root = approved(temp.path());
        let inventory = inspect(&root, Path::new("repo"), &ScanControl::default());
        assert!(inventory.complete, "{inventory:?}");
        assert!(tracked_tree(&inventory, &repository).is_main);
        assert_eq!(
            tracked_tree(&inventory, &repository)
                .relative_path
                .as_deref(),
            Some(Path::new("repo"))
        );
        let clean_tree = tracked_tree(&inventory, &clean);
        assert!(!clean_tree.dirty && !clean_tree.untracked);
        assert_eq!(clean_tree.ahead, Some(0));
        assert!(clean_tree.upstream.is_some());
        let ahead_tree = tracked_tree(&inventory, &ahead);
        assert_eq!(ahead_tree.ahead, Some(1));
        assert!(ahead_tree.protected);
        let locked_tree = tracked_tree(&inventory, &locked);
        assert!(locked_tree.locked && locked_tree.protected);
        let detached_tree = tracked_tree(&inventory, &detached);
        assert!(detached_tree.detached && detached_tree.protected);
    }

    #[test]
    fn missing_bubblewrap_protects_without_falling_back_to_host_git() {
        let temp = TempDir::new().unwrap();
        let repository = temp.path().join("repo");
        std::fs::create_dir(&repository).unwrap();
        assert!(git(&repository, &["init", "-q", "-b", "main"]));
        let root = approved(temp.path());
        let missing = temp.path().join("not-installed-bwrap");
        let inventory = inspect_with_tools(
            &root,
            Path::new("repo"),
            &ScanControl::default(),
            &missing,
            Path::new(GIT),
        );
        assert!(inventory.protected);
        assert!(inventory.worktrees.is_empty());
        assert_eq!(
            inventory.reasons,
            ["Bubblewrap ou Git confiável indisponível."]
        );
    }

    #[test]
    fn submodule_manifest_is_explicitly_protected_when_status_ignores_submodules() {
        if !Path::new(BWRAP).exists() || !Path::new(GIT).exists() {
            return;
        }
        let temp = TempDir::new().unwrap();
        let repository = temp.path().join("repo");
        std::fs::create_dir(&repository).unwrap();
        assert!(git(&repository, &["init", "-q", "-b", "main"]));
        std::fs::write(repository.join("tracked.txt"), "tracked\n").unwrap();
        commit(&repository);
        std::fs::write(
            repository.join(".gitmodules"),
            b"[submodule \"fixture\"]\n\tpath = modules/fixture\n\turl = ../fixture.git\n",
        )
        .unwrap();
        commit(&repository);

        let root = approved(temp.path());
        let inventory = inspect(&root, Path::new("repo"), &ScanControl::default());
        let main = tracked_tree(&inventory, &repository);
        assert!(main.submodule_state_unknown && main.protected);
        assert!(main
            .reasons
            .iter()
            .any(|reason| reason.contains("Submódulos")));
    }

    #[test]
    fn pre_cancelled_inventory_stops_before_git() {
        let temp = TempDir::new().unwrap();
        let root = approved(temp.path());
        let control = ScanControl::default();
        control.cancel();
        let inventory = inspect(&root, Path::new("repo"), &control);
        assert!(inventory.protected && inventory.cancelled);
        assert!(!inventory.complete);
        assert!(inventory.worktrees.is_empty());
    }

    #[test]
    fn nested_mount_preflight_protects_and_reports_the_sandbox_reason() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("approved");
        let nested = root.join("mount");
        let mountinfo = format!("41 1 0:41 / {} rw - tmpfs tmpfs rw\n", nested.display());
        assert!(mountinfo_has_nested_mount(&root, mountinfo.as_bytes()));
        assert_eq!(
            sandbox_block_reason(true, true, true),
            Some("Há um mount abaixo da raiz aprovada; sandbox recusada.")
        );
    }

    #[test]
    fn subprocess_timeout_kills_the_child() {
        let temp = TempDir::new().unwrap();
        if !Path::new("/usr/bin/sleep").exists() {
            return;
        }
        let mut child = Command::new("/usr/bin/sleep")
            .arg("5")
            .current_dir(temp.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let result = bounded_wait_with_timeout(
            &mut child,
            &ScanControl::default(),
            Duration::from_millis(40),
        );
        assert!(result.unwrap_err().contains("tempo limite"));
    }

    #[test]
    fn outside_include_and_fsmonitor_helper_cannot_read_or_write_sibling_fixtures() {
        if !Path::new(BWRAP).exists() || !Path::new(GIT).exists() {
            return;
        }
        let temp = TempDir::new().unwrap();
        let approved_dir = temp.path().join("approved");
        let escape = temp.path().join("escape");
        let repository = approved_dir.join("repo");
        std::fs::create_dir(&approved_dir).unwrap();
        std::fs::create_dir(&escape).unwrap();
        std::fs::create_dir(&repository).unwrap();
        assert!(git(&repository, &["init", "-q", "-b", "main"]));
        std::fs::write(repository.join("tracked.txt"), "tracked\n").unwrap();
        commit(&repository);
        std::fs::write(repository.join("sentinel.txt"), "untracked\n").unwrap();
        std::fs::write(escape.join("exclude"), "sentinel.txt\n").unwrap();
        let helper = escape.join("helper");
        std::fs::write(
            &helper,
            b"#!/usr/bin/sh\nprintf invoked > /proc/self/fd/0/../escape/executed\nprintf 'token\\n'\n",
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&helper).unwrap().permissions();
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(0o700);
        std::fs::set_permissions(&helper, permissions).unwrap();
        std::fs::write(
            escape.join("included-config"),
            b"[core]\n\texcludesFile = /proc/self/fd/0/../escape/exclude\n",
        )
        .unwrap();
        append_config(
            &repository.join(".git/config"),
            b"\n[include]\n\tpath = /proc/self/fd/0/../escape/included-config\n[core]\n\tfsmonitor = /proc/self/fd/0/../escape/helper\n",
        );

        let root = approved(&approved_dir);
        let inventory = inspect(&root, Path::new("repo"), &ScanControl::default());
        assert!(inventory.protected);
        assert!(
            inventory.worktrees.iter().all(|tree| !tree.ignored),
            "{inventory:?}"
        );
        assert!(!escape.join("executed").exists());
    }

    #[test]
    fn sandboxed_git_cannot_follow_stdin_fd_to_an_external_fixture() {
        if !Path::new(BWRAP).exists() || !Path::new("/usr/bin/python3").exists() {
            return;
        }
        let temp = TempDir::new().unwrap();
        let approved = temp.path().join("approved");
        let escape = temp.path().join("escape");
        std::fs::create_dir(&approved).unwrap();
        std::fs::create_dir(&escape).unwrap();
        std::fs::write(escape.join("sentinel"), "SECRET_SENTINEL").unwrap();
        let root_fd = File::open(&approved).unwrap();
        let result = Command::new(BWRAP)
            .env_clear()
            .stdin(Stdio::from(root_fd))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .args([
                "--die-with-parent",
                "--unshare-all",
                "--ro-bind",
                "/usr",
                "/usr",
                "--ro-bind-try",
                "/lib",
                "/lib",
                "--ro-bind-try",
                "/lib64",
                "/lib64",
                "--dev",
                "/dev",
                "--proc",
                "/proc",
                "--ro-bind-fd",
                "0",
            ])
            .arg(&approved)
            .args([
                "--chdir",
                approved.to_str().unwrap(),
                "/usr/bin/python3",
                "-c",
                "p='/proc/self/fd/0/../escape/sentinel';\ntry: print(open(p).read())\nexcept OSError: print('DENIED')",
            ])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(result.stdout, b"DENIED\n");
    }

    #[test]
    fn outside_gitfile_target_is_protected_before_git_runs() {
        let container = TempDir::new().unwrap();
        let approved_dir = container.path().join("approved");
        let outside = container.path().join("outside");
        std::fs::create_dir(&approved_dir).unwrap();
        std::fs::create_dir(&outside).unwrap();
        std::fs::create_dir(approved_dir.join("repo")).unwrap();
        assert!(git(&outside, &["init", "-q"]));
        let pointer = format!("gitdir: {}\n", outside.join(".git").display());
        std::fs::write(approved_dir.join("repo/.git"), pointer).unwrap();
        let root = approved(&approved_dir);
        let inventory = inspect(&root, Path::new("repo"), &ScanControl::default());
        assert!(inventory.protected);
        assert!(inventory.worktrees.is_empty());
    }

    #[test]
    fn malformed_status_and_worktree_records_are_rejected() {
        assert!(parse_worktrees(b"worktree /tmp/a\0unknown thing\0\0").is_err());
        assert!(parse_status(b"# branch.oid 123\0# branch.head main\0# mystery value\0").is_err());
        let parsed = parse_status(b"# branch.oid 123\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\0? new\0").unwrap();
        assert_eq!(parsed.ahead, Some(2));
        assert_eq!(parsed.behind, Some(1));
        assert!(parsed.dirty && parsed.untracked);
        assert!(parse_index_for_gitlinks(
            b"160000 0123456789012345678901234567890123456789 0\tmodules/x\0"
        )
        .unwrap());
        assert!(!parse_index_for_gitlinks(
            b"100644 0123456789012345678901234567890123456789 0\tfile\0"
        )
        .unwrap());
        assert!(parse_index_for_gitlinks(b"malformed\0").is_err());
    }

    #[test]
    fn non_utf8_worktree_paths_keep_lossy_display_and_exact_hex_evidence() {
        let path = PathBuf::from(OsString::from_vec(b"repo/\xff".to_vec()));
        let tree = GitWorktree {
            path: path.clone(),
            relative_path: Some(path.clone()),
            path_bytes_hex: Some(hex_bytes(path.as_os_str().as_bytes())),
            relative_path_bytes_hex: Some(hex_bytes(path.as_os_str().as_bytes())),
            head: None,
            branch: None,
            upstream: None,
            ahead: None,
            behind: None,
            is_main: false,
            detached: false,
            locked: false,
            prunable: false,
            dirty: false,
            untracked: false,
            ignored: false,
            submodule_state_unknown: false,
            protected: true,
            reasons: vec!["unknown".into()],
        };
        let value = serde_json::to_value(tree).unwrap();
        assert_eq!(value["path_bytes_hex"], "7265706f2fff");
        assert_eq!(value["relative_path_bytes_hex"], "7265706f2fff");
    }
}
