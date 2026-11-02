//! Private, descriptor-relative storage. No symlink fallback or chmod of existing data.
use rustix::{
    fd::{AsFd, OwnedFd},
    fs::{self, AtFlags, FileType, Mode, OFlags, ResolveFlags},
    io::Errno,
};
use std::{
    io::Write,
    path::{Component, Path, PathBuf},
};

const MAX_HISTORY_ENTRIES: u64 = 200;
const MAX_HISTORY_BYTES: u64 = 256 * 1024 * 1024;
const HISTORY_HEADROOM: u64 = 64 * 1024 * 1024;
/// Writer and reader share this cap.
pub const MAX_JOURNAL_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_JOURNAL_RECORD: usize = 8 * 1024 * 1024;
const MAX_REPORT_BYTES: usize = 8 * 1024 * 1024;

pub(crate) fn open_dir(path: &Path, create: bool, private: bool) -> Result<OwnedFd, String> {
    open_dir_with(path, create, private, &inodune_core::access::SystemGroups)
}
fn open_dir_with(
    path: &Path,
    create: bool,
    private: bool,
    groups: &dyn inodune_core::access::GroupQuery,
) -> Result<OwnedFd, String> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("Armazenamento requer caminho absoluto sem '..'.".into());
    }
    let mut fd = fs::open(
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|e| e.to_string())?;
    check_storage_ancestor(&fd, groups)?;
    for part in path.components() {
        let Component::Normal(name) = part else {
            continue;
        };
        let next = fs::openat2(
            &fd,
            name,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        );
        fd = match next {
            Ok(next) => next,
            Err(Errno::NOENT) if create => {
                match fs::mkdirat(&fd, name, Mode::from_raw_mode(0o700)) {
                    Ok(()) | Err(Errno::EXIST) => {}
                    Err(e) => return Err(e.to_string()),
                };
                fs::openat2(
                    &fd,
                    name,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
                    Mode::empty(),
                    ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
                )
                .map_err(|e| e.to_string())?
            }
            Err(e) => {
                return Err(format!(
                    "Armazenamento inacessível, links não permitidos: {e}"
                ))
            }
        };
        check_storage_ancestor(&fd, groups)?;
    }
    if private {
        check_private_with(&fd, groups)?;
    }
    Ok(fd)
}

