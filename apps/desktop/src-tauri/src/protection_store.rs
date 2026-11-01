//! Private persistent cleanup policy. Paths here are revocations/protections,
//! never filesystem capabilities; roots are opened and verified on each scan.
#[cfg(any(not(test), feature = "fixture-tests"))]
use rustix::fs::FileType;
use rustix::{
    fd::OwnedFd,
    fs::{self, Mode, OFlags, RenameFlags, ResolveFlags},
    io::Errno,
};
use serde::{Deserialize, Serialize};
#[cfg(any(not(test), feature = "fixture-tests"))]
use std::io::Read;
#[cfg(all(test, feature = "fixture-tests"))]
use std::os::unix::fs::PermissionsExt;
use std::{
    io::Write,
    path::{Component, Path, PathBuf},
};

const MAX_POLICY_BYTES: usize = 1024 * 1024;
const MAX_POLICY_PATHS: usize = 512;
const MAX_PATH_BYTES: usize = 4096;
const STORE_DIR: &str = "inodune";
const STORE_FILE: &str = "protection-v1.json";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Policy {
    version: u8,
    pub(super) revoked_roots: Vec<PathBuf>,
    pub(super) protected_roots: Vec<PathBuf>,
    pub(super) exclusions: Vec<PersistedExclusion>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PersistedExclusion {
    pub(super) root: PathBuf,
    pub(super) relative_paths: Vec<PathBuf>,
}

pub(super) struct ProtectionStore {
    config_dir: PathBuf,
}

impl ProtectionStore {
    #[cfg(not(test))]
    pub(super) fn from_env() -> Result<Self, String> {
        let config_dir = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|value| !value.is_empty() && Path::new(value).is_absolute())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .filter(|value| !value.is_empty() && Path::new(value).is_absolute())
                    .map(|value| PathBuf::from(value).join(".config"))
            })
            .ok_or_else(|| {
                "Não foi possível localizar a configuração local do Inodune.".to_string()
            })?;
        Self::at(config_dir)
    }

    #[cfg(any(not(test), feature = "fixture-tests"))]
    pub(super) fn at(config_dir: PathBuf) -> Result<Self, String> {
        if !config_dir.is_absolute()
            || config_dir
                .components()
                .any(|part| matches!(part, Component::ParentDir))
        {
            return Err("Pasta de configuração inválida.".into());
        }
        Ok(Self { config_dir })
    }

    #[cfg(any(not(test), feature = "fixture-tests"))]
    pub(super) fn load(&self) -> Result<Policy, String> {
        let directory = match self.open_policy_dir(false) {
            Ok(directory) => directory,
            Err(Errno::NOENT) => return Ok(Policy::new()),
            Err(error) => {
                return Err(format!(
                    "Não foi possível abrir a política de proteção local: {error}"
                ))
            }
        };
        let file = match fs::openat2(
            &directory,
            STORE_FILE,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
            ResolveFlags::BENEATH
                | ResolveFlags::NO_SYMLINKS
                | ResolveFlags::NO_MAGICLINKS
                | ResolveFlags::NO_XDEV,
        ) {
            Ok(file) => file,
            Err(Errno::NOENT) => return Ok(Policy::new()),
            Err(error) => {
                return Err(format!(
                    "A política de proteção não pôde ser lida com segurança: {error}"
                ))
            }
        };
        let metadata = fs::fstat(&file).map_err(|error| error.to_string())?;
        let uid = rustix::process::getuid().as_raw();
        if FileType::from_raw_mode(metadata.st_mode) != FileType::RegularFile
            || metadata.st_uid != uid
            || metadata.st_mode & 0o077 != 0
            || metadata.st_nlink != 1
        {
            return Err("A política de proteção precisa pertencer ao usuário atual, ter modo 0600 e link único.".into());
        }
        if metadata.st_size < 0 || metadata.st_size as usize > MAX_POLICY_BYTES {
            return Err("A política de proteção excede o limite de 1 MiB.".into());
        }
        let mut bytes = Vec::with_capacity(metadata.st_size as usize);
        let mut reader = std::fs::File::from(file);
        (&mut reader)
            .take((MAX_POLICY_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("Falha ao ler a política de proteção: {error}"))?;
        let after_read = fs::fstat(&reader).map_err(|error| error.to_string())?;
        if (
            metadata.st_dev,
            metadata.st_ino,
            metadata.st_size,
            metadata.st_mtime,
            metadata.st_mtime_nsec,
            metadata.st_ctime,
            metadata.st_ctime_nsec,
        ) != (
            after_read.st_dev,
            after_read.st_ino,
            after_read.st_size,
            after_read.st_mtime,
            after_read.st_mtime_nsec,
            after_read.st_ctime,
            after_read.st_ctime_nsec,
        ) || bytes.len() != metadata.st_size as usize
        {
            return Err(
                "A política de proteção mudou durante a leitura; análise bloqueada.".into(),
            );
        }
        if bytes.len() > MAX_POLICY_BYTES {
            return Err("A política de proteção excede o limite de 1 MiB.".into());
        }
        let policy: Policy = serde_json::from_slice(&bytes).map_err(|error| {
            format!("A política de proteção está inválida; análise bloqueada: {error}")
        })?;
        validate_policy(&policy)?;
        Ok(policy)
    }

    pub(super) fn save(&self, policy: &Policy) -> Result<(), String> {
        validate_policy(policy)?;
        let directory = self.open_policy_dir(true).map_err(|error| {
            format!("Não foi possível abrir o armazenamento privado de proteção: {error}")
        })?;
        let bytes = serde_json::to_vec(policy).map_err(|error| error.to_string())?;
        if bytes.len() > MAX_POLICY_BYTES {
            return Err(
                "A política de proteção excede o limite de 1 MiB; alteração não aplicada.".into(),
            );
        }
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|error| error.to_string())?;
        let staging = format!(
            ".protection-{}.tmp",
            random
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        let fd = fs::openat(
            &directory,
            staging.as_str(),
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o600),
        )
        .map_err(|error| format!("Não foi possível criar a política pendente: {error}"))?;
        let mut file = std::fs::File::from(fd);
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("Não foi possível gravar a política de proteção: {error}"))?;
        fs::renameat_with(
            &directory,
            staging.as_str(),
            &directory,
            STORE_FILE,
            RenameFlags::empty(),
        )
        .map_err(|error| {
            format!("Não foi possível publicar atomicamente a política de proteção: {error}")
        })?;
        fs::fsync(&directory).map_err(|error| {
            format!("Não foi possível sincronizar a política de proteção: {error}")
        })?;
        Ok(())
    }

    fn open_policy_dir(&self, create: bool) -> Result<OwnedFd, Errno> {
        let mut current = fs::open(
            "/",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )?;
        let components = self.config_dir.components().collect::<Vec<_>>();
        for (index, component) in components.iter().enumerate() {
            let Component::Normal(name) = *component else {
                continue;
            };
            let next = fs::openat2(
                &current,
                name,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
                ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
            );
            current = match next {
                Ok(next) => next,
                Err(Errno::NOENT) if create && index + 1 == components.len() => {
                    match fs::mkdirat(&current, name, Mode::from_raw_mode(0o700)) {
                        Ok(()) | Err(Errno::EXIST) => {}
                        Err(error) => return Err(error),
                    }
                    fs::openat2(
                        &current,
                        name,
                        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                        Mode::empty(),
                        ResolveFlags::BENEATH
                            | ResolveFlags::NO_SYMLINKS
                            | ResolveFlags::NO_MAGICLINKS,
                    )?
                }
                Err(error) => return Err(error),
            };
            check_ancestor(&current)?;
        }
        let app_dir = match open_dir_at(&current, STORE_DIR) {
            Ok(directory) => directory,
            Err(Errno::NOENT) if create => {
                match fs::mkdirat(&current, STORE_DIR, Mode::from_raw_mode(0o700)) {
                    Ok(()) | Err(Errno::EXIST) => {}
                    Err(error) => return Err(error),
                }
                open_dir_at(&current, STORE_DIR)?
            }
            Err(error) => return Err(error),
        };
        check_private_dir(&app_dir)?;
        Ok(app_dir)
    }
}

