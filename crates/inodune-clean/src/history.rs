//! Read-only presentation of saved cleanup reports. Persisted JSON is untrusted data.
use crate::{executor::ExecutionReport, storage};
use rustix::{
    fd::AsFd,
    fs::{self, FileType, OFlags, ResolveFlags},
};
use serde::Serialize;
use std::{
    io::{BufRead, BufReader, Read},
    path::Path,
};

const MAX_REPORT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_DIRECTORY_ENTRIES: usize = 4096;

#[derive(Clone, Debug, Serialize)]
pub struct HistoryEntry {
    pub report_id: String,
    pub created_unix: u64,
    pub plan_id: String,
    pub action: Option<crate::plan::Action>,
    pub complete: bool,
    pub cancelled: bool,
    pub logical_bytes_processed: u64,
    pub files_count: usize,
    pub error: Option<String>,
}

pub fn list_history(directory: &Path) -> Result<Vec<HistoryEntry>, String> {
    let Some(dir) = storage::open_dir_optional(directory, true)? else {
        return Ok(Vec::new());
    };
    let mut names = Vec::new();
    let mut seen = 0usize;
    let mut buffer = [std::mem::MaybeUninit::uninit(); 8192];
    let mut entries = fs::RawDir::new(&dir, &mut buffer);
    while let Some(entry) = entries.next() {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_bytes();
        if name == b"." || name == b".." {
            continue;
        }
        seen = seen.saturating_add(1);
        if seen > MAX_DIRECTORY_ENTRIES {
            return Err(
                "Diretório de histórico excede 4.096 entradas; arquive arquivos manualmente."
                    .into(),
            );
        }
        if let Ok(name) = std::str::from_utf8(name) {
            if valid_report_name(name) || valid_journal_name(name) {
                names.push(name.to_owned());
            }
        }
    }
    names.sort();
    let mut result = Vec::with_capacity(names.len());
    // Streaming read; this only bounds total I/O when listing many journals.
    let mut journal_budget = (4 * storage::MAX_JOURNAL_BYTES) as usize;
    for name in names {
        if valid_journal_name(&name) {
            let error = inspect_journal(&dir, &name, &mut journal_budget).err();
            if let Some(error) = error {
                result.push(HistoryEntry {
                    report_id: name.clone(),
                    created_unix: 0,
                    plan_id: String::new(),
                    action: None,
                    complete: false,
                    cancelled: false,
                    logical_bytes_processed: 0,
                    files_count: 0,
                    error: Some(format!(
                        "{error} Journal: {}",
                        directory.join(&name).display()
                    )),
                });
            }
            continue;
        }
        let id = name.trim_end_matches(".json").to_owned();
        match read_report_at(&dir, &name) {
            Ok((report, created)) => result.push(entry(id, created, &report, None)),
            Err(error) => result.push(HistoryEntry {
                report_id: id,
                created_unix: 0,
                plan_id: String::new(),
                action: None,
                complete: false,
                cancelled: false,
                logical_bytes_processed: 0,
                files_count: 0,
                error: Some(error),
            }),
        }
    }
    result.sort_by(|a, b| {
        b.created_unix
            .cmp(&a.created_unix)
            .then_with(|| a.report_id.cmp(&b.report_id))
    });
    Ok(result)
}

pub fn get_history_report(directory: &Path, report_id: &str) -> Result<ExecutionReport, String> {
    if !valid_report_id(report_id) {
        return Err("Identificador de relatório inválido.".into());
    }
    let dir = storage::open_dir(directory, false, true)?;
    read_report_at(&dir, &format!("{report_id}.json")).map(|(r, _)| r)
}

fn entry(
    id: String,
    created: u64,
    report: &ExecutionReport,
    error: Option<String>,
) -> HistoryEntry {
    HistoryEntry {
        report_id: id,
        created_unix: created,
        plan_id: report.plan_id.clone(),
        action: Some(report.action),
        complete: report.complete,
        cancelled: report.cancelled,
        logical_bytes_processed: report.logical_bytes_processed,
        files_count: report.items.iter().map(|i| i.files.len()).sum(),
        error,
    }
}
fn valid_report_id(id: &str) -> bool {
    id.strip_prefix("report-")
        .is_some_and(|tail| tail.len() == 32 && tail.bytes().all(|b| b.is_ascii_hexdigit()))
}
fn valid_report_name(name: &str) -> bool {
    name.strip_suffix(".json").is_some_and(valid_report_id)
}
fn valid_journal_name(name: &str) -> bool {
    name.strip_prefix("journal-")
        .and_then(|tail| tail.strip_suffix(".jsonl"))
        .is_some_and(|tail| tail.len() == 32 && tail.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn mib(bytes: u64) -> String {
    format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0)).replace('.', ",")
}

