use inodune_core::{
    scan::{self, ApprovedRoot, ScanControl, ScanLimits, ScopeKind},
    Risk, SupportedAction,
};
use std::{
    io::{BufRead, IsTerminal, Write},
    path::PathBuf,
    process::ExitCode,
    sync::Arc,
};

fn main() -> ExitCode {
    inodune_clean::diagnostics::install_local_panic_reporter();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{}", usage());
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("inodune-cli {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let control = Arc::new(ScanControl::default());
    let signal_control = control.clone();
    if let Err(e) = ctrlc::set_handler(move || signal_control.cancel()) {
        eprintln!("Não foi possível registrar cancelamento: {e}");
        return ExitCode::from(2);
    }
    match run_with_control(args, &control) {
        Ok((value, partial)) => {
            println!("{value}");
            if partial {
                ExitCode::from(3)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
fn run(args: Vec<String>) -> Result<(String, bool), String> {
    run_with_control(args, &ScanControl::default())
}

fn run_with_control(args: Vec<String>, control: &ScanControl) -> Result<(String, bool), String> {
    let values: Vec<&str> = args.iter().map(String::as_str).collect();
    if values.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        return Ok((usage(), false));
    }
    if values.iter().any(|arg| matches!(*arg, "--version" | "-V")) {
        return Ok((format!("inodune-cli {}", env!("CARGO_PKG_VERSION")), false));
    }
    match values.as_slice() {
        ["scan", "--demo", "--json"] => {
            return inodune_core::demo_inventory()
                .and_then(|i| serde_json::to_string_pretty(&i))
                .map(|s| (s, false))
                .map_err(|e| e.to_string())
        }
        ["clean", "--demo", "--dry-run"] => {
            let inv = inodune_core::demo_inventory().map_err(|e| e.to_string())?;
            let ids = inv
                .candidates
                .iter()
                .filter(|c| c.selected_by_default && c.risk == Risk::Low)
                .map(|c| c.id.clone())
                .collect::<Vec<_>>();
            return serde_json::to_string_pretty(
                &inodune_clean::dry_run(&inv.candidates, &ids).map_err(str::to_string)?,
            )
            .map(|s| (s, false))
            .map_err(|e| e.to_string());
        }
        _ => {}
    }
    match values.first() {
        Some(&"scan") => run_scan(&args, control),
        Some(&"clean") => run_clean(&args, control),
        _ => Err(usage()),
    }
}

fn run_scan(args: &[String], control: &ScanControl) -> Result<(String, bool), String> {
    run_scan_with_discovery(args, control, inodune_core::discovery::discover)
}
fn run_scan_with_discovery(
    args: &[String],
    control: &ScanControl,
    discover: impl FnOnce(
        &ScanControl,
        &ScanLimits,
    ) -> Result<inodune_core::discovery::DiscoveryResult, String>,
) -> Result<(String, bool), String> {
    let mut automatic = false;
    let mut paths = Vec::new();
    let mut exclusions = Vec::new();
    let mut kind = ScopeKind::Projects;
    let mut json = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--auto" => automatic = true,
            "--root" => {
                index += 1;
                paths.push(PathBuf::from(args.get(index).ok_or_else(usage)?));
            }
            "--exclude" => {
                index += 1;
                exclusions.push(PathBuf::from(args.get(index).ok_or_else(usage)?));
            }
            "--scope" => {
                index += 1;
                kind = parse_scope(args.get(index).map(String::as_str).ok_or_else(usage)?)?;
            }
            "--json" => json = true,
            _ => return Err(usage()),
        }
        index += 1;
    }
    if automatic && (!paths.is_empty() || kind != ScopeKind::Projects || !exclusions.is_empty()) {
        return Err("--auto não combina com --root/--scope/--exclude; configure exclusões pelo desktop ou use roots explícitas.".into());
    }
    let limits = ScanLimits::default();
    let result = if automatic {
        inodune_core::discovery::scan_discovered(
            discover(control, &limits)?,
            control,
            &limits,
            |_| {},
        )?
    } else {
        validate_root_count(paths.len())?;
        let roots = paths
            .iter()
            .map(|p| ApprovedRoot::open(p, kind)?.with_exclusions(exclusions.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        scan::scan(&roots, control, &limits, |_| {})?
    };
    let partial = !result.complete;
    let output = if json {
        serde_json::to_string_pretty(&result.view()).map_err(|e| e.to_string())?
    } else {
        format!(
            "Análise somente leitura: {} candidatos, {} entradas, {} exclusões/erros. Cobertura {}. Nenhuma limpeza executada.",
            result.candidates.len(), result.stats.visited, result.stats.skipped,
            if partial { "parcial" } else { "completa" }
        )
    };
    Ok((output, partial))
}

#[derive(serde::Serialize)]
struct CleanView {
    mode: &'static str,
    action: inodune_clean::plan::Action,
    plan: Option<inodune_clean::plan::PlanView>,
    eligible_items: usize,
    protected_or_review_items: usize,
    complete: bool,
}

fn run_clean(args: &[String], control: &ScanControl) -> Result<(String, bool), String> {
    let mut paths = Vec::new();
    let mut exclusions = Vec::new();
    let mut kind = ScopeKind::Projects;
    let mut json = false;
    let mut apply = false;
    let mut dry_run = false;
    let mut review: Option<String> = None;
    let mut action = inodune_clean::plan::Action::Trash;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--root" => {
                index += 1;
                paths.push(PathBuf::from(args.get(index).ok_or_else(usage)?));
            }
            "--exclude" => {
                index += 1;
                exclusions.push(PathBuf::from(args.get(index).ok_or_else(usage)?));
            }
            "--scope" => {
                index += 1;
                kind = parse_scope(args.get(index).map(String::as_str).ok_or_else(usage)?)?;
            }
            "--json" => json = true,
            "--apply" => apply = true,
            "--dry-run" => dry_run = true,
            "--review" => {
                index += 1;
                let detector = args.get(index).ok_or_else(usage)?.clone();
                if !matches!(
                    detector.as_str(),
                    "cargo.fingerprint-json" | "terraform.provider-package"
                ) {
                    return Err(format!("Detector de revisão desconhecido: {detector}. Use cargo.fingerprint-json ou terraform.provider-package."));
                }
                if review.replace(detector).is_some() {
                    return Err("Use --review uma única vez.".into());
                }
            }
            "--permanent" => action = inodune_clean::plan::Action::Permanent,
            _ => return Err(usage()),
        }
        index += 1;
    }
    if apply && dry_run {
        return Err("--apply e --dry-run são opções incompatíveis.".into());
    }
    validate_root_count(paths.len())?;
    let roots = paths
        .iter()
        .map(|p| ApprovedRoot::open(p, kind)?.with_exclusions(exclusions.clone()))
        .collect::<Result<Vec<_>, _>>()?;
    let snapshot = scan::scan(&roots, control, &ScanLimits::default(), |_| {})?;
    if !snapshot.complete || snapshot.cancelled {
        let output = if json {
            serde_json::to_string_pretty(&snapshot.view()).map_err(|e| e.to_string())?
        } else {
            format!(
                "Análise incompleta ou cancelada: {} candidatos. Nenhum plano pode ser executado.",
                snapshot.candidates.len()
            )
        };
        return Ok((output, true));
    }

    let selected = if let Some(detector) = &review {
        let eligible = snapshot
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.detector_id == *detector
                    && candidate.risk == Risk::Review
                    && matches!(
                        &candidate.supported_action,
                        SupportedAction::ReviewedCacheContents
                    )
            })
            .collect::<Vec<_>>();
        match eligible.as_slice() {
            [] => {
                return Err(format!(
                "Nenhum candidato elegível para revisão '{detector}' foi encontrado nesta análise."
            ))
            }
            [candidate] => vec![candidate.id.clone()],
            many => {
                let paths = many
                    .iter()
                    .map(|candidate| format!("  {}", candidate.display_path))
                    .collect::<Vec<_>>()
                    .join("\n");
                return Err(format!("Há {} candidatos elegíveis para '{detector}'; restrinja o root para selecionar um único candidato:\n{paths}", many.len()));
            }
        }
    } else {
        snapshot
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.selected_by_default
                    && candidate.risk == Risk::Low
                    && matches!(&candidate.supported_action, SupportedAction::CacheContents)
            })
            .map(|candidate| candidate.id.clone())
            .collect::<Vec<_>>()
    };
    let protected_or_review_items = snapshot
        .candidates
        .iter()
        .filter(|candidate| candidate.risk != Risk::Low)
        .count();
    if selected.is_empty() {
        let view = CleanView {
            mode: "dry_run",
            action,
            plan: None,
            eligible_items: 0,
            protected_or_review_items,
            complete: true,
        };
        let output = if json {
            serde_json::to_string_pretty(&view).map_err(|e| e.to_string())?
        } else {
            format!(
                "Plano somente leitura: nenhum item elegível; {} itens protegidos ou para revisão. Nenhuma limpeza executada.",
                protected_or_review_items
            )
        };
        return Ok((output, false));
    }

    let session = inodune_clean::plan::PlanSession::prepare(Arc::new(snapshot), &selected, action)?;
    let plan_view = session.view();
    if !apply {
        let view = CleanView {
            mode: "dry_run",
            action,
            eligible_items: plan_view.items.len(),
            protected_or_review_items,
            complete: true,
            plan: Some(plan_view),
        };
        let output = if json {
            serde_json::to_string_pretty(&view).map_err(|e| e.to_string())?
        } else {
            format_plan(&view)
        };
        return Ok((output, false));
    }

    confirm_plan(&plan_view)?;
    let authorized = session.consume(&plan_view.hash)?;
    let options = inodune_clean::executor::ExecutionOptions::local()?;
    let report = inodune_clean::executor::execute(authorized, &options, control, |_| {});
    let partial = !report.complete || report.cancelled;
    let output = if json {
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
    } else {
        let mut text = format!(
            "Execução {}: {} itens, {} bytes lógicos processados.\n{}",
            if partial { "parcial" } else { "completa" },
            report.items.len(),
            report.logical_bytes_processed,
            report.measurement_note
        );
        for item in &report.items {
            for file in &item.files {
                text.push_str(&format!(
                    "\n{:?}: {} — {}",
                    file.status, file.display_path, file.message
                ));
                if let Some(path) = &file.preserved_at {
                    text.push_str(&format!("\n  Preservado em: {path}"));
                }
            }
        }
        for volume in &report.volumes {
            text.push_str(&format!(
                "\nDevice {}: variação observada de disponibilidade {:?} bytes (não atribuída).",
                volume.device, volume.observed_delta
            ));
        }
        for warning in &report.warnings {
            text.push_str(&format!("\nAviso: {warning}"));
        }
        if let Some(path) = &report.history_path {
            text.push_str(&format!("\nRelatório: {path}"));
        }
        text
    };
    Ok((output, partial))
}