impl Policy {
    pub(super) fn new() -> Self {
        Self {
            version: 1,
            revoked_roots: Vec::new(),
            protected_roots: Vec::new(),
            exclusions: Vec::new(),
        }
    }
}

fn open_dir_at(parent: &OwnedFd, name: impl rustix::path::Arg) -> Result<OwnedFd, Errno> {
    fs::openat2(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH
            | ResolveFlags::NO_SYMLINKS
            | ResolveFlags::NO_MAGICLINKS
            | ResolveFlags::NO_XDEV,
    )
}

fn check_ancestor(fd: &OwnedFd) -> Result<(), Errno> {
    let metadata = fs::fstat(fd)?;
    inodune_core::access::stat_write_allowed(
        inodune_core::access::WriteRole::StorageAncestor,
        metadata.st_uid,
        metadata.st_gid,
        metadata.st_mode,
        &inodune_core::access::SystemGroups,
    )
    .map_err(|_| Errno::ACCESS)
}

fn check_private_dir(fd: &OwnedFd) -> Result<(), Errno> {
    let metadata = fs::fstat(fd)?;
    inodune_core::access::stat_write_allowed(
        inodune_core::access::WriteRole::PrivateStore,
        metadata.st_uid,
        metadata.st_gid,
        metadata.st_mode,
        &inodune_core::access::SystemGroups,
    )
    .map_err(|_| Errno::ACCESS)
}

