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
    let control = Arc::new(ScanControl::default());
    let signal_control = control.clone();
    if let Err(e) = ctrlc::set_handler(move || signal_control.cancel()) {
        eprintln!("Não foi possível registrar cancelamento: {e}");
        return ExitCode::from(2);
    }
    match run_with_control(std::env::args().skip(1).collect(), &control) {
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
    let mut paths = Vec::new();
    let mut kind = ScopeKind::Projects;
    let mut json = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--root" => {
                index += 1;
                paths.push(PathBuf::from(args.get(index).ok_or_else(usage)?));
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
    validate_root_count(paths.len())?;
    let roots = paths
        .iter()
        .map(|p| ApprovedRoot::open(p, kind))
        .collect::<Result<Vec<_>, _>>()?;
    let result = scan::scan(&roots, control, &ScanLimits::default(), |_| {})?;
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
    let mut kind = ScopeKind::Projects;
    let mut json = false;
    let mut apply = false;
    let mut dry_run = false;
    let mut action = inodune_clean::plan::Action::Trash;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--root" => {
                index += 1;
                paths.push(PathBuf::from(args.get(index).ok_or_else(usage)?));
            }
            "--scope" => {
                index += 1;
                kind = parse_scope(args.get(index).map(String::as_str).ok_or_else(usage)?)?;
            }
            "--json" => json = true,
            "--apply" => apply = true,
            "--dry-run" => dry_run = true,
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
        .map(|p| ApprovedRoot::open(p, kind))
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

    let selected = snapshot
        .candidates
        .iter()
        .filter(|candidate| {
            candidate.selected_by_default
                && candidate.risk == Risk::Low
                && matches!(&candidate.supported_action, SupportedAction::CacheContents)
        })
        .map(|candidate| candidate.id.clone())
        .collect::<Vec<_>>();
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
        "Plano {} | hash {} | ação {:?} | {} itens",
        plan.id,
        plan.hash,
        plan.action,
        plan.items.len()
    );
    for item in &plan.items {
        eprintln!("{}", item.display_path);
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
    let paths = plan
        .items
        .iter()
        .map(|item| item.display_path.as_str())
        .collect::<Vec<_>>()
        .join("\n  ");
    format!(
        "Plano somente leitura: {} itens, hash {}. Ação {:?}.\n  {}\nNenhuma limpeza executada.",
        plan.items.len(),
        plan.hash,
        plan.action,
        paths
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
    "Uso: inodune-cli scan --root ABS [--root ABS] [--scope projects|thumbnails|protected] [--json]\n     inodune-cli clean --root ABS [--root ABS] [--scope projects|thumbnails|protected] [--dry-run|--apply] [--permanent] [--json]\nDemonstração: scan --demo --json | clean --demo --dry-run. A limpeza real começa em simulação; --apply exige confirmação interativa. Exit 2: erro; exit 3: cobertura ou execução parcial.".into()
}

#[cfg(test)]
mod tests {
    use super::{confirmation_matches, run};

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
}
