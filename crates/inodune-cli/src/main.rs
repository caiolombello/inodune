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
        return write_output(&mut std::io::stdout().lock(), &usage(), false);
    }
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        return write_output(
            &mut std::io::stdout().lock(),
            &format!("inodune-cli {}", env!("CARGO_PKG_VERSION")),
            false,
        );
    }
    let control = Arc::new(ScanControl::default());
    let signal_control = control.clone();
    if let Err(e) = ctrlc::set_handler(move || signal_control.cancel()) {
        eprintln!("Não foi possível registrar cancelamento: {e}");
        return ExitCode::from(2);
    }
    match run_with_control(args, &control) {
        Ok((value, partial)) => write_output(&mut std::io::stdout().lock(), &value, partial),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn write_output(output: &mut impl Write, value: &str, partial: bool) -> ExitCode {
    match writeln!(output, "{value}").and_then(|_| output.flush()) {
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(2),
        Ok(()) if partial => ExitCode::from(3),
        Ok(()) => ExitCode::SUCCESS,
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
        ["apps", "list"] | ["apps", "list", "--json"] => {
            return present_apps(
                &inodune_apps::inventory()?,
                values.last() == Some(&"--json"),
            );
        }
        ["apps", "uninstall", ..] => return uninstall_apps(&values[2..]),

        ["scan", "--demo", "--json"] => {
            return inodune_core::demo_inventory()
                .and_then(|i| serde_json::to_string_pretty(&i))
                .map(|s| (s, false))
                .map_err(|e| e.to_string())
        }
        ["clean", "--demo"] | ["clean", "--demo", "--dry-run"] => {
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
    let mut safe = false;
    let mut yes = false;
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
                let requested = args.get(index).ok_or_else(usage)?;
                let detector = review_detector(requested).ok_or_else(|| {
                    format!(
                        "Detector de revisão desconhecido: {requested}. IDs aceitos (como na análise): {}.",
                        REVIEW_DETECTORS.join(", ")
                    )
                })?;
                if review.replace(detector).is_some() {
                    return Err("Use --review uma única vez.".into());
                }
            }
            "--permanent" => action = inodune_clean::plan::Action::Permanent,
            "--safe" => safe = true,
            "--yes" => yes = true,
            _ => return Err(usage()),
        }
        index += 1;
    }
    if !dry_run {
        apply = true;
    }
    if apply && dry_run {
        return Err("--apply e --dry-run são opções incompatíveis.".into());
    }
    if yes && !apply {
        return Err("--yes não combina com --dry-run.".into());
    }
    validate_root_count(paths.len())?;
    let roots = paths
        .iter()
        .map(|p| ApprovedRoot::open(p, kind)?.with_exclusions(exclusions.clone()))
        .collect::<Result<Vec<_>, _>>()?;
    let snapshot = scan::scan(&roots, control, &ScanLimits::default(), |_| {})?;
    // Fail-closed: any incomplete or cancelled analysis refuses every plan
    // (review, safe and default), listing the reasons.
    if snapshot.cancelled || !snapshot.complete {
        let all: Vec<&scan::ScanIssue> = snapshot.issues.iter().collect();
        return Ok((refuse_incomplete(&snapshot, &all, json)?, true));
    }

    let selected = if safe {
        snapshot
            .candidates
            .iter()
            .filter(|candidate| {
                is_safe_detector(&candidate.detector_id)
                    && candidate.risk == Risk::Low
                    && !matches!(candidate.supported_action, SupportedAction::None)
            })
            .map(|candidate| candidate.id.clone())
            .collect::<Vec<_>>()
    } else if let Some(detector) = &review {
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
        if eligible.is_empty() {
            // Repeat why each candidate of this detector is not eligible.
            let reasons = snapshot
                .candidates
                .iter()
                .filter(|candidate| candidate.detector_id == *detector)
                .map(|candidate| {
                    format!(
                        "  {} ({}): {}",
                        scan::tidy_display_path(&candidate.display_path),
                        risk_label(&candidate.risk),
                        candidate.reason
                    )
                })
                .collect::<Vec<_>>();
            let detail = if reasons.is_empty() {
                String::new()
            } else {
                format!(" Motivos:\n{}", reasons.join("\n"))
            };
            return Err(format!(
                "Nenhum candidato elegível para revisão '{detector}' foi encontrado nesta análise.{detail}"
            ));
        }
        // Every candidate of the detector goes into one plan (use --root to
        // narrow it). The analysis is complete here (checked above).
        eligible
            .iter()
            .map(|candidate| candidate.id.clone())
            .collect::<Vec<_>>()
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
        // An apply request with nothing eligible is not a dry run.
        let view = CleanView {
            mode: if apply { "nothing_eligible" } else { "dry_run" },
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
                "{}; {} protegidos ou para revisão. Nenhuma limpeza executada.",
                if apply {
                    "Nada elegível para limpar"
                } else {
                    "Plano somente leitura: nenhum item elegível"
                },
                count(protected_or_review_items, "item", "itens")
            )
        };
        return Ok((output, false));
    }

    let mounts: Vec<(u64, String)> = snapshot
        .volumes
        .iter()
        .map(|v| (v.device, scan::tidy_display_path(&v.scope_path)))
        .collect();
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

    let options = inodune_clean::executor::ExecutionOptions::local()?;
    inodune_clean::executor::preflight(&options, action)?;
    if !yes {
        confirm_plan(&plan_view)?;
    }
    let authorized = session.consume(&plan_view.hash)?;
    let report = inodune_clean::executor::execute(authorized, &options, control, |_| {});
    let partial = !report.complete || report.cancelled;
    let output = if json {
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
    } else {
        let mut text = format!(
            "Execução {}: {}, {} processados.\n{}",
            if partial { "parcial" } else { "completa" },
            count(report.items.len(), "item", "itens"),
            inodune_apps::presentation::human_bytes(report.logical_bytes_processed),
            report.measurement_note
        );
        for item in &report.items {
            for file in &item.files {
                text.push_str(&format!(
                    "\n{}: {} — {}",
                    status_label(&file.status),
                    scan::tidy_display_path(&file.display_path),
                    os_error_text(&file.message)
                ));
                if let Some(path) = &file.preserved_at {
                    text.push_str(&format!(
                        "\n  Preservado em: {}",
                        scan::tidy_display_path(path)
                    ));
                }
            }
        }
        for volume in &report.volumes {
            let delta = match volume.observed_delta {
                Some(delta) if delta < 0 => format!(
                    "−{}",
                    inodune_apps::presentation::human_bytes(
                        u64::try_from(delta.unsigned_abs()).unwrap_or(u64::MAX)
                    )
                ),
                Some(delta) => format!(
                    "+{}",
                    inodune_apps::presentation::human_bytes(
                        u64::try_from(delta.unsigned_abs()).unwrap_or(u64::MAX)
                    )
                ),
                None => "não medida".to_owned(),
            };
            let name = mounts
                .iter()
                .find(|(device, _)| *device == volume.device)
                .map_or("analisado", |(_, path)| path.as_str());
            text.push_str(&format!(
                "\nVolume {name}: variação observada de espaço disponível {delta} (não atribuída à limpeza)."
            ));
        }
        for warning in &report.warnings {
            text.push_str(&format!("\nAviso: {}", os_error_text(warning)));
        }
        if let Some(path) = &report.history_path {
            text.push_str(&format!("\nRelatório: {path}"));
        }
        text
    };
    Ok((output, partial))
}

