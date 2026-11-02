use crate::{
    command::{run, Query},
    files, report, timestamp, valid_id, InstalledItem, PackageSource, Source, SourceReport,
    UsageSignal,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::Path,
};
fn binary(name: &str) -> bool {
    Path::new(name).is_file()
}
fn ids(text: &str) -> Result<BTreeSet<String>, String> {
    text.lines()
        .map(|s| {
            if valid_id(s) {
                Ok(s.to_owned())
            } else {
                Err("ID inválido na fonte".into())
            }
        })
        .collect()
}
pub struct RpmDnf;
impl PackageSource for RpmDnf {
    fn name(&self) -> &'static str {
        "rpm"
    }
    fn available(&self) -> bool {
        binary("/usr/bin/dnf") && binary("/usr/bin/rpm")
    }
    fn list(&self) -> Result<SourceReport, String> {
        let names: Vec<_> = ids(&run(Query::Dnf)?)?.into_iter().collect();
        let protected = protected_names(Path::new("/etc/dnf/protected.d"));
        let mut items = vec![];
        let mut warnings = vec![];
        for batch in names.chunks(128) {
            let (parsed, batch_warnings) =
                parse_rpm(&run(Query::Rpm(batch))?, protected.as_ref().ok())?;
            items.extend(parsed);
            warnings.extend(batch_warnings);
        }
        let owners = crate::uninstall::core_utility_owners(Source::Rpm)?;
        for item in &mut items {
            if owners.contains(&item.id) {
                item.protected = true;
                item.notes.push("Fornece binário básico do sistema".into());
            }
        }
        let mut result = report(self.name(), "ok", items);
        result.warnings = warnings;
        if let Err(e) = protected {
            result.warnings.push(e);
        }
        Ok(result)
    }
}
fn protected_names(dir: &Path) -> Result<BTreeSet<String>, String> {
    let mut names = BTreeSet::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(names),
        Err(e) => return Err(e.to_string()),
    };
    for (i, entry) in entries.enumerate() {
        if i >= 256 {
            return Err("Limite protected.d".into());
        }
        let text = files::read(&entry.map_err(|e| e.to_string())?.path())?;
        for line in text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
        {
            // Unknown glob/config syntax fails closed for the entire RPM inventory.
            if !valid_id(line) {
                return Err("Sintaxe protected.d não suportada".into());
            }
            names.insert(line.into());
        }
    }
    Ok(names)
}
fn parse_rpm(
    text: &str,
    _protected: Option<&BTreeSet<String>>,
) -> Result<(Vec<InstalledItem>, Vec<String>), String> {
    let mut items = vec![];
    let mut warnings = vec![];
    for line in text.lines() {
        let f: Vec<_> = line.split('\t').collect();
        if f.len() != 4 || !valid_id(f[0]) {
            return Err("Registro RPM inválido".into());
        }
        let mut item = InstalledItem::new(Source::Rpm, f[0], f[0]);
        item.version = (!f[1].is_empty()).then(|| f[1].into());
        match rpm_size(f[2]) {
            Ok(size) => item.size_bytes = size,
            Err(()) => {
                warnings.push(format!("Tamanho RPM inválido: {}", f[0]));
                item.notes
                    .push("Tamanho RPM inválido; tamanho desconhecido".into());
            }
        }
        item.install_date = timestamp(f[3]);
        if crate::uninstall::package_blocked(f[0]) {
            item.protect();
        }
        item.notes
            .push("Motivo de instalação DNF não prova autoria humana; sem sinal de uso".into());
        items.push(item);
    }
    Ok((items, warnings))
}
fn rpm_size(raw: &str) -> Result<Option<u64>, ()> {
    let raw = raw.trim();
    if raw.is_empty() || raw == "(none)" {
        return Ok(None);
    }
    raw.parse::<u64>().map(Some).map_err(|_| ())
}
pub struct Flatpak<'a> {
    pub home: &'a Path,
}
impl PackageSource for Flatpak<'_> {
    fn name(&self) -> &'static str {
        "flatpak"
    }
    fn available(&self) -> bool {
        binary("/usr/bin/flatpak")
    }
    fn list(&self) -> Result<SourceReport, String> {
        let mut result = report(self.name(), "ok", vec![]);
        for user in [true, false] {
            match run(Query::Flatpak(user)).and_then(|s| parse_flatpak(&s, user)) {
                Ok(mut items) => {
                    for item in &mut items {
                        if let Ok(dir) =
                            files::open_safe(&self.home.join(".var/app").join(&item.id), true)
                        {
                            if let Ok(time) =
                                dir.metadata().and_then(|m| m.modified()).and_then(|t| {
                                    t.duration_since(std::time::UNIX_EPOCH)
                                        .map_err(std::io::Error::other)
                                })
                            {
                                item.signals.push(UsageSignal {
                                    kind: "flatpak_data_directory_mtime".into(),
                                    timestamp: time.as_secs(),
                                    confidence: "low".into(),
                                });
                            }
                        }
                        item.notes.push(
                            "mtime de dados compartilhado por ID: alteração não comprova execução"
                                .into(),
                        );
                    }
                    result.items.extend(items);
                }
                Err(e) => {
                    result.status = "partial".into();
                    result.warnings.push(e);
                }
            }
        }
        Ok(result)
    }
}
fn parse_flatpak(text: &str, user: bool) -> Result<Vec<InstalledItem>, String> {
    text.lines()
        .map(|line| {
            let f: Vec<_> = line.split('\t').collect();
            if f.len() != 6 || !valid_id(f[0]) || !f[0].contains('.') {
                return Err("Registro Flatpak inválido".into());
            }
            let mut item = InstalledItem::new(
                if user {
                    Source::FlatpakUser
                } else {
                    Source::FlatpakSystem
                },
                f[0],
                f[1],
            );
            item.version = (!f[2].is_empty()).then(|| f[2].into());
            item.size_bytes = human_size(f[3]);
            if item.size_bytes.is_none() && !f[3].trim().is_empty() {
                item.notes.push("Tamanho Flatpak ilegível".into());
            }
            item.notes.push(format!(
                "branch={}, arch={}; tamanho aproximado do Flatpak",
                f[4], f[5]
            ));
            Ok(item)
        })
        .collect()
}
fn human_size(s: &str) -> Option<u64> {
    let f: Vec<_> = s.split_whitespace().collect();
    if f.len() != 2 {
        return None;
    }
    let factor = match f[1] {
        "bytes" | "B" => 1.,
        "kB" => 1e3,
        "MB" => 1e6,
        "GB" => 1e9,
        "TB" => 1e12,
        _ => return None,
    };
    let n = f[0].parse::<f64>().ok()? * factor;
    (n.is_finite() && n >= 0. && n < u64::MAX as f64).then_some(n.round() as u64)
}
pub struct AptDpkg;
impl PackageSource for AptDpkg {
    fn name(&self) -> &'static str {
        "apt"
    }
    fn available(&self) -> bool {
        binary("/usr/bin/apt-mark") && binary("/usr/bin/dpkg-query")
    }
    fn list(&self) -> Result<SourceReport, String> {
        let manual = ids(&run(Query::AptManual)?)?;
        let (mut items, mut warnings) = parse_dpkg(&run(Query::Dpkg)?, &manual)?;
        let owners = crate::uninstall::core_utility_owners(Source::Apt)?;
        for item in &mut items {
            if owners.contains(item.id.split(':').next().unwrap_or(&item.id)) {
                item.protected = true;
                item.notes.push("Fornece binário básico do sistema".into());
            }
        }
        let (dates, date_warnings) = dpkg_dates(Path::new("/var/log"));
        warnings.extend(date_warnings);
        for item in &mut items {
            item.install_date = dates
                .get(&item.id)
                .or_else(|| dates.get(item.id.split(':').next().unwrap_or(&item.id)))
                .copied();
            item.notes.push("Data do primeiro install retido nos logs; timezone local atual, histórico pode estar incompleto".into());
        }
        Ok(SourceReport {
            warnings,
            ..report(self.name(), "ok", items)
        })
    }
}
pub(crate) fn parse_dpkg(
    text: &str,
    manual: &BTreeSet<String>,
) -> Result<(Vec<InstalledItem>, Vec<String>), String> {
    let mut items = vec![];
    let mut warnings = vec![];
    for line in text.lines() {
        let f: Vec<_> = line.split('\t').collect();
        if f.len() != 6 || !valid_id(f[0]) {
            return Err("Registro dpkg inválido".into());
        }
        if f[3] != "install ok installed"
            || !(manual.contains(f[0]) || manual.contains(f[0].split(':').next().unwrap_or(f[0])))
        {
            continue;
        }
        let mut item = InstalledItem::new(Source::Apt, f[0], f[0]);
        item.version = (!f[1].is_empty()).then(|| f[1].into());
        match apt_installed_bytes(f[2]) {
            Ok(size) => item.size_bytes = size,
            Err(()) => {
                warnings.push(format!("Installed-Size inválido: {}", f[0]));
                item.notes
                    .push("Installed-Size inválido; tamanho desconhecido".into());
            }
        }
        if f[4] == "yes" || f[5] == "required" || crate::uninstall::package_blocked(f[0]) {
            item.protect();
        }
        items.push(item);
    }
    Ok((items, warnings))
}
fn apt_installed_bytes(raw: &str) -> Result<Option<u64>, ()> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    let kib = raw.parse::<u64>().map_err(|_| ())?;
    kib.checked_mul(1024).map(Some).ok_or(())
}
fn log_dates(text: &str, dates: &mut BTreeMap<String, u64>) {
    use chrono::TimeZone;
    for line in text.lines() {
        let f: Vec<_> = line.split_whitespace().collect();
        if f.len() != 6 || f[2] != "install" || f[4] != "<none>" || !valid_id(f[3]) {
            continue;
        }
        let date = chrono::NaiveDateTime::parse_from_str(
            &format!("{} {}", f[0], f[1]),
            "%Y-%m-%d %H:%M:%S",
        );
        if let Ok(date) = date {
            if let Some(date) = chrono::Local.from_local_datetime(&date).single() {
                if let Ok(t) = u64::try_from(date.timestamp()) {
                    let id = f[3].split(':').next().unwrap_or(f[3]);
                    dates
                        .entry(id.into())
                        .and_modify(|old| *old = (*old).min(t))
                        .or_insert(t);
                }
            }
        }
    }
}
fn decode_log(file: std::fs::File, gzip: bool) -> Result<String, String> {
    let reader: Box<dyn Read> = if gzip {
        Box::new(flate2::read::GzDecoder::new(
            file.take((files::MAX_FILE + 1) as u64),
        ))
    } else {
        Box::new(file)
    };
    let mut bytes = vec![];
    reader
        .take((files::MAX_FILE + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > files::MAX_FILE {
        return Err("Log excede limite".into());
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}
fn dpkg_dates(dir: &Path) -> (BTreeMap<String, u64>, Vec<String>) {
    let mut dates = BTreeMap::new();
    let mut warnings = vec![];
    let paths = std::iter::once(dir.join("dpkg.log")).chain((1..=12).flat_map(|n| {
        [
            dir.join(format!("dpkg.log.{n}")),
            dir.join(format!("dpkg.log.{n}.gz")),
        ]
    }));
    for path in paths {
        if !path.exists() {
            continue;
        }
        match files::open_safe(&path, false)
            .and_then(|f| decode_log(f, path.extension().is_some_and(|s| s == "gz")))
        {
            Ok(text) => log_dates(&text, &mut dates),
            Err(_) => warnings.push("Log dpkg ilegível ou excede limite; datas parciais".into()),
        }
    }
    (dates, warnings)
}
pub struct Snap;
impl PackageSource for Snap {
    fn name(&self) -> &'static str {
        "snap"
    }
    fn available(&self) -> bool {
        Path::new("/run/snapd.socket").exists()
    }
    fn list(&self) -> Result<SourceReport, String> {
        Ok(report(
            self.name(),
            "ok",
            crate::snap_api::list().or_else(|_| parse_snap(&run(Query::Snap)?))?,
        ))
    }
}
fn parse_snap(text: &str) -> Result<Vec<InstalledItem>, String> {
    let mut lines = text.lines();
    let Some(header) = lines.next() else {
        return Err("Cabeçalho Snap ausente".into());
    };
    if !header.starts_with("Name ") {
        return Err("Cabeçalho Snap inválido".into());
    }
    lines
        .map(|line| {
            let f: Vec<_> = line.split_whitespace().collect();
            if f.len() != 6 || !valid_id(f[0]) || !f[2].bytes().all(|b| b.is_ascii_alphanumeric()) {
                return Err("Registro Snap inválido".into());
            }
            let mut item = InstalledItem::new(Source::Snap, &format!("{}@{}", f[0], f[2]), f[0]);
            item.version = Some(f[1].into());
            let notes: Vec<_> = f[5].split(',').collect();
            if f[0] == "snapd"
                || f[0] == "core"
                || f[0]
                    .strip_prefix("core")
                    .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
                || notes.iter().any(|s| matches!(*s, "base" | "snapd"))
            {
                item.protect();
            }
            if notes.contains(&"disabled") {
                item.notes.push("Revisão desabilitada".into());
            }
            item.notes
                .push("Snap não informa tamanho nem data de instalação.".into());
            Ok(item)
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn synthetic_rpm_dnf_flatpak() {
        let names = ids("editor\ncompiler\n").unwrap();
        assert_eq!(names.len(), 2);
        assert!(ids("-bad\n").is_err());
        let (mut parsed, warnings) = parse_rpm(
            "editor\t1.0-1\t1024\t1700000000\n",
            Some(&BTreeSet::from(["editor".into()])),
        )
        .unwrap();
        assert!(warnings.is_empty());
        let item = parsed.remove(0);
        assert!(!item.protected);
        assert_eq!(item.install_date, Some(1700000000));
        assert!(parse_rpm("bad data", None).is_err());
        for user in [true, false] {
            let items = parse_flatpak(
                "org.example.Editor\tEditor\t1\t1.5 MB\tstable\tx86_64\n",
                user,
            )
            .unwrap();
            assert_eq!(items[0].size_bytes, Some(1500000));
            assert_eq!(
                items[0].source,
                if user {
                    Source::FlatpakUser
                } else {
                    Source::FlatpakSystem
                }
            );
        }
    }
    #[test]
    fn synthetic_apt_status_sizes_protection() {
        let manual = ids("editor\nessential\npriority\nremoved\n").unwrap();
        let text="editor\t1\t12\tinstall ok installed\tno\toptional\nessential\t2\t1\tinstall ok installed\tyes\toptional\npriority\t1\t2\tinstall ok installed\tno\timportant\nremoved\t1\t0\tdeinstall ok config-files\tno\toptional\ndependency\t1\t3\tinstall ok installed\tno\toptional\n";
        let (items, warnings) = parse_dpkg(text, &manual).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].size_bytes, Some(12 * 1024));
        assert!(items[1].protected && !items[2].protected);
        assert!(parse_dpkg("bad", &manual).is_err());
    }
    #[test]
    fn empty_installed_size_keeps_valid_apt_packages() {
        let manual = ids("synthetic-editor\nvendor-vpn-client\nvendor-agent\nvendor-endpoint-security\nvendor-network-agent\nvendor-tool\nsynthetic-sdk\nsynthetic-lib\n").unwrap();
        let (items, warnings) = parse_dpkg(
            include_str!("../fixtures/dpkg-empty-installed-size.txt"),
            &manual,
        )
        .expect("Installed-Size vazio não pode falhar a fonte apt");
        assert!(warnings.is_empty());
        let json = serde_json::to_value(&items).unwrap();
        assert!(json.as_array().unwrap().iter().any(|item| {
            item["id"] == "vendor-vpn-client:amd64" && item["size_bytes"].is_null()
        }));
        assert_eq!(items.len(), 8);
        let by_id = |id: &str| items.iter().find(|item| item.id == id).expect(id);
        assert_eq!(by_id("synthetic-editor:amd64").size_bytes, Some(12 * 1024));
        assert_eq!(by_id("synthetic-lib:amd64").size_bytes, Some(1024));
        assert_eq!(
            by_id("synthetic-sdk:amd64").size_bytes,
            Some(1_234_567 * 1024)
        );
        for (id, version) in [
            ("vendor-vpn-client:amd64", "6.2.0"),
            ("vendor-agent:amd64", "12.5.0-1569"),
            ("vendor-endpoint-security:amd64", "16.2.0.1352"),
            ("vendor-network-agent:amd64", "1.2.835.0-1"),
            ("vendor-tool:amd64", "6.2.0"),
        ] {
            let item = by_id(id);
            assert_eq!(item.size_bytes, None, "{id}");
            assert_eq!(item.version.as_deref(), Some(version));
        }
        assert!(items.iter().all(|item| item.id != "not-manual:amd64"));
        assert!(items.iter().all(|item| item.id != "removed-pkg:amd64"));
    }
    #[test]
    fn malformed_installed_size_warns_without_failing_apt() {
        let manual = ids("synthetic-editor\nvendor-badsize\nvendor-huge\nvendor-blank\n").unwrap();
        let text = "\
synthetic-editor:amd64\t1.2.3-1\t12\tinstall ok installed\tno\toptional\n\
vendor-badsize:amd64\t9.9.9\tnope\tinstall ok installed\tno\toptional\n\
vendor-huge:amd64\t1\t18446744073709551615\tinstall ok installed\tno\toptional\n\
vendor-blank:amd64\t\t   \tinstall ok installed\tno\toptional\n";
        let (items, warnings) = parse_dpkg(text, &manual).unwrap();
        assert_eq!(items.len(), 4);
        assert_eq!(items[0].size_bytes, Some(12 * 1024));
        assert_eq!(items[1].size_bytes, None);
        assert_eq!(items[2].size_bytes, None);
        assert_eq!(items[3].size_bytes, None);
        assert_eq!(items[3].version, None);
        assert!(items[1]
            .notes
            .iter()
            .any(|note| note.contains("desconhecido")));
        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().any(|w| w.contains("vendor-badsize:amd64")));
        assert!(warnings.iter().any(|w| w.contains("vendor-huge:amd64")));
        assert!(warnings.iter().all(|w| !w.contains("vendor-blank")));
        assert!(warnings.iter().all(|w| !w.contains("synthetic-editor")));
    }
    #[test]
    fn rpm_missing_size_does_not_fail_the_source() {
        let (items, warnings) = parse_rpm(
            "editor\t1.0-1\t\t1700000000\nlibrary\t2\t(none)\t1700000000\nbroken\t3\txyz\t1700000000\n",
            Some(&BTreeSet::new()),
        )
        .unwrap();
        assert_eq!(items.len(), 3);
        assert!(items.iter().all(|item| item.size_bytes.is_none()));
        assert!(!items[0].protected && !items[1].protected);
        assert_eq!(warnings, vec!["Tamanho RPM inválido: broken".to_string()]);
        assert!(parse_rpm("bad data", None).is_err());
    }
    #[test]
    fn flatpak_missing_size_stays_on_the_item() {
        let items = parse_flatpak(
            "org.example.Editor\tEditor\t1\t1.5 MB\tstable\tx86_64\norg.example.Empty\tEmpty\t\t\tstable\tx86_64\norg.example.Bad\tBad\t1\tnope\tstable\tx86_64\n",
            false,
        )
        .unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].size_bytes, Some(1_500_000));
        assert_eq!(items[1].size_bytes, None);
        assert_eq!(items[1].version, None);
        assert!(items[1]
            .notes
            .iter()
            .all(|n| n != "Tamanho Flatpak ilegível"));
        assert_eq!(items[2].size_bytes, None);
        assert!(items[2]
            .notes
            .iter()
            .any(|n| n == "Tamanho Flatpak ilegível"));
    }
    #[test]
    fn dpkg_plain_rotated_gzip_and_bombs() {
        use std::io::Write;
        let temp = tempfile::tempdir().unwrap();
        let log="2026-01-02 03:04:05 install editor:amd64 <none> 1.0\nmalformed\n2026-01-03 00:00:00 upgrade editor:amd64 1.0 2.0\n";
        std::fs::write(temp.path().join("dpkg.log"), "invalid\n").unwrap();
        let file = std::fs::File::create(temp.path().join("dpkg.log.1.gz")).unwrap();
        let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        encoder.write_all(log.as_bytes()).unwrap();
        encoder.finish().unwrap();
        let (dates, warnings) = dpkg_dates(temp.path());
        assert!(dates.contains_key("editor"));
        assert!(warnings.is_empty());
        std::fs::write(temp.path().join("dpkg.log.2.gz"), b"bad gzip").unwrap();
        assert!(!dpkg_dates(temp.path()).1.is_empty());
        let file = std::fs::File::create(temp.path().join("bomb.gz")).unwrap();
        let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        encoder.write_all(&vec![b'a'; files::MAX_FILE + 1]).unwrap();
        encoder.finish().unwrap();
        assert!(decode_log(
            files::open_safe(&temp.path().join("bomb.gz"), false).unwrap(),
            true
        )
        .is_err());
    }
    #[test]
    fn snap_revisions_and_protection() {
        let items=parse_snap("Name Version Rev Tracking Publisher Notes\ncore22 1 123 latest/stable canonical base,disabled\neditor 2 456 latest/stable example -\n").unwrap();
        assert_eq!(items.len(), 2);
        assert!(items[0].protected);
        assert!(!items[1].protected);
        assert!(items[0].notes.iter().any(|n| n.contains("desabilitada")));
        assert!(parse_snap("Name Version Rev Tracking Publisher Notes\n-bad 1 2 x x -\n").is_err());
    }
}