fn validate_policy(policy: &Policy) -> Result<(), String> {
    if policy.version != 1 {
        return Err("Versão da política de proteção desconhecida; análise bloqueada.".into());
    }
    let count = policy.revoked_roots.len()
        + policy.protected_roots.len()
        + policy
            .exclusions
            .iter()
            .map(|item| 1 + item.relative_paths.len())
            .sum::<usize>();
    if count > MAX_POLICY_PATHS {
        return Err("Política de proteção contém mais caminhos que o limite permitido.".into());
    }
    for path in policy
        .revoked_roots
        .iter()
        .chain(&policy.protected_roots)
        .chain(policy.exclusions.iter().map(|item| &item.root))
    {
        validate_absolute_path(path)?;
    }
    for exclusion in &policy.exclusions {
        for path in &exclusion.relative_paths {
            let mut depth = 0usize;
            if path.as_os_str().len() > MAX_PATH_BYTES
                || path.is_absolute()
                || path.as_os_str().is_empty()
                || path
                    .to_str()
                    .is_none_or(|value| value.chars().any(char::is_control))
            {
                return Err("Caminho excluído inválido.".into());
            }
            if path == Path::new(".") {
                continue;
            }
            for component in path.components() {
                match component {
                    Component::Normal(_) => depth += 1,
                    Component::CurDir => {}
                    _ => {
                        return Err(
                            "Caminho excluído precisa ser relativo e não pode escapar do escopo."
                                .into(),
                        )
                    }
                }
            }
            if depth == 0 || depth > 256 {
                return Err("Profundidade de exclusão fora do limite.".into());
            }
        }
    }
    Ok(())
}

fn validate_absolute_path(path: &Path) -> Result<(), String> {
    if !path.is_absolute()
        || path.as_os_str().len() > MAX_PATH_BYTES
        || path
            .to_str()
            .is_none_or(|value| value.chars().any(char::is_control))
    {
        return Err("Caminho protegido inválido.".into());
    }
    let mut depth = 0usize;
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(_) => depth += 1,
            _ => return Err("Caminho protegido não pode conter componentes ambíguos.".into()),
        }
    }
    if depth == 0 || depth > 256 {
        return Err("Profundidade do caminho protegido fora do limite.".into());
    }
    Ok(())
}