fn confirm_plan(plan: &inodune_clean::plan::PlanView) -> Result<(), String> {
    let stdin = std::io::stdin();
    if !stdin.is_terminal() {
        return Err(
            "Aplicação exige stdin interativo para confirmação; nenhum item foi alterado.".into(),
        );
    }
    let expected = match plan.action {
        inodune_clean::plan::Action::Trash => format!("TRASH {}", plan.items.len()),
        inodune_clean::plan::Action::Permanent => format!("DELETE {}", plan.items.len()),
    };
    eprintln!(
        "Plano {} | hash {} | ação {:?} | {} itens | risco {} | impacto estimado {} bytes lógicos / {} bytes alocados",
        plan.id,
        plan.hash,
        plan.action,
        plan.items.len(),
        if plan.review_required { "revisão" } else { "baixo" },
        plan.logical_bytes_estimate,
        plan.allocated_bytes_estimate.map(|n| n.to_string()).unwrap_or_else(|| "desconhecido".into())
    );
    for item in &plan.items {
        eprintln!(
            "{} | {:?} | {} bytes lógicos | impacto: {} | reconstrução: {}",
            item.display_path, item.risk, item.logical_bytes, item.impact, item.rebuild_hint
        );
    }
    eprint!("Digite exatamente '{expected}' para confirmar: ");
    std::io::stderr().flush().map_err(|e| e.to_string())?;
    let mut input = String::new();
    stdin
        .lock()
        .read_line(&mut input)
        .map_err(|e| format!("Falha ao ler confirmação: {e}"))?;
    if !confirmation_matches(&input, &expected) {
        return Err("Confirmação não corresponde; nenhum item foi alterado.".into());
    }
    Ok(())
}

