//! Plano determinístico de um único snap, sem purge ou remoção de dependências.
use crate::{
    snap_api,
    uninstall::{validate_id, Removal, Request},
    Source,
};
use serde_json::Value;
use std::time::{Duration, Instant};
fn identity(id: &str) -> Result<(&str, Option<&str>), String> {
    let (name, rev) = id.split_once('@').map_or((id, None), |(n, r)| (n, Some(r)));
    if name.is_empty()
        || !name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"-_".contains(&c))
        || !name.as_bytes()[0].is_ascii_alphanumeric()
        || rev.is_some_and(|r| r.is_empty() || !r.bytes().all(|c| c.is_ascii_digit()))
    {
        return Err("ID ou revisão Snap inválido".into());
    }
    Ok((name, rev))
}
fn result(bytes: &[u8]) -> Result<Value, String> {
    let v: Value = serde_json::from_slice(bytes).map_err(|_| "JSON snapd inválido")?;
    if v["type"] != "sync" || v["status-code"] != 200 {
        return Err("snapd não confirmou consulta completa".into());
    }
    Ok(v["result"].clone())
}
pub(super) fn simulate(request: &Request) -> Result<Vec<Removal>, String> {
    let snaps = result(&snap_api::request(
        "GET",
        "/v2/snaps?select=all",
        None,
        false,
    )?)?;
    let connections = result(&snap_api::request(
        "GET",
        "/v2/connections?select=all",
        None,
        false,
    )?)?;
    assess(&request.id, &snaps, &connections)
}
fn assess(id: &str, snaps: &Value, connections: &Value) -> Result<Vec<Removal>, String> {
    let (name, revision) = identity(id)?;
    let snaps = snaps.as_array().ok_or("Lista snapd incompleta")?;
    let matching: Vec<_> = snaps
        .iter()
        .filter(|s| s["name"] == name && s["status"] == "active")
        .collect();
    if matching.len() != 1 {
        return Err("Snap não ativo ou identidade ambígua".into());
    }
    let target = matching[0];
    if target["type"] != "app" {
        return Err("Tipo Snap protegido: somente app pode ser removido".into());
    }
    let rev = target["revision"]
        .as_str()
        .filter(|r| !r.is_empty() && r.bytes().all(|c| c.is_ascii_digit()))
        .ok_or("Revisão Snap desconhecida")?;
    if revision.is_some_and(|expected| expected != rev) {
        return Err("Revisão Snap mudou desde o inventário".into());
    }
    for snap in snaps {
        if snap["name"] == name {
            continue;
        }
        if snap["type"] == "app" {
            let base=snap["base"].as_str().ok_or("snapd não informou a base de todos os apps; dependentes desconhecidos, remoção bloqueada")?;
            if base == name {
                return Err("Snap é base de outro aplicativo; protegido".into());
            }
        } else if snap["type"].as_str().is_none() {
            return Err("Tipo de snap dependente desconhecido".into());
        }
    }
    let established = connections["established"]
        .as_array()
        .ok_or("Conexões Snap incompletas; content providers desconhecidos")?;
    for connection in established {
        let interface = connection["interface"]
            .as_str()
            .ok_or("Interface Snap desconhecida")?;
        let slot = connection["slot"]["snap"]
            .as_str()
            .ok_or("Slot Snap desconhecido")?;
        let plug = connection["plug"]["snap"]
            .as_str()
            .ok_or("Plug Snap desconhecido")?;
        if interface == "content" && slot == name && plug != name {
            return Err("Snap fornece conteúdo para outro app; protegido".into());
        }
    }
    Ok(vec![Removal {
        id: name.into(),
        version: rev.into(),
    }])
}
pub(super) fn execute(request: &Request) -> Result<(), String> {
    validate_id(Source::Snap, &request.id)?;
    let (name, _) = identity(&request.id)?;
    let path = format!("/v2/snaps/{name}");
    let response = snap_api::request("POST", &path, Some(r#"{"action":"remove"}"#), true)?;
    let response: Value =
        serde_json::from_slice(&response).map_err(|_| "snapd: resultado desconhecido")?;
    if response["type"] != "async" || response["status-code"] != 202 {
        return Err("snapd não iniciou a remoção; consulte o inventário".into());
    }
    let change = response["change"]
        .as_str()
        .filter(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()))
        .ok_or("ID de transação snapd inválido")?;
    let start = Instant::now();
    loop {
        let status = result(&snap_api::request(
            "GET",
            &format!("/v2/changes/{change}"),
            None,
            false,
        )?)?;
        if status["ready"] == true {
            return if status["status"] == "Done" {
                Ok(())
            } else {
                Err("snapd não confirmou remoção completa".into())
            };
        }
        if start.elapsed() > Duration::from_secs(90) {
            return Err("Timeout snapd; resultado desconhecido, sem retry".into());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn snaps() -> Value {
        json!([{"name":"vendor-tool","revision":"42","type":"app","status":"active","base":"core24"},{"name":"other-app","revision":"2","type":"app","status":"active","base":"core24"}])
    }
    #[test]
    fn plans_exact_revision_and_blocks_base_content_unknown_and_changed_snap() {
        let mut snaps = snaps();
        let connections = json!({"established":[]});
        assert_eq!(
            assess("vendor-tool@42", &snaps, &connections).unwrap()[0].version,
            "42"
        );
        assert!(assess("vendor-tool@41", &snaps, &connections).is_err());
        snaps[1]["base"] = json!("vendor-tool");
        assert!(assess("vendor-tool", &snaps, &connections).is_err());
        snaps[1]["base"] = Value::Null;
        assert!(assess("vendor-tool", &snaps, &connections).is_err());
        snaps[1]["base"] = json!("core24");
        let content = json!({"established":[{"interface":"content","slot":{"snap":"vendor-tool"},"plug":{"snap":"other-app"}}]});
        assert!(assess("vendor-tool", &snaps, &content).is_err());
        for kind in ["base", "os", "core", "snapd", "kernel", "gadget"] {
            snaps[0]["type"] = json!(kind);
            assert!(assess("vendor-tool", &snaps, &connections).is_err());
        }
    }
}
