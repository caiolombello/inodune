use rustix::fs::{open, openat, Mode, OFlags};
use std::{
    fs::File,
    io::Read,
    path::{Component, Path},
};
pub(crate) const MAX_FILE: usize = 2 * 1024 * 1024;
/// Open every component relative to pinned descriptors. No symlinks, including ancestors.
pub(crate) fn open_safe(path: &Path, directory: bool) -> Result<File, String> {
    if !path.is_absolute() {
        return Err("Caminho não absoluto".into());
    }
    let mut fd = open(
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|e| e.to_string())?;
    let components: Vec<_> = path.components().collect();
    for (index, component) in components.iter().enumerate().skip(1) {
        let Component::Normal(name) = component else {
            return Err("Componente inválido".into());
        };
        let dir = index + 1 < components.len() || directory;
        fd = openat(
            &fd,
            *name,
            OFlags::RDONLY
                | OFlags::NOFOLLOW
                | OFlags::CLOEXEC
                | OFlags::NONBLOCK
                | if dir {
                    OFlags::DIRECTORY
                } else {
                    OFlags::empty()
                },
            Mode::empty(),
        )
        .map_err(|e| e.to_string())?;
    }
    let file = File::from(fd);
    if !directory && !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Não é arquivo regular".into());
    }
    Ok(file)
}
pub(crate) fn read(path: &Path) -> Result<String, String> {
    let file = open_safe(path, false)?;
    let mut bytes = vec![];
    file.take((MAX_FILE + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_FILE {
        return Err("Arquivo excede 2 MiB".into());
    }
    String::from_utf8(bytes).map_err(|_| "Arquivo não UTF-8".into())
}