// Presentation only: never open recovery paths or turn stored intents into plans.
// Error codes: JOURNAL_UNSAFE (owner/type/mode/links), JOURNAL_TOO_LARGE,
// JOURNAL_BUDGET, JOURNAL_CHANGED, JOURNAL_UNREADABLE; UNKNOWN is reserved for an
// execution whose outcome the journal cannot confirm.
fn inspect_journal(dir: impl AsFd, name: &str, budget: &mut usize) -> Result<(), String> {
    let fd = fs::openat2(
        dir,
        name,
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        fs::Mode::empty(),
        ResolveFlags::BENEATH
            | ResolveFlags::NO_SYMLINKS
            | ResolveFlags::NO_MAGICLINKS
            | ResolveFlags::NO_XDEV,
    )
    .map_err(|e| format!("JOURNAL_UNREADABLE: {e}."))?;
    let before = fs::fstat(&fd).map_err(|e| format!("JOURNAL_UNREADABLE: {e}."))?;
    if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile
        || before.st_nlink != 1
        || before.st_uid != rustix::process::getuid().as_raw()
        || before.st_mode & 0o777 != 0o600
        || before.st_size < 0
    {
        return Err(
            "JOURNAL_UNSAFE: journal não é um arquivo privado válido (regular, do usuário, modo 0600, sem links)."
                .into(),
        );
    }
    let size = before.st_size as u64;
    if size > storage::MAX_JOURNAL_BYTES {
        return Err(format!(
            "JOURNAL_TOO_LARGE: journal grande demais: {} > {}.",
            mib(size),
            mib(storage::MAX_JOURNAL_BYTES)
        ));
    }
    if size as usize > *budget {
        return Err(format!(
            "JOURNAL_BUDGET: leitura do histórico limitada; este journal ({}) não foi inspecionado nesta listagem.",
            mib(size)
        ));
    }
    *budget -= size as usize;
    let file = std::fs::File::from(fd);
    let mut reader = BufReader::new((&file).take(size + 1));
    let mut line = Vec::new();
    let mut total = 0u64;
    let mut finished = false;
    let mut paths = Vec::new();
    loop {
        line.clear();
        let read = reader
            .read_until(b'\n', &mut line)
            .map_err(|e| format!("JOURNAL_UNREADABLE: {e}."))?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if line.len() > storage::MAX_JOURNAL_RECORD + 1 {
            return Err(
                "UNKNOWN: registro de journal acima do limite; inspeção manual necessária.".into(),
            );
        }
        let record = line.strip_suffix(b"\n").unwrap_or(&line);
        if record.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_slice(record)
            .map_err(|_| "UNKNOWN: journal incompleto ou corrompido.".to_string())?;
        if finished {
            return Err("UNKNOWN: registros após encerramento do journal.".into());
        }
        match value.get("phase").and_then(|p| p.as_str()) {
            Some("finished") => {
                let report: ExecutionReport =
                    serde_json::from_value(value.get("report").cloned().unwrap_or_default())
                        .map_err(|_| "UNKNOWN: encerramento inválido.".to_string())?;
                if report.schema_version != 1 {
                    return Err("UNKNOWN: versão de relatório desconhecida.".into());
                }
                finished = true;
            }
            Some("capture_intent") => {
                if let Some(path) = value.get("recovery_path").and_then(|p| p.as_str()) {
                    if paths.len() < 8 && path.len() <= 4096 && !path.chars().any(char::is_control)
                    {
                        paths.push(path.to_owned());
                    }
                }
            }
            Some("plan" | "intent" | "outcome") => {}
            _ => return Err("UNKNOWN: fase de journal desconhecida.".into()),
        }
    }
    let after = fs::fstat(&file).map_err(|e| format!("JOURNAL_UNREADABLE: {e}."))?;
    if total != size
        || inodune_core::scan::Fingerprint::from_stat(&before)
            != inodune_core::scan::Fingerprint::from_stat(&after)
    {
        return Err("JOURNAL_CHANGED: journal alterado durante a leitura.".into());
    }
    if finished {
        Ok(())
    } else {
        Err(format!("UNKNOWN: execução sem encerramento confirmado; pode estar em andamento ou ter sido interrompida. Não repita a limpeza nem restaure automaticamente. Preserve o journal e confira os locais registrados (até 8): {}", paths.join("; ")))
    }
}