/// Like `open_dir`, but distinguishes a genuinely missing component from every
/// other failure so history listing never treats permission errors as empty.
pub(crate) fn open_dir_optional(path: &Path, private: bool) -> Result<Option<OwnedFd>, String> {
    open_dir_optional_with(path, private, &inodune_core::access::SystemGroups)
}
fn open_dir_optional_with(
    path: &Path,
    private: bool,
    groups: &dyn inodune_core::access::GroupQuery,
) -> Result<Option<OwnedFd>, String> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("Armazenamento requer caminho absoluto sem '..'.".into());
    }
    let mut fd = fs::open(
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|e| e.to_string())?;
    check_storage_ancestor(&fd, groups)?;
    for part in path.components() {
        let Component::Normal(name) = part else {
            continue;
        };
        let next = fs::openat2(
            &fd,
            name,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        );
        fd = match next {
            Ok(next) => next,
            Err(Errno::NOENT) => return Ok(None),
            Err(e) => {
                return Err(format!(
                    "Armazenamento inacessível, links não permitidos: {e}"
                ))
            }
        };
        check_storage_ancestor(&fd, groups)?;
    }
    if private {
        check_private_with(&fd, groups)?;
    }
    Ok(Some(fd))
}
fn check_storage_ancestor(
    fd: impl AsFd,
    groups: &dyn inodune_core::access::GroupQuery,
) -> Result<(), String> {
    let st = fs::fstat(fd).map_err(|e| e.to_string())?;
    // Root-owned sticky temporary directories protect an owned child against
    // cross-user renames. Other writable ancestors cannot provide that boundary.
    inodune_core::access::stat_write_allowed(
        inodune_core::access::WriteRole::StorageAncestor,
        st.st_uid,
        st.st_gid,
        st.st_mode,
        groups,
    )
    .map_err(|refusal| {
        format!(
            "Ancestral do armazenamento é gravável por terceiros ou tem proprietário inesperado. {}. Nenhuma permissão foi alterada.",
            inodune_core::access::describe_write_refusal(refusal)
        )
    })
}
pub(crate) fn open_child(fd: impl AsFd, name: &Path) -> Result<OwnedFd, Errno> {
    fs::openat2(
        fd,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH
            | ResolveFlags::NO_SYMLINKS
            | ResolveFlags::NO_MAGICLINKS
            | ResolveFlags::NO_XDEV,
    )
}
pub(crate) fn check_private(fd: impl AsFd) -> Result<(), String> {
    check_private_with(fd, &inodune_core::access::SystemGroups)
}
fn check_private_with(
    fd: impl AsFd,
    groups: &dyn inodune_core::access::GroupQuery,
) -> Result<(), String> {
    let st = fs::fstat(fd).map_err(|e| e.to_string())?;
    inodune_core::access::stat_write_allowed(
        inodune_core::access::WriteRole::PrivateStore,
        st.st_uid,
        st.st_gid,
        st.st_mode,
        groups,
    )
    .map_err(|refusal| {
        format!(
            "Armazenamento deve pertencer ao usuário, não pode ser gravável por outros e só aceita escrita de grupo quando o grupo é privado desse usuário. {}. Permissões existentes não foram alteradas.",
            inodune_core::access::describe_write_refusal(refusal)
        )
    })
}
fn child_private_with(
    fd: impl AsFd,
    name: &str,
    groups: &dyn inodune_core::access::GroupQuery,
) -> Result<OwnedFd, String> {
    match fs::mkdirat(fd.as_fd(), name, Mode::from_raw_mode(0o700)) {
        Ok(()) | Err(Errno::EXIST) => {}
        Err(e) => return Err(e.to_string()),
    };
    let child = open_child(fd, Path::new(name)).map_err(|e| e.to_string())?;
    check_private_with(&child, groups)?;
    Ok(child)
}
pub(crate) fn random_name(prefix: &str) -> Result<String, String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    Ok(format!(
        "{prefix}{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}
pub fn xdg_home(variable: &str, fallback: &str) -> Result<PathBuf, String> {
    std::env::var_os(variable)
        .filter(|p| !p.is_empty() && Path::new(p).is_absolute())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|p| Path::new(p).is_absolute())
                .map(|p| PathBuf::from(p).join(fallback))
        })
        .ok_or_else(|| format!("{variable}/HOME não disponível"))
}
/// Local-only report persistence. A caller supplies trusted backend configuration, never renderer paths.
pub fn save_report(
    directory: &Path,
    report: &crate::executor::ExecutionReport,
) -> Result<PathBuf, String> {
    let dir = open_dir(directory, true, true)?;
    check_capacity(&dir, MAX_REPORT_BYTES as u64, 1)?;
    let bytes = serde_json::to_vec_pretty(report).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_REPORT_BYTES {
        return Err("Relatório excede 8 MiB; execução marcada como incompleta e nenhum arquivo de relatório foi criado.".into());
    }
    let name = random_name("report-")? + ".json";
    write_new_atomic(&dir, &name, &bytes)?;
    Ok(directory.join(name))
}

/// Export only a revalidated presentation report to a native-picker destination.
/// Existing files, symlinks and writable directory ancestors are never replaced.
pub fn export_report(
    destination: &Path,
    report: &crate::executor::ExecutionReport,
) -> Result<(), String> {
    let parent = destination.parent().ok_or("Destino sem diretório pai.")?;
    let name = destination.file_name().ok_or("Nome de arquivo inválido.")?;
    let dir = open_dir(parent, false, false)?;
    let bytes = serde_json::to_vec_pretty(report).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_REPORT_BYTES {
        return Err("Relatório excede 8 MiB.".into());
    }
    write_new_atomic(&dir, name, &bytes)
}

