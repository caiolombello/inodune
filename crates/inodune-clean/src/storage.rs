//! Private, descriptor-relative storage. No symlink fallback or chmod of existing data.
use rustix::{
    fd::{AsFd, OwnedFd},
    fs::{self, Mode, OFlags, ResolveFlags},
    io::Errno,
};
use std::{
    io::Write,
    path::{Component, Path, PathBuf},
};

pub(crate) fn open_dir(path: &Path, create: bool, private: bool) -> Result<OwnedFd, String> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("Armazenamento requer caminho absoluto sem '..'.".into());
    }
    let mut fd = fs::open(
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|e| e.to_string())?;
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
    }
    if private {
        check_private(&fd)?;
    }
    Ok(fd)
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
    let st = fs::fstat(fd).map_err(|e| e.to_string())?;
    if st.st_uid != rustix::process::getuid().as_raw() || st.st_mode & 0o077 != 0 {
        return Err("Armazenamento deve pertencer ao usuário e ser privado (0700). Permissões existentes não foram alteradas.".into());
    }
    Ok(())
}
pub(crate) fn child_private(fd: impl AsFd, name: &str) -> Result<OwnedFd, String> {
    match fs::mkdirat(fd.as_fd(), name, Mode::from_raw_mode(0o700)) {
        Ok(()) | Err(Errno::EXIST) => {}
        Err(e) => return Err(e.to_string()),
    };
    let child = open_child(fd, Path::new(name)).map_err(|e| e.to_string())?;
    check_private(&child)?;
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
    let name = random_name("report-")? + ".json";
    let fd = fs::openat(
        &dir,
        &name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|e| e.to_string())?;
    let mut file = std::fs::File::from(fd);
    let bytes = serde_json::to_vec_pretty(report).map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    fs::fsync(&dir).map_err(|e| e.to_string())?;
    Ok(directory.join(name))
}

pub(crate) struct Trash {
    pub files: OwnedFd,
    info: OwnedFd,
    path: PathBuf,
    identity: (u64, u64),
}
impl Trash {
    pub fn open(path: &Path) -> Result<Self, String> {
        let root = open_dir(path, true, true)?;
        let st = fs::fstat(&root).map_err(|e| e.to_string())?;
        Ok(Self {
            files: child_private(&root, "files")?,
            info: child_private(&root, "info")?,
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
    pub path: PathBuf,
}
impl Journal {
    pub fn create(directory: &Path) -> Result<Self, String> {
        let dir = open_dir(directory, true, true)?;
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
            path: directory.join(name),
        })
    }
    pub fn record(&mut self, value: &impl serde::Serialize) -> Result<(), String> {
        let mut bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        self.file
            .write_all(&bytes)
            .and_then(|_| self.file.sync_all())
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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
}