fn confirmation_matches(input: &str, expected: &str) -> bool {
    let line = input
        .strip_suffix("\r\n")
        .or_else(|| input.strip_suffix('\n'))
        .or_else(|| input.strip_suffix('\r'))
        .unwrap_or(input);
    line == expected
}

fn format_plan(view: &CleanView) -> String {
    let Some(plan) = &view.plan else {
        return format!(
            "Plano somente leitura: nenhum item elegível; {} itens protegidos ou para revisão. Nenhuma limpeza executada.",
            view.protected_or_review_items
        );
    };
    let details = plan
        .items
        .iter()
        .map(|item| {
            format!(
                "  {} | {:?} | {} bytes lógicos | impacto: {} | reconstrução: {}",
                item.display_path, item.risk, item.logical_bytes, item.impact, item.rebuild_hint
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Plano somente leitura: {} itens, hash {}. Ação {:?}; risco {}; impacto estimado {} bytes lógicos / {} bytes alocados.\n{}\nNenhuma limpeza executada.",
        plan.items.len(),
        plan.hash,
        plan.action,
        if plan.review_required { "revisão" } else { "baixo" },
        plan.logical_bytes_estimate,
        plan.allocated_bytes_estimate.map(|n| n.to_string()).unwrap_or_else(|| "desconhecido".into()),
        details
    )
}

fn parse_scope(value: &str) -> Result<ScopeKind, String> {
    match value {
        "projects" => Ok(ScopeKind::Projects),
        "thumbnails" => Ok(ScopeKind::Thumbnails),
        "protected" => Ok(ScopeKind::Protected),
        _ => Err(usage()),
    }
}

fn validate_root_count(count: usize) -> Result<(), String> {
    if count == 0 || count > 32 {
        Err("Informe entre 1 e 32 roots explícitas com --root.".into())
    } else {
        Ok(())
    }
}

fn usage() -> String {
    "Uso: inodune-cli [--help|-h] [--version|-V]\n     inodune-cli scan --auto [--json]\n     inodune-cli scan --root ABS [--root ABS] [--exclude REL]... [--scope projects|thumbnails|protected] [--json]\n     inodune-cli clean --root ABS [--root ABS] [--exclude REL]... [--scope projects|thumbnails|protected] [--dry-run|--apply] [--review cargo.fingerprint-json|terraform.provider-package] [--permanent] [--json]\nDemonstração: scan --demo --json | clean --demo --dry-run. --exclude aceita caminhos relativos literais; '.' exclui a root inteira. --review seleciona um único candidato verificado e não aplica a limpeza; por padrão, apenas itens de baixo risco entram no plano. A limpeza real começa em simulação; --apply exige confirmação interativa e digitação exata. Exit 2: erro; exit 3: cobertura ou execução parcial.".into()
}

#[cfg(test)]
mod tests {
    use super::{confirmation_matches, run};
    use std::fs;

    #[test]
    fn automatic_scan_uses_discovery_without_paths_or_cleanup() {
        let temp = tempfile::tempdir().unwrap();
        let project = cargo_fixture(temp.path(), "automatic");
        let (output, _) = super::run_scan_with_discovery(
            &["scan".into(), "--auto".into(), "--json".into()],
            &Default::default(),
            |control, limits| {
                inodune_core::discovery::discover_from(temp.path(), "", &[], control, limits)
            },
        )
        .unwrap();
        assert!(output.contains("cargo.fingerprint-json"));
        assert!(output.contains("read_only"));
        assert!(project.join("Cargo.toml").exists());
        assert!(run(vec![
            "scan".into(),
            "--auto".into(),
            "--root".into(),
            temp.path().to_string_lossy().into_owned()
        ])
        .is_err());
    }

    fn cargo_fixture(parent: &std::path::Path, name: &str) -> std::path::PathBuf {
        let project = parent.join(name);
        let json = project.join(format!(
            "target/debug/.fingerprint/{name}-0123456789abcdef/lib-{name}.json"
        ));
        fs::create_dir_all(json.parent().unwrap()).unwrap();
        fs::write(
            project.join("Cargo.toml"),
            format!("[package]\nname=\"{name}\"\nversion=\"0.1.0\"\n"),
        )
        .unwrap();
        let value = serde_json::json!({"rustc":1,"features":"[]","declared_features":"[]","target":2,"profile":3,"path":4,"deps":[],"local":[],"rustflags":[],"config":5,"compile_kind":0});
        fs::write(&json, serde_json::to_vec(&value).unwrap()).unwrap();
        fs::write(json.with_extension(""), b"0123456789abcdef").unwrap();
        project
    }

    #[test]
    fn requires_exact_confirmation_text() {
        assert!(confirmation_matches("TRASH 2\n", "TRASH 2"));
        assert!(!confirmation_matches("TRASH 2 extra\n", "TRASH 2"));
        assert!(!confirmation_matches("trash 2\n", "TRASH 2"));
    }

    #[test]
    fn refuses_missing_roots_and_conflicting_apply_modes() {
        for args in [
            vec!["scan", "--json"],
            vec!["clean", "--apply", "--dry-run"],
        ] {
            assert!(run(args.into_iter().map(str::to_string).collect()).is_err());
        }
    }

    #[test]
    fn scan_and_clean_apply_repeatable_relative_exclusions() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("app/__pycache__")).unwrap();
        fs::write(root.path().join("app/main.py"), b"pass").unwrap();
        fs::write(root.path().join("app/__pycache__/note.txt"), b"private").unwrap();
        let scan = run(vec![
            "scan".into(),
            "--root".into(),
            root.path().to_string_lossy().into(),
            "--exclude".into(),
            "app/__pycache__".into(),
            "--json".into(),
        ])
        .unwrap();
        assert!(scan.0.contains("policy_excluded"));
        let clean = run(vec![
            "clean".into(),
            "--root".into(),
            root.path().to_string_lossy().into(),
            "--exclude".into(),
            "app".into(),
            "--dry-run".into(),
            "--json".into(),
        ])
        .unwrap();
        assert!(clean.0.contains("\"complete\": true"));
        assert!(clean.0.contains("\"eligible_items\": 0"));
        assert!(run(vec![
            "scan".into(),
            "--root".into(),
            root.path().to_string_lossy().into(),
            "--exclude".into(),
            "../escape".into()
        ])
        .is_err());
    }

    #[test]
    fn review_selects_one_exact_detector_for_dry_run() {
        let temp = tempfile::tempdir().unwrap();
        cargo_fixture(temp.path(), "fixture");
        let (output, partial) = run(vec![
            "clean",
            "--root",
            temp.path().to_str().unwrap(),
            "--review",
            "cargo.fingerprint-json",
            "--dry-run",
        ]
        .into_iter()
        .map(str::to_string)
        .collect())
        .unwrap();
        assert!(!partial);
        assert!(output.contains("risco revisão"));
        assert!(output.contains("fingerprint"));
        assert!(output.contains("impacto:"));
        assert!(output.contains("reconstrução:"));
        assert!(output.contains("Nenhuma limpeza executada."));
    }

    #[test]
    fn review_rejects_multiple_candidates_with_paths() {
        let temp = tempfile::tempdir().unwrap();
        cargo_fixture(temp.path(), "one");
        cargo_fixture(temp.path(), "two");
        let error = run(vec![
            "clean",
            "--root",
            temp.path().to_str().unwrap(),
            "--review",
            "cargo.fingerprint-json",
        ]
        .into_iter()
        .map(str::to_string)
        .collect())
        .unwrap_err();
        assert!(error.contains("restrinja o root"));
        assert!(error.contains("lib-one.json"));
        assert!(error.contains("lib-two.json"));
    }

    #[test]
    fn rejects_unknown_review_detector_and_help_is_available() {
        assert!(run(
            vec!["clean", "--root", "/tmp", "--review", "arbitrary.path"]
                .into_iter()
                .map(str::to_string)
                .collect()
        )
        .unwrap_err()
        .contains("desconhecido"));
        let (help, partial) = run(vec!["--help".into()]).unwrap();
        assert!(!partial);
        assert!(help.contains("--review"));
        assert!(help.contains("--version"));
    }
}