fn write_new_atomic(
    dir: &OwnedFd,
    name: impl rustix::path::Arg,
    bytes: &[u8],
) -> Result<(), String> {
    // Pending files cannot match the history namespace. A crash can leave one
    // for manual inspection; neither export nor history silently removes it.
    let pending = random_name(".inodune-pending-")?;
    let fd = fs::openat(
        dir,
        &pending,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|e| e.to_string())?;
    let mut file = std::fs::File::from(fd);
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    fs::renameat_with(dir, &pending, dir, name, fs::RenameFlags::NOREPLACE).map_err(|e| {
        format!("Exportação não substitui arquivos existentes; arquivo pendente {pending}: {e}")
    })?;
    fs::fsync(dir).map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) struct Trash {
    pub files: OwnedFd,
    info: OwnedFd,
    path: PathBuf,
    identity: (u64, u64),
}
impl Trash {
    pub fn open(path: &Path) -> Result<Self, String> {
        Self::open_with(path, &inodune_core::access::SystemGroups)
    }
    pub(crate) fn open_with(
        path: &Path,
        groups: &dyn inodune_core::access::GroupQuery,
    ) -> Result<Self, String> {
        let root = open_dir_with(path, true, true, groups)?;
        let st = fs::fstat(&root).map_err(|e| e.to_string())?;
        Ok(Self {
            files: child_private_with(&root, "files", groups)?,
            info: child_private_with(&root, "info", groups)?,
            path: path.to_owned(),
            identity: (st.st_dev, st.st_ino),
        })
    }
    pub fn revalidate(&self, dev: u64) -> Result<(), String> {
        let fd = open_dir(&self.path, false, true)?;
        let st = fs::fstat(fd).map_err(|e| e.to_string())?;
        if (st.st_dev, st.st_ino) != self.identity || st.st_dev != dev {
            return Err("Lixeira mudou ou está em outro filesystem; nenhuma cópia ou exclusão alternativa será feita.".into());
        }
        for (name, expected) in [("files", &self.files), ("info", &self.info)] {
            let current = open_dir(&self.path.join(name), false, true)?;
            let a = fs::fstat(current).map_err(|e| e.to_string())?;
            let b = fs::fstat(expected).map_err(|e| e.to_string())?;
            if (a.st_dev, a.st_ino) != (b.st_dev, b.st_ino) {
                return Err("Subdiretório da Lixeira mudou.".into());
            }
        }
        Ok(())
    }
    pub fn reserve(&self, original: &Path) -> Result<String, String> {
        use std::os::unix::ffi::OsStrExt;
        let name = random_name("inodune-")?;
        let info_name = format!("{name}.trashinfo");
        let fd = fs::openat(
            &self.info,
            &info_name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o600),
        )
        .map_err(|e| e.to_string())?;
        let mut file = std::fs::File::from(fd);
        let encoded = original
            .as_os_str()
            .as_bytes()
            .iter()
            .map(|b| match *b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                    (*b as char).to_string()
                }
                _ => format!("%{b:02X}"),
            })
            .collect::<String>();
        let content = format!(
            "[Trash Info]\nPath={encoded}\nDeletionDate={}\n",
            chrono::Local::now().format("%Y-%m-%dT%H:%M:%S")
        );
        if let Err(e) = file
            .write_all(content.as_bytes())
            .and_then(|_| file.sync_all())
        {
            let _ = fs::unlinkat(&self.info, &info_name, fs::AtFlags::empty());
            return Err(e.to_string());
        }
        if let Err(e) = fs::fsync(&self.info) {
            let cleanup = fs::unlinkat(&self.info, &info_name, fs::AtFlags::empty());
            return Err(format!(
                "Falha ao sincronizar metadata da Lixeira: {e}; remoção da reserva: {cleanup:?}"
            ));
        }
        Ok(name)
    }
    pub fn abandon(&self, name: &str) -> Result<(), String> {
        fs::unlinkat(
            &self.info,
            format!("{name}.trashinfo"),
            fs::AtFlags::empty(),
        )
        .map_err(|e| e.to_string())
    }
    pub fn sync(&self) -> Result<(), String> {
        fs::fsync(&self.files)
            .and_then(|_| fs::fsync(&self.info))
            .map_err(|e| e.to_string())
    }
    pub fn destination(&self, name: &str) -> PathBuf {
        self.path.join("files").join(name)
    }
}

/// Append-only local journal. A durable intent precedes each filesystem mutation.
/// An interrupted intent means UNKNOWN, never inferred success or automatic rollback.
pub(crate) struct Journal {
    file: std::fs::File,
    dir: OwnedFd,
    pub path: PathBuf,
}
impl Journal {
    pub fn create(directory: &Path) -> Result<Self, String> {
        let dir = open_dir(directory, true, true)?;
        check_capacity(&dir, HISTORY_HEADROOM, 2)?;
        let name = random_name("journal-")? + ".jsonl";
        let fd = fs::openat(
            &dir,
            &name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o600),
        )
        .map_err(|e| e.to_string())?;
        fs::fsync(&dir).map_err(|e| e.to_string())?;
        Ok(Self {
            file: std::fs::File::from(fd),
            dir,
            path: directory.join(name),
        })
    }
    pub fn record(&mut self, value: &impl serde::Serialize) -> Result<(), String> {
        let mut bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        if bytes.len() > MAX_JOURNAL_RECORD {
            return Err(
                "Registro do journal excede 8 MiB; operações seguintes foram bloqueadas.".into(),
            );
        }
        let current = self.file.metadata().map_err(|e| e.to_string())?.len();
        if current.saturating_add(bytes.len() as u64) > MAX_JOURNAL_BYTES {
            return Err("Journal atingiu 64 MiB; operações seguintes foram bloqueadas. Arquive o histórico local para liberar capacidade.".into());
        }
        check_capacity(&self.dir, bytes.len() as u64, 0)?;
        self.file
            .write_all(&bytes)
            .and_then(|_| self.file.sync_all())
            .map_err(|e| e.to_string())
    }
}