fn is_safe_detector(id: &str) -> bool {
    matches!(
        id,
        "python.pycache"
            | "thumbnails"
            | "python.pip-cache"
            | "npm.cache"
            | "next.cache"
            | "python.mypy-cache"
            | "python.ruff-cache"
            | "uv.cache"
            | "trash.entry"
            | "rust.target"
            | "cargo.fingerprint-json"
            | "terraform.provider-package"
    )
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
        "Plano {} | hash {} | ação: {} | {} | risco {} | impacto estimado {} lógicos / {} alocados\n{}",
        plan.id,
        plan.hash,
        plan.action.label(false),
        count(plan.items.len(), "item", "itens"),
        if plan.review_required { "revisão" } else { "baixo" },
        inodune_apps::presentation::human_bytes(plan.logical_bytes_estimate),
        plan.allocated_bytes_estimate
            .map(inodune_apps::presentation::human_bytes)
            .unwrap_or_else(|| "desconhecido".into()),
        plan.action.consequence(false)
    );
    for item in &plan.items {
        eprintln!(
            "{} | {} | {} | impacto: {} | reconstrução: {}",
            scan::tidy_display_path(&item.display_path),
            risk_label(&item.risk),
            inodune_apps::presentation::human_bytes(item.logical_bytes),
            item.impact,
            item.rebuild_hint
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
            "Plano somente leitura: nenhum item elegível; {} protegidos ou para revisão. Nenhuma limpeza executada.",
            count(view.protected_or_review_items, "item", "itens")
        );
    };
    let details = plan
        .items
        .iter()
        .map(|item| {
            format!(
                "  {} | {} | {} | impacto: {} | reconstrução: {}",
                scan::tidy_display_path(&item.display_path),
                risk_label(&item.risk),
                inodune_apps::presentation::human_bytes(item.logical_bytes),
                item.impact,
                item.rebuild_hint
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Plano somente leitura: {}, hash {}. Ação: {}; risco {}; impacto estimado {} lógicos / {} alocados.\n{}\n{}\nNenhuma limpeza executada.",
        count(plan.items.len(), "item", "itens"),
        plan.hash,
        plan.action.label(false),
        if plan.review_required { "revisão" } else { "baixo" },
        inodune_apps::presentation::human_bytes(plan.logical_bytes_estimate),
        plan.allocated_bytes_estimate
            .map(inodune_apps::presentation::human_bytes)
            .unwrap_or_else(|| "desconhecido".into()),
        plan.action.consequence(false),
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

#[cfg(feature = "container-dpkg-test")]
fn container_marker() -> bool {
    std::path::Path::new("/run/.containerenv").exists()
        || std::path::Path::new("/.dockerenv").exists()
}
#[cfg(feature = "container-dpkg-test")]
fn dpkg_query_blocks(record: &str) -> bool {
    let mut fields = record.split('\t');
    let essential = fields.next().unwrap_or("").trim();
    let priority = fields.next().unwrap_or("").trim();
    essential.eq_ignore_ascii_case("yes") || priority.eq_ignore_ascii_case("required")
}
#[cfg(feature = "container-dpkg-test")]
fn dpkg_direct(ids: &[String], yes: bool) -> Result<(String, bool), String> {
    if !yes {
        return Err("dpkg direto no container exige --yes. Nada foi removido.".into());
    }
    if !container_marker() {
        return Err("dpkg direto só é aceito dentro de um container. Nada foi removido.".into());
    }
    let uid = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let root = uid
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .is_some_and(|rest| rest.split_whitespace().next() == Some("0"));
    if !root {
        return Err("dpkg direto no container exige root. Nada foi removido.".into());
    }
    for id in ids {
        inodune_apps::uninstall::validate_id(inodune_apps::Source::Apt, id)?;
        if inodune_apps::uninstall::package_blocked(id) {
            return Err(format!("Pacote bloqueado: {id}"));
        }
        let query = std::process::Command::new("/usr/bin/dpkg-query")
            .args(["-W", "-f=${Essential}\t${Priority}\n", "--", id])
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|e| format!("dpkg-query indisponível: {e}"))?;
        if !query.status.success() {
            return Err(format!("dpkg-query recusou {id} antes da remoção"));
        }
        let record = String::from_utf8_lossy(&query.stdout);
        if dpkg_query_blocks(record.trim()) {
            return Err(format!(
                "Pacote essencial ou de prioridade required: {id}. Nada foi removido."
            ));
        }
    }
    let status = std::process::Command::new("/usr/bin/dpkg")
        .arg("--remove")
        .arg("--")
        .args(ids)
        .stdin(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("dpkg indisponível: {e}"))?;
    if status.success() {
        Ok((format!("dpkg remove concluído: {}", ids.join(" ")), false))
    } else {
        Err(format!("dpkg recusou a remoção: {status}"))
    }
}
fn uninstall_apps(args: &[&str]) -> Result<(String, bool), String> {
    let mut yes = false;
    let mut ids = Vec::new();
    for arg in args {
        if *arg == "--yes" {
            yes = true;
        } else if arg.starts_with('-') {
            return Err(usage());
        } else {
            ids.push((*arg).to_owned());
        }
    }
    if ids.is_empty() {
        return Err("Informe ao menos um ID. Uso: inodune-cli apps uninstall ID... [--yes]".into());
    }
    #[cfg(feature = "container-dpkg-test")]
    if std::env::var("INODUNE_CONTAINER_DPKG").ok().as_deref() == Some("1") {
        return dpkg_direct(&ids, yes);
    }
    let inventory = inodune_apps::inventory()?;
    let mut grouped: std::collections::BTreeMap<inodune_apps::Source, Vec<String>> =
        std::collections::BTreeMap::new();
    for id in &ids {
        let item = inventory
            .sources
            .iter()
            .flat_map(|source| source.items.iter())
            .find(|item| item.id == *id)
            .ok_or_else(|| format!("ID não está no inventário: {id}"))?;
        if item.protected {
            return Err(format!("Pacote bloqueado: {id}"));
        }
        grouped.entry(item.source).or_default().push(id.clone());
    }
    if !yes && !std::io::stdin().is_terminal() {
        return Err(
            "Desinstalação exige --yes ou um terminal para confirmar. Nada foi removido.".into(),
        );
    }
    // Simulate first so the person sees packages, dependencies and size before typing.
    let all_items: Vec<inodune_apps::InstalledItem> = inventory
        .sources
        .into_iter()
        .flat_map(|source| source.items)
        .collect();
    let mut prepared = Vec::new();
    let mut packages = 0;
    for (source, group) in grouped {
        let request = inodune_apps::uninstall::Request {
            source,
            id: group[0].clone(),
            ids: group.clone(),
        };
        let mut transport = inodune_apps::uninstall::SystemTransport;
        let session = inodune_apps::uninstall::Session::prepare(request, &mut transport)?;
        let view = session.view();
        let details = inodune_apps::presentation::describe(source, &view.items, &group, &all_items);
        packages += details.len();
        eprintln!(
            "{}\n",
            inodune_apps::presentation::confirmation_text(&details, view.steam_handoff)
        );
        prepared.push((session, view.id));
    }
    if !yes {
        let stdin = std::io::stdin();
        eprintln!(
            "Plano válido por 2 minutos. Digite UNINSTALL {} para confirmar ({} no total).",
            ids.len(),
            if packages == 1 {
                "1 pacote".to_owned()
            } else {
                format!("{packages} pacotes")
            }
        );
        let mut line = String::new();
        stdin
            .lock()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if line.trim() != format!("UNINSTALL {}", ids.len()) {
            return Err("Confirmação não corresponde; nada foi removido.".into());
        }
    }
    for (session, plan_id) in prepared {
        let mut transport = inodune_apps::uninstall::SystemTransport;
        session.execute(&plan_id, &mut transport)?;
    }
    Ok((
        format!(
            "Desinstalação concluída: {}.",
            if packages == 1 {
                "1 pacote".to_owned()
            } else {
                format!("{packages} pacotes")
            }
        ),
        false,
    ))
}
fn present_apps(inventory: &inodune_apps::Inventory, json: bool) -> Result<(String, bool), String> {
    let partial = inventory
        .sources
        .iter()
        .any(|source| matches!(source.status.as_str(), "partial" | "error"));
    let output = if json {
        serde_json::to_string_pretty(inventory).map_err(|e| e.to_string())?
    } else {
        let mut lines = vec!["Programas & Jogos · somente leitura".to_string()];
        for source in &inventory.sources {
            lines.push(format!(
                "{}: {} ({})",
                source.source,
                source.status,
                count(source.items.len(), "item", "itens")
            ));
            for item in &source.items {
                let size = match item.size_bytes {
                    Some(bytes) => inodune_apps::presentation::human_bytes(bytes),
                    None => "tamanho desconhecido".to_string(),
                };
                lines.push(format!(
                    "{:?} | {} | {} | {size} | uso {:?} | protegido={}",
                    item.source, item.id, item.name, item.signals, item.protected
                ));
            }
            lines.extend(source.warnings.iter().cloned());
        }
        lines.join("\n")
    };
    Ok((output, partial))
}

/// Count per issue code plus a few example paths.
fn issue_summary(issues: &[&scan::ScanIssue]) -> String {
    if issues.is_empty() {
        return "  (sem avisos registrados; a análise foi interrompida ou excedeu limites)".into();
    }
    let mut by_code: std::collections::BTreeMap<&str, Vec<&scan::ScanIssue>> =
        std::collections::BTreeMap::new();
    for issue in issues {
        by_code.entry(issue.code.as_str()).or_default().push(issue);
    }
    by_code
        .iter()
        .map(|(code, list)| {
            let examples = list
                .iter()
                .take(3)
                .map(|i| {
                    format!(
                        "{} ({})",
                        scan::tidy_display_path(&i.display_path),
                        i.message
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            format!("  {code}: {} — ex.: {examples}", list.len())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn refuse_incomplete(
    snapshot: &scan::ScanSnapshot,
    issues: &[&scan::ScanIssue],
    json: bool,
) -> Result<String, String> {
    if json {
        return serde_json::to_string_pretty(&snapshot.view()).map_err(|e| e.to_string());
    }
    let state = if snapshot.cancelled {
        "Análise cancelada"
    } else {
        "Análise incompleta"
    };
    Ok(format!(
        "{state}: {}. Nenhum plano pode ser executado. Motivos:\n{}",
        count(snapshot.candidates.len(), "candidato", "candidatos"),
        issue_summary(issues)
    ))
}

fn risk_label(risk: &Risk) -> &'static str {
    match risk {
        Risk::Low => "risco baixo",
        Risk::Review => "revisão",
        Risk::Protected => "protegido",
    }
}

fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn status_label(status: &inodune_clean::executor::FileStatus) -> &'static str {
    use inodune_clean::executor::FileStatus as S;
    match status {
        S::Trashed => "Movido à Lixeira",
        S::Removed => "Removido",
        S::SkippedChanged => "Alterado; preservado",
        S::Failed => "Falhou; preservado",
        S::Cancelled => "Cancelado; preservado",
        S::Quarantined => "Quarentena; preservado",
    }
}

fn os_error_text(message: &str) -> String {
    scan::os_error_pt(message)
}

/// Reviewable cache detectors, exactly as `scan` reports them.
const REVIEW_DETECTORS: [&str; 10] = [
    "cargo.fingerprint-json",
    "terraform.provider-package",
    "python.pytest-nodeids",
    "rust.target",
    "python.pip-cache",
    "npm.cache",
    "next.cache",
    "python.mypy-cache",
    "python.ruff-cache",
    "node.modules",
];

/// Scan id, or the same id with `_`/`-` instead of `.` (`node_modules`).
fn review_detector(requested: &str) -> Option<String> {
    let normal = requested
        .trim()
        .to_ascii_lowercase()
        .replace(['_', '-'], ".");
    REVIEW_DETECTORS
        .iter()
        .find(|id| id.replace(['_', '-'], ".") == normal)
        .map(|id| (*id).to_owned())
}

fn usage() -> String {
    "Uso: inodune-cli apps list [--json]\n     inodune-cli apps uninstall ID... [--yes]\n     inodune-cli [--help|-h] [--version|-V]\n     inodune-cli scan --auto [--json]\n     inodune-cli scan --root ABS [--root ABS] [--exclude REL]... [--scope projects|thumbnails|protected] [--json]\n     inodune-cli clean --root ABS [--root ABS] [--exclude REL]... [--scope projects|thumbnails|protected] [--dry-run|--apply] [--safe] [--yes] [--review DETECTOR] [--permanent] [--json]\nDemonstração (dados fictícios, nada é lido nem alterado): scan --demo --json | clean --demo [--dry-run]. --exclude aceita caminhos relativos literais; '.' exclui a root inteira. clean executa depois de uma confirmação; --dry-run é explícito e não é o padrão. --safe escolhe o conjunto de um clique. --yes não pergunta. --permanent exclui sem passar pela Lixeira (irreversível). apps uninstall mostra o plano (pacotes, dependências e tamanho) antes de pedir confirmação. Exit 2: erro; exit 3: cobertura ou execução parcial.".into()
}

#[cfg(test)]
mod tests {
    use super::{confirmation_matches, present_apps, run};
    use std::fs;

    #[cfg(feature = "container-dpkg-test")]
    use super::dpkg_query_blocks;

    #[test]
    #[cfg(feature = "container-dpkg-test")]
    fn dpkg_query_refuses_essential_and_required_priority() {
        assert!(dpkg_query_blocks("yes\toptional"));
        assert!(dpkg_query_blocks("no\trequired"));
        assert!(dpkg_query_blocks("\trequired"));
        assert!(!dpkg_query_blocks("no\toptional"));
        assert!(!dpkg_query_blocks("\timportant"));
    }

    #[test]
    fn broken_pipe_exits_cleanly_without_hiding_other_output_errors() {
        struct Failed(std::io::ErrorKind);
        impl std::io::Write for Failed {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(self.0.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        assert_eq!(
            super::write_output(&mut Failed(std::io::ErrorKind::BrokenPipe), "fixture", true),
            std::process::ExitCode::SUCCESS
        );
        assert_eq!(
            super::write_output(&mut Failed(std::io::ErrorKind::Other), "fixture", false),
            std::process::ExitCode::from(2)
        );
        assert_eq!(
            super::write_output(&mut Vec::new(), "fixture", true),
            std::process::ExitCode::from(3)
        );
    }

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

    fn pytest_fixture(parent: &std::path::Path) {
        let cache = parent.join("project/.pytest_cache");
        let v = cache.join("v/cache");
        fs::create_dir_all(&v).unwrap();
        fs::write(cache.join("README.md"), b"# pytest cache directory #\n\nThis directory contains data from the pytest's cache plugin,\nwhich provides the `--lf` and `--ff` options, as well as the `cache` fixture.\n\n**Do not** commit this to version control.\nSee [the docs](https://docs.pytest.org/en/stable/how-to/cache.html) for more information.\n").unwrap();
        fs::write(
            cache.join(".gitignore"),
            b"# Created by pytest automatically.\n*\n",
        )
        .unwrap();
        fs::write(cache.join("CACHEDIR.TAG"), b"Signature: 8a477f597d28d172789f06886806bc55\n# This file is a cache directory tag created by pytest.\n# For information about cache directory tags, see:\n#\thttps://bford.info/cachedir/spec.html\n").unwrap();
        fs::write(v.join("nodeids"), br#"["tests/test_one.py::test_one"]"#).unwrap();
        fs::write(
            v.join("lastfailed"),
            br#"{"tests/test_private.py::test_secret":true}"#,
        )
        .unwrap();
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
    fn pytest_review_selects_only_verified_nodeids_for_dry_run() {
        let temp = tempfile::tempdir().unwrap();
        pytest_fixture(temp.path());
        let (output, partial) = run(vec![
            "clean",
            "--root",
            temp.path().to_str().unwrap(),
            "--review",
            "python.pytest-nodeids",
            "--dry-run",
            "--json",
        ]
        .into_iter()
        .map(str::to_string)
        .collect())
        .unwrap();
        assert!(!partial);
        assert!(output.contains("\"eligible_items\": 1"));
        assert!(output.contains("new-first"));
        assert!(temp
            .path()
            .join("project/.pytest_cache/v/cache/nodeids")
            .exists());
        assert!(temp
            .path()
            .join("project/.pytest_cache/v/cache/lastfailed")
            .exists());
    }

    #[test]
    fn recreatable_caches_are_available_for_cli_review() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join(".mypy_cache")).unwrap();
        std::fs::write(temp.path().join(".mypy_cache/fixture"), b"cache").unwrap();
        let (text, partial) = run(vec![
            "clean".into(),
            "--root".into(),
            temp.path().to_str().unwrap().into(),
            "--review".into(),
            "python.mypy-cache".into(),
            "--dry-run".into(),
            "--json".into(),
        ])
        .unwrap();
        assert!(!partial);
        assert!(text.contains("\"review_required\": true"));
        assert!(temp.path().join(".mypy_cache/fixture").exists());
    }

    #[test]
    fn review_plans_every_candidate_of_the_detector_and_lists_each() {
        let temp = tempfile::tempdir().unwrap();
        cargo_fixture(temp.path(), "one");
        cargo_fixture(temp.path(), "two");
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
        assert!(output.contains("2 itens"), "{output}");
        assert!(output.contains("lib-one.json"), "{output}");
        assert!(output.contains("lib-two.json"), "{output}");
        assert!(output.contains("Nenhuma limpeza executada."));
    }

    #[test]
    fn readable_texts_for_errors_and_plurals() {
        use super::{count, os_error_text};
        assert_eq!(count(1, "item", "itens"), "1 item");
        assert_eq!(count(2, "item", "itens"), "2 itens");
        assert_eq!(
            os_error_text("Not a directory (os error 20)"),
            "um arquivo está onde se esperava uma pasta"
        );
        assert_eq!(
            os_error_text("Captura recusada: Not a directory (os error 20)"),
            "Captura recusada: um arquivo está onde se esperava uma pasta"
        );
        assert_eq!(
            os_error_text("Weird (os error 9999)"),
            "Weird (os error 9999)"
        );
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

    #[test]
    fn unknown_apt_size_does_not_make_apps_list_fail() {
        let inventory = inodune_apps::Inventory {
            read_only: true,
            distro: inodune_apps::Distro::default(),
            sources: vec![inodune_apps::SourceReport {
                source: "apt".into(),
                status: "ok".into(),
                warnings: vec!["Installed-Size inválido: vendor-badsize:amd64".into()],
                items: vec![inodune_apps::InstalledItem {
                    source: inodune_apps::Source::Apt,
                    id: "vendor-vpn-client:amd64".into(),
                    name: "vendor-vpn-client:amd64".into(),
                    version: Some("6.2.0".into()),
                    size_bytes: None,
                    install_date: None,
                    signals: vec![],
                    risk: "unknown".into(),
                    protected: false,
                    notes: vec![],
                }],
            }],
        };
        let (text, partial) = present_apps(&inventory, false).unwrap();
        assert!(!partial);
        assert!(text.contains("apt: ok (1 item)"));
        assert!(text.contains("desconhecido"));
        assert!(!text.contains("None"));
        assert!(text.contains("vendor-badsize:amd64"));
        let (json, partial) = present_apps(&inventory, true).unwrap();
        assert!(!partial);
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value["sources"][0]["items"][0]["size_bytes"].is_null());
        assert_eq!(value["sources"][0]["status"], "ok");
    }
}
