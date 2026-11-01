//! Processo não privilegiado, uma preparação e no máximo uma execução.
use inodune_apps::uninstall::{self, Request, Session, SystemTransport};
use std::{io::Write, time::Duration};
fn main() {
    let result = run();
    if let Err(error) = result {
        let _ = writeln!(std::io::stdout(), "{}", serde_json::json!({"error":error}));
        std::process::exit(2);
    }
}
fn run() -> Result<(), String> {
    uninstall::refuse_root()?;
    let mut input = std::io::stdin().lock();
    let request: Request = serde_json::from_str(&uninstall::read_message(
        &mut input,
        Duration::from_secs(10),
    )?)
    .map_err(|_| "Pedido inválido")?;
    let mut transport = SystemTransport;
    let session = Session::prepare(request, &mut transport)?;
    let view = session.view();
    writeln!(
        std::io::stdout(),
        "{}",
        serde_json::to_string(&view).map_err(|e| e.to_string())?
    )
    .map_err(|e| e.to_string())?;
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let plan_id: String = serde_json::from_str(&uninstall::read_message(
        &mut input,
        Duration::from_secs(120),
    )?)
    .map_err(|_| "ID do plano inválido")?;
    session.execute(&plan_id, &mut transport)?;
    writeln!(
        std::io::stdout(),
        "{}",
        serde_json::json!({"status":"completed"})
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
