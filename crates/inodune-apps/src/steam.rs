use crate::{
    files, report, timestamp, vdf, InstalledItem, PackageSource, Source, SourceReport, UsageSignal,
};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
pub struct Steam<'a> {
    pub home: &'a Path,
}
impl PackageSource for Steam<'_> {
    fn name(&self) -> &'static str {
        "steam"
    }
    fn available(&self) -> bool {
        self.home.join(".local/share/Steam").exists()
    }
    fn list(&self) -> Result<SourceReport, String> {
        collect(&self.home.join(".local/share/Steam"))
    }
}
fn collect(root: &Path) -> Result<SourceReport, String> {
    let text = files::read(&root.join("steamapps/libraryfolders.vdf"))?;
    let parsed = vdf::parse(&text)?;
    let folders = parsed
        .get("libraryfolders")
        .and_then(vdf::Value::object)
        .ok_or("libraryfolders ausente")?;
    let mut libraries = BTreeSet::from([root.to_path_buf()]);
    for (key, value) in folders {
        if !key.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let path = value
            .text()
            .or_else(|| value.object()?.get("path")?.text())
            .ok_or("Biblioteca inválida")?;
        libraries.insert(PathBuf::from(path));
        if libraries.len() > 64 {
            return Err("Muitas bibliotecas".into());
        }
    }
    let mut result = report("steam", "ok", vec![]);
    for path in libraries {
        match library(&path) {
            Ok(items) => result.items.extend(items),
            Err(_) => {
                result.status = "partial".into();
                result
                    .warnings
                    .push("Biblioteca Steam recusada ou inválida (inclui symlinks)".into());
            }
        }
    }
    Ok(result)
}
fn library(path: &Path) -> Result<Vec<InstalledItem>, String> {
    // Pin the directory for iteration; all file opens below independently reject links.
    let directory = files::open_safe(&path.join("steamapps"), true)?;
    use std::os::fd::AsRawFd;
    let entries = std::fs::read_dir(format!("/proc/self/fd/{}", directory.as_raw_fd()))
        .map_err(|e| e.to_string())?;
    let mut items = vec![];
    for (i, entry) in entries.enumerate() {
        if i >= 20000 {
            return Err("Limite de entradas Steam".into());
        }
        let name = entry.map_err(|e| e.to_string())?.file_name();
        let name = name.to_string_lossy();
        let Some(id) = name
            .strip_prefix("appmanifest_")
            .and_then(|s| s.strip_suffix(".acf"))
        else {
            continue;
        };
        if id.is_empty() || id.len() > 20 || !id.bytes().all(|b| b.is_ascii_digit()) {
            return Err("ID Steam inválido".into());
        }
        items.push(manifest(
            &files::read(&path.join("steamapps").join(name.as_ref()))?,
            id,
        )?);
    }
    Ok(items)
}
fn manifest(text: &str, id: &str) -> Result<InstalledItem, String> {
    let parsed = vdf::parse(text)?;
    let state = parsed
        .get("AppState")
        .and_then(vdf::Value::object)
        .ok_or("AppState ausente")?;
    let get = |key| state.get(key).and_then(vdf::Value::text);
    if get("appid") != Some(id) {
        return Err("AppID divergente".into());
    }
    let mut item = InstalledItem::new(Source::Steam, id, get("name").ok_or("Nome ausente")?);
    item.size_bytes = get("SizeOnDisk").and_then(|s| s.parse().ok());
    if let Some(t) = get("LastPlayed").and_then(timestamp) {
        item.signals.push(UsageSignal {
            kind: "steam_last_played".into(),
            timestamp: t,
            confidence: "medium".into(),
        });
    }
    if let Some(t) = get("LastUpdated").and_then(timestamp) {
        item.notes
            .push(format!("Última atualização Steam (não é uso): {t}"));
    }
    item.notes
        .push("Metadados locais Steam; tamanho lógico declarado, não espaço recuperável".into());
    Ok(item)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manifest_fields_and_no_update_as_usage() {
        let s = r#""AppState" { "appid" "42" "name" "Synthetic Game" "SizeOnDisk" "1024" "LastUpdated" "1700000000" "LastPlayed" "1700000010" }"#;
        let item = manifest(s, "42").unwrap();
        assert_eq!(item.size_bytes, Some(1024));
        assert_eq!(item.signals.len(), 1);
        assert!(manifest(s, "43").is_err());
    }
    #[test]
    fn library_and_manifest_symlinks_are_refused() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Steam");
        std::fs::create_dir_all(root.join("steamapps")).unwrap();
        std::fs::write(
            root.join("steamapps/libraryfolders.vdf"),
            "\"libraryfolders\" {}",
        )
        .unwrap();
        std::fs::write(
            root.join("steamapps/appmanifest_42.acf"),
            r#""AppState" { "appid" "42" "name" "Synthetic" }"#,
        )
        .unwrap();
        assert_eq!(collect(&root).unwrap().items.len(), 1);
        symlink(&root, temp.path().join("link")).unwrap();
        assert!(collect(&temp.path().join("link")).is_err());
        symlink(
            root.join("steamapps/appmanifest_42.acf"),
            root.join("steamapps/appmanifest_43.acf"),
        )
        .unwrap();
        assert_eq!(collect(&root).unwrap().status, "partial");
    }
}
