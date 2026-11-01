use inodune_core::scan::{self, ApprovedRoot, ScanControl, ScanLimits, ScopeKind};
use std::{path::PathBuf, process::ExitCode, sync::Arc};
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
                .filter(|c| c.selected_by_default && c.risk == inodune_core::Risk::Low)
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
    if values.first() != Some(&"scan") {
        return Err(usage());
    }
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
                kind = match args.get(index).map(String::as_str) {
                    Some("projects") => ScopeKind::Projects,
                    Some("thumbnails") => ScopeKind::Thumbnails,
                    Some("protected") => ScopeKind::Protected,
                    _ => return Err(usage()),
                };
            }
            "--json" => json = true,
            _ => return Err(usage()),
        }
        index += 1;
    }
    if paths.is_empty() || paths.len() > 32 {
        return Err("Informe entre 1 e 32 roots explícitas com --root.".into());
    }
    let roots = paths
        .iter()
        .map(|p| ApprovedRoot::open(p, kind))
        .collect::<Result<Vec<_>, _>>()?;
    let result = scan::scan(&roots, control, &ScanLimits::default(), |_| {})?;
    let partial = !result.complete;
    let output = if json {
        serde_json::to_string_pretty(&result.view()).map_err(|e| e.to_string())?
    } else {
        format!("Análise somente leitura: {} candidatos, {} entradas, {} exclusões/erros. Cobertura {}. Nenhuma limpeza executada.",result.candidates.len(),result.stats.visited,result.stats.skipped,if partial{"parcial"}else{"completa"})
    };
    Ok((output, partial))
}
fn usage() -> String {
    "Uso: inodune-cli scan --root /caminho/aprovado [--root /outra/pasta] [--scope projects|thumbnails|protected] [--json]\nDemonstração: scan --demo --json | clean --demo --dry-run. Limpeza real ainda não habilitada. Exit 2: erro; exit 3: cobertura parcial.".into()
}
#[cfg(test)]
mod tests {
    #[test]
    fn refuses_apply_and_implicit_real_roots() {
        for args in [
            vec!["clean", "--apply"],
            vec!["scan", "--json"],
            vec!["scan", "/home"],
        ] {
            assert!(super::run(args.into_iter().map(str::to_string).collect()).is_err());
        }
    }
}