fn read_report_at(dir: impl AsFd, name: &str) -> Result<(ExecutionReport, u64), String> {
    let fd = fs::openat2(
        dir,
        name,
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|e| e.to_string())?;
    let stat = fs::fstat(&fd).map_err(|e| e.to_string())?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || stat.st_nlink != 1
        || stat.st_uid != rustix::process::getuid().as_raw()
        || stat.st_mode & 0o777 != 0o600
    {
        return Err("Relatório não é arquivo regular privado de link único.".into());
    }
    if stat.st_size < 0 || stat.st_size as u64 > MAX_REPORT_BYTES {
        return Err("Relatório excede 8 MiB.".into());
    }
    let created = stat.st_mtime.max(0) as u64;
    let file = std::fs::File::from(fd);
    let mut bytes = Vec::with_capacity(stat.st_size as usize);
    file.take(MAX_REPORT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_REPORT_BYTES {
        return Err("Relatório excede 8 MiB.".into());
    }
    let report: ExecutionReport =
        serde_json::from_slice(&bytes).map_err(|e| format!("Relatório inválido: {e}"))?;
    if report.schema_version != 1 {
        return Err(format!(
            "Versão de esquema não suportada: {}",
            report.schema_version
        ));
    }
    Ok((report, created))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        executor::{ExecutionReport, ItemResult},
        plan::Action,
    };
    use std::fs;

    fn sample_report() -> ExecutionReport {
        ExecutionReport {
            schema_version: 1,
            plan_id: "plan-fixture".into(),
            action: Action::Trash,
            items: vec![ItemResult {
                candidate_id: "candidate".into(),
                files: vec![],
                complete: true,
                omitted_files: 0,
            }],
            volumes: vec![],
            complete: true,
            cancelled: false,
            logical_bytes_processed: 17,
            attributed_freed_bytes: None,
            measurement_note: "fixture".into(),
            warnings: vec![],
            history_path: None,
            journal_path: None,
        }
    }
    #[test]
    fn report_id_is_strictly_generated_namespace() {
        assert!(valid_report_id("report-0123456789abcdef0123456789abcdef"));
        assert!(!valid_report_id(
            "../report-0123456789abcdef0123456789abcdef"
        ));
        assert!(!valid_report_id("report-0123456789abcdef0123456789abcg"));
    }

    #[test]
    fn saved_report_is_reread_from_private_disk_boundary() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("reports");
        let saved = storage::save_report(&path, &sample_report()).unwrap();
        let id = saved.file_stem().unwrap().to_str().unwrap();
        let listed = list_history(&path).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].plan_id, "plan-fixture");
        let reread = get_history_report(&path, id).unwrap();
        assert_eq!(reread.logical_bytes_processed, 17);
        assert!(get_history_report(&path, "../anything").is_err());
    }

    #[test]
    fn export_is_private_and_cannot_replace_files_or_follow_symlinks() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("export.json");
        storage::export_report(&destination, &sample_report()).unwrap();
        let bytes = fs::read(&destination).unwrap();
        assert_eq!(
            fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(storage::export_report(&destination, &sample_report()).is_err());
        assert_eq!(fs::read(&destination).unwrap(), bytes);
        let linked = temp.path().join("link.json");
        symlink(&destination, &linked).unwrap();
        assert!(storage::export_report(&linked, &sample_report()).is_err());
        let directory = temp.path().join("linked-dir");
        symlink(temp.path(), &directory).unwrap();
        assert!(storage::export_report(&directory.join("new.json"), &sample_report()).is_err());
        assert!(!temp.path().join("new.json").exists());
    }

    #[test]
    fn corrupt_unknown_schema_and_linked_reports_remain_visible_as_errors() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("reports");
        let saved = storage::save_report(&path, &sample_report()).unwrap();
        let hard = path.join("report-11111111111111111111111111111111.json");
        fs::hard_link(&saved, &hard).unwrap();
        let soft = path.join("report-22222222222222222222222222222222.json");
        symlink(&saved, &soft).unwrap();
        let unknown = path.join("report-33333333333333333333333333333333.json");
        let mut value = serde_json::to_value(sample_report()).unwrap();
        value["schema_version"] = serde_json::json!(99);
        fs::write(&unknown, serde_json::to_vec(&value).unwrap()).unwrap();
        fs::set_permissions(&unknown, fs::Permissions::from_mode(0o600)).unwrap();
        let entries = list_history(&path).unwrap();
        assert_eq!(entries.len(), 4);
        assert!(entries.iter().filter(|e| e.error.is_some()).count() >= 3);
    }

    #[test]
    fn corrupt_report_is_reported_instead_of_empty_history() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("reports");
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        use std::os::unix::fs::PermissionsExt;
        let corrupt = path.join("report-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.json");
        std::fs::write(&corrupt, b"not-json").unwrap();
        std::fs::set_permissions(&corrupt, std::fs::Permissions::from_mode(0o600)).unwrap();
        let entries = list_history(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].error.is_some());
    }

    #[test]
    fn public_permissions_are_reported_and_not_rewritten() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("reports");
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let file = path.join("report-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.json");
        std::fs::write(&file, b"{}").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640)).unwrap();
        let entries = list_history(&path).unwrap();
        assert!(entries[0].error.is_some());
        assert_eq!(
            std::fs::metadata(file).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[test]
    fn journal_capacity_refuses_before_creating_a_new_journal() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("reports");
        let dir = storage::open_dir(&path, true, true).unwrap();
        drop(dir);
        for i in 0..199 {
            std::fs::write(path.join(format!("unrecognized-{i}")), b"keep").unwrap();
        }
        assert!(storage::Journal::create(&path).is_err());
        assert_eq!(std::fs::read_dir(&path).unwrap().count(), 199);
    }
    #[test]
    fn unfinished_journal_is_visible_without_replaying_or_opening_target_paths() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("reports");
        drop(storage::open_dir(&path, true, true).unwrap());
        let journal = path.join("journal-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.jsonl");
        let bytes = b"{\"phase\":\"plan\"}\n{\"phase\":\"capture_intent\",\"recovery_path\":\"/not-opened/fixture-stage\"}\n";
        fs::write(&journal, bytes).unwrap();
        fs::set_permissions(&journal, fs::Permissions::from_mode(0o600)).unwrap();
        let entries = list_history(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].error.as_ref().unwrap().contains("UNKNOWN"));
        assert!(entries[0]
            .error
            .as_ref()
            .unwrap()
            .contains("/not-opened/fixture-stage"));
        assert_eq!(fs::read(&journal).unwrap(), bytes);
    }
    #[test]
    fn finished_journals_are_not_pending_and_linked_or_oversized_journals_get_specific_codes() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("reports");
        drop(storage::open_dir(&path, true, true).unwrap());
        let finished = path.join("journal-11111111111111111111111111111111.jsonl");
        fs::write(
            &finished,
            serde_json::to_vec(&serde_json::json!({"phase":"finished", "report":sample_report()}))
                .unwrap(),
        )
        .unwrap();
        fs::set_permissions(&finished, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(list_history(&path).unwrap().is_empty());
        symlink(
            &finished,
            path.join("journal-22222222222222222222222222222222.jsonl"),
        )
        .unwrap();
        let oversized = path.join("journal-33333333333333333333333333333333.jsonl");
        fs::File::create(&oversized)
            .unwrap()
            .set_len(storage::MAX_JOURNAL_BYTES + 1)
            .unwrap();
        fs::set_permissions(&oversized, fs::Permissions::from_mode(0o600)).unwrap();
        let entries = list_history(&path).unwrap();
        assert_eq!(entries.len(), 2);
        let error = |id: &str| {
            entries
                .iter()
                .find(|e| e.report_id.contains(id))
                .and_then(|e| e.error.clone())
                .unwrap()
        };
        assert!(
            error("2222").contains("JOURNAL_UNREADABLE"),
            "{}",
            error("2222")
        );
        let large = error("3333");
        assert!(large.contains("JOURNAL_TOO_LARGE"), "{large}");
        assert!(
            large.contains("grande demais: 64,0 MiB > 64,0 MiB"),
            "{large}"
        );
        assert!(!large.contains("UNKNOWN"));
    }
    #[test]
    fn large_valid_private_journal_is_read_in_full() {
        use std::{io::Write, os::unix::fs::PermissionsExt};
        // QA: 0600 journal of 16,6 MiB with 41.651 lines (~13.900 files).
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("reports");
        drop(storage::open_dir(&path, true, true).unwrap());
        let journal = path.join("journal-44444444444444444444444444444444.jsonl");
        let mut file = std::io::BufWriter::new(fs::File::create(&journal).unwrap());
        let filler = "x".repeat(380);
        for i in 0..41_650 {
            writeln!(
                file,
                "{{\"phase\":\"intent\",\"path\":\"/fixture/{i}/{filler}\"}}"
            )
            .unwrap();
        }
        writeln!(
            file,
            "{}",
            serde_json::json!({"phase":"finished", "report":sample_report()})
        )
        .unwrap();
        drop(file);
        fs::set_permissions(&journal, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(fs::metadata(&journal).unwrap().len() > 16 * 1024 * 1024);
        assert!(list_history(&path).unwrap().is_empty());
        // The privacy checks still apply to large journals.
        fs::set_permissions(&journal, fs::Permissions::from_mode(0o644)).unwrap();
        let entries = list_history(&path).unwrap();
        assert!(entries[0]
            .error
            .as_ref()
            .unwrap()
            .contains("JOURNAL_UNSAFE"));
    }
}
