use std::process::ExitCode;
fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}
fn run(args: Vec<String>) -> Result<String, String> {
    let inv = inodune_core::demo_inventory().map_err(|e| e.to_string())?;
    let values: Vec<&str> = args.iter().map(String::as_str).collect();
    match values.as_slice() {
        ["scan", "--demo", "--json"] => serde_json::to_string_pretty(&inv).map_err(|e| e.to_string()),
        ["clean", "--demo", "--dry-run"] => {
            let ids = inv.candidates.iter().filter(|c| c.selected_by_default && c.risk == inodune_core::Risk::Low).map(|c| c.id.clone()).collect::<Vec<_>>();
            let plan = inodune_clean::dry_run(&inv.candidates, &ids).map_err(str::to_string)?;
            serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())
        }
        _ => Err("Phase 0: synthetic data only. Usage: inodune-cli scan --demo --json | clean --demo --dry-run. No roots or --apply supported.".into()),
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn refuses_apply_and_real_roots() {
        for args in [
            vec!["clean", "--apply"],
            vec!["scan", "--json"],
            vec!["scan", "/home"],
        ] {
            assert!(super::run(args.into_iter().map(str::to_string).collect()).is_err());
        }
    }
}