#[cfg(all(test, feature = "fixture-tests"))]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::symlink};

    #[test]
    fn private_policy_round_trips_only_explicit_protections_and_revocations() {
        let temporary = tempfile::tempdir().unwrap();
        let config = temporary.path().join("config");
        fs::create_dir(&config).unwrap();
        let store = ProtectionStore::at(config).unwrap();
        let mut policy = Policy::new();
        policy
            .revoked_roots
            .push(temporary.path().join("home/revoked"));
        policy
            .protected_roots
            .push(temporary.path().join("home/private"));
        policy.exclusions.push(PersistedExclusion {
            root: temporary.path().join("home/project"),
            relative_paths: vec![PathBuf::from("fixtures/local-data")],
        });

        store.save(&policy).unwrap();
        assert_eq!(store.load().unwrap(), policy);
        let directory = fs::symlink_metadata(temporary.path().join("config/inodune")).unwrap();
        assert_eq!(directory.permissions().mode() & 0o777, 0o700);
        let file = fs::symlink_metadata(temporary.path().join("config/inodune/protection-v1.json"))
            .unwrap();
        assert_eq!(file.permissions().mode() & 0o777, 0o600);
    }

    #[test]
    fn policy_store_creates_only_the_missing_config_home_as_private() {
        let temporary = tempfile::tempdir().unwrap();
        let config = temporary.path().join("missing-config");
        let store = ProtectionStore::at(config.clone()).unwrap();

        store.save(&Policy::new()).unwrap();
        assert_eq!(store.load().unwrap(), Policy::new());
        assert_eq!(
            fs::symlink_metadata(&config).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }

    #[test]
    fn policy_store_rejects_symlinked_ancestors_and_policy_files() {
        let temporary = tempfile::tempdir().unwrap();
        let actual = temporary.path().join("actual");
        fs::create_dir(&actual).unwrap();
        let linked_config = temporary.path().join("linked-config");
        symlink(&actual, &linked_config).unwrap();
        let linked_store = ProtectionStore::at(linked_config).unwrap();
        assert!(linked_store.load().is_err());

        let secure_config = temporary.path().join("secure-config");
        fs::create_dir(&secure_config).unwrap();
        let store = ProtectionStore::at(secure_config.clone()).unwrap();
        store.save(&Policy::new()).unwrap();
        let hardlink = temporary.path().join("policy-hardlink");
        fs::hard_link(secure_config.join("inodune/protection-v1.json"), &hardlink).unwrap();
        assert!(store.load().is_err());
        fs::remove_file(hardlink).unwrap();
        fs::remove_file(secure_config.join("inodune/protection-v1.json")).unwrap();
        symlink(
            temporary.path().join("victim"),
            secure_config.join("inodune/protection-v1.json"),
        )
        .unwrap();
        assert!(store.load().is_err());
    }

    #[test]
    fn policy_store_rejects_non_regular_files_without_blocking_on_them() {
        let temporary = tempfile::tempdir().unwrap();
        let config = temporary.path().join("config");
        fs::create_dir(&config).unwrap();
        let store = ProtectionStore::at(config).unwrap();
        store.save(&Policy::new()).unwrap();
        let directory = store.open_policy_dir(false).unwrap();
        rustix::fs::unlinkat(&directory, STORE_FILE, rustix::fs::AtFlags::empty()).unwrap();
        rustix::fs::mkfifoat(&directory, STORE_FILE, Mode::from_raw_mode(0o600)).unwrap();
        assert!(store.load().is_err());
    }

    #[test]
    fn invalid_or_unknown_policy_never_becomes_an_empty_policy() {
        let temporary = tempfile::tempdir().unwrap();
        let config = temporary.path().join("config");
        fs::create_dir(&config).unwrap();
        let store = ProtectionStore::at(config.clone()).unwrap();
        store.save(&Policy::new()).unwrap();
        fs::write(
            config.join("inodune/protection-v1.json"),
            br#"{"version":99,"revoked_roots":[],"protected_roots":[],"exclusions":[]}"#,
        )
        .unwrap();
        assert!(store.load().unwrap_err().contains("Versão"));
    }

    #[test]
    fn policy_with_missing_protection_fields_is_rejected_instead_of_reset() {
        let temporary = tempfile::tempdir().unwrap();
        let config = temporary.path().join("config");
        let store = ProtectionStore::at(config.clone()).unwrap();
        store.save(&Policy::new()).unwrap();
        for incomplete in [
            br#"{"version":1}"#.as_slice(),
            br#"{"version":1,"revoked_roots":[],"protected_roots":[]}"#.as_slice(),
            br#"{"version":1,"protected_roots":[],"exclusions":[]}"#.as_slice(),
            br#"{"version":1,"revoked_roots":[],"exclusions":[]}"#.as_slice(),
        ] {
            fs::write(config.join("inodune/protection-v1.json"), incomplete).unwrap();
            assert!(
                store.load().is_err(),
                "Incomplete policies must fail closed"
            );
        }
    }
}