pub fn probe_reports(directory: &Path) -> Result<(), String> {
    let dir = open_dir(directory, true, true)?;
    check_capacity(&dir, 1, 1)
}
fn check_capacity(dir: &OwnedFd, addition: u64, new_entries: u64) -> Result<(), String> {
    let mut count = 0u64;
    let mut total = 0u64;
    let mut buffer = [std::mem::MaybeUninit::uninit(); 8192];
    let mut entries = fs::RawDir::new(dir, &mut buffer);
    while let Some(entry) = entries.next() {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        if name.to_bytes() == b"." || name.to_bytes() == b".." {
            continue;
        }
        count = count.saturating_add(1);
        if count > MAX_HISTORY_ENTRIES {
            return Err(format!("Histórico local contém mais de {MAX_HISTORY_ENTRIES} entradas; arquive arquivos manualmente em XDG_STATE_HOME/inodune/reports."));
        }
        let stat = fs::statat(dir, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|e| e.to_string())?;
        if FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile {
            total = total.saturating_add(stat.st_size.max(0) as u64);
        }
    }
    if count.saturating_add(new_entries) > MAX_HISTORY_ENTRIES
        || total
            .saturating_add(addition)
            .saturating_add(HISTORY_HEADROOM)
            > MAX_HISTORY_BYTES
    {
        return Err(format!("Histórico local atingiu o limite de armazenamento (máximo {MAX_HISTORY_ENTRIES} entradas e 256 MiB, com 64 MiB reservados). Arquive manualmente arquivos em {} e tente novamente.", "XDG_STATE_HOME/inodune/reports"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use inodune_core::access::{GroupCensus, GroupQuery};
    use std::fs;
    struct FakeGroups {
        uid: u32,
        gid: u32,
        name: &'static str,
        census: Result<GroupCensus, inodune_core::access::GroupLookupError>,
    }
    impl GroupQuery for FakeGroups {
        fn uid_gid(&self) -> Result<(u32, u32), inodune_core::access::GroupLookupError> {
            Ok((self.uid, self.gid))
        }
        fn user_name(&self) -> Result<String, inodune_core::access::GroupLookupError> {
            Ok(self.name.into())
        }
        fn census(&self, _: u32) -> Result<GroupCensus, inodune_core::access::GroupLookupError> {
            self.census.clone()
        }
    }
    fn exclusive(uid: u32, gid: u32) -> FakeGroups {
        FakeGroups {
            uid,
            gid,
            name: "only",
            census: Ok(GroupCensus {
                name: "only".into(),
                supplementary: vec![],
                primary_users: vec!["only".into()],
            }),
        }
    }
    fn mode_of(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }
    #[test]
    fn trashinfo_percent_encodes_paths_and_existing_entries_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let trash = Trash::open(&dir.path().join("Trash")).unwrap();
        fs::write(dir.path().join("Trash/files/existing"), b"keep").unwrap();
        fs::write(
            dir.path().join("Trash/info/existing.trashinfo"),
            b"existing metadata",
        )
        .unwrap();
        let name = trash
            .reserve(Path::new("/tmp/projeto com espaço/%arquivo\n.pyc"))
            .unwrap();
        let value = fs::read_to_string(
            dir.path()
                .join("Trash/info")
                .join(format!("{name}.trashinfo")),
        )
        .unwrap();
        assert!(value.contains("Path=/tmp/projeto%20com%20espa%C3%A7o/%25arquivo%0A.pyc\n"));
        assert_eq!(value.lines().count(), 3);
        assert_eq!(
            fs::read(dir.path().join("Trash/files/existing")).unwrap(),
            b"keep"
        );
        trash.abandon(&name).unwrap();
        assert_eq!(
            fs::read(dir.path().join("Trash/info/existing.trashinfo")).unwrap(),
            b"existing metadata"
        );
    }
    #[test]
    fn writable_ancestor_is_rejected_before_creating_private_storage() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let shared = dir.path().join("shared");
        fs::create_dir(&shared).unwrap();
        fs::set_permissions(&shared, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(Trash::open(&shared.join("Trash")).is_err());
        assert!(!shared.join("Trash").exists());
    }
    #[test]
    fn symlinked_storage_and_public_existing_storage_are_rejected() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let dir = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        symlink(external.path(), dir.path().join("link")).unwrap();
        assert!(Trash::open(&dir.path().join("link/Trash")).is_err());
        assert_eq!(fs::read_dir(external.path()).unwrap().count(), 0);
        let public = dir.path().join("public");
        fs::create_dir(&public).unwrap();
        fs::set_permissions(&public, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(Trash::open(&public).is_err());
        assert_eq!(
            fs::metadata(public).unwrap().permissions().mode() & 0o777,
            0o777
        );
    }
    #[test]
    fn group_writable_ancestor_follows_the_private_group_rule() {
        use std::os::unix::fs::PermissionsExt;
        let uid = rustix::process::getuid().as_raw();
        let gid = rustix::process::getgid().as_raw();
        let dir = tempfile::tempdir().unwrap();
        let ancestor = dir.path().join("ancestor");
        let child = ancestor.join("reports");
        fs::create_dir(&ancestor).unwrap();
        fs::set_permissions(&ancestor, fs::Permissions::from_mode(0o775)).unwrap();
        assert!(open_dir_with(&child, true, true, &exclusive(uid, gid)).is_ok());
        assert_eq!(mode_of(&ancestor), 0o775);
        fs::remove_dir_all(&child).unwrap();
        let shared = FakeGroups {
            uid,
            gid,
            name: "only",
            census: Ok(GroupCensus {
                name: "only".into(),
                supplementary: vec!["guest".into()],
                primary_users: vec!["only".into()],
            }),
        };
        assert!(open_dir_with(&child, true, true, &shared).is_err());
        assert!(!child.exists());
        let unknown = FakeGroups {
            uid,
            gid,
            name: "only",
            census: Err(inodune_core::access::GroupLookupError),
        };
        assert!(open_dir_with(&child, true, true, &unknown).is_err());
        fs::set_permissions(&ancestor, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(open_dir_with(&child, true, true, &exclusive(uid, gid)).is_err());
        assert!(!child.exists());
    }
    #[test]
    fn existing_trash_may_be_group_writable_only_for_a_private_group_and_new_trash_stays_0700() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let uid = rustix::process::getuid().as_raw();
        let gid = rustix::process::getgid().as_raw();
        let dir = tempfile::tempdir().unwrap();
        let created = dir.path().join("created");
        let trash = Trash::open_with(&created, &exclusive(uid, gid)).unwrap();
        drop(trash);
        assert_eq!(mode_of(&created), 0o700);
        assert_eq!(mode_of(&created.join("files")), 0o700);
        assert_eq!(mode_of(&created.join("info")), 0o700);
        let existing = dir.path().join("existing");
        for name in ["", "files", "info"] {
            let path = if name.is_empty() {
                existing.clone()
            } else {
                existing.join(name)
            };
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o775)).unwrap();
        }
        assert!(Trash::open_with(&existing, &exclusive(uid, gid)).is_ok());
        assert_eq!(mode_of(&existing), 0o775);
        assert_eq!(mode_of(&existing.join("files")), 0o775);
        assert_eq!(mode_of(&existing.join("info")), 0o775);
        fs::set_permissions(existing.join("files"), fs::Permissions::from_mode(0o777)).unwrap();
        assert!(Trash::open_with(&existing, &exclusive(uid, gid)).is_err());
        assert_eq!(mode_of(&existing.join("files")), 0o777);
        fs::set_permissions(existing.join("files"), fs::Permissions::from_mode(0o775)).unwrap();
        let shared = FakeGroups {
            uid,
            gid,
            name: "only",
            census: Ok(GroupCensus {
                name: "only".into(),
                supplementary: vec![],
                primary_users: vec!["only".into(), "guest".into()],
            }),
        };
        assert!(Trash::open_with(&existing, &shared).is_err());
        assert_eq!(mode_of(&existing), 0o775);
        let link = dir.path().join("link");
        symlink(&existing, &link).unwrap();
        assert!(Trash::open_with(&link, &exclusive(uid, gid)).is_err());
    }
}
