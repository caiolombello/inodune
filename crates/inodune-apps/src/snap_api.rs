//! Optional local, read-only snapd HTTP endpoint. CLI remains fallback.
use crate::{valid_id, InstalledItem, Source};
use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    time::Duration,
};
const MAX: usize = 8 * 1024 * 1024;
pub(crate) fn list() -> Result<Vec<InstalledItem>, String> {
    // Snapd is local. No URL, path, method or socket supplied by callers.
    let mut socket = UnixStream::connect("/run/snapd.socket").map_err(|e| e.to_string())?;
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    socket
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    socket
        .write_all(
            b"GET /v2/snaps?select=all HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .map_err(|e| e.to_string())?;
    let started = std::time::Instant::now();
    let mut bytes = Vec::new();
    loop {
        if started.elapsed() > Duration::from_secs(20) {
            return Err("snapd timeout".into());
        }
        let mut buf = [0; 16384];
        let n = socket.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&buf[..n]);
        if bytes.len() > MAX {
            return Err("snapd excede limite".into());
        }
    }
    let split = bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("HTTP snapd inválido")?;
    let header = std::str::from_utf8(&bytes[..split]).map_err(|_| "HTTP inválido")?;
    if !header.starts_with("HTTP/1.1 200 ") && !header.starts_with("HTTP/1.0 200 ") {
        return Err("snapd HTTP erro".into());
    }
    let body = &bytes[split + 4..];
    if header
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        let mut remaining = body;
        let mut decoded = Vec::new();
        loop {
            let end = remaining
                .windows(2)
                .position(|w| w == b"\r\n")
                .ok_or("Chunk inválido")?;
            let size = usize::from_str_radix(
                std::str::from_utf8(&remaining[..end])
                    .map_err(|_| "Chunk inválido")?
                    .split(';')
                    .next()
                    .ok_or("Chunk inválido")?,
                16,
            )
            .map_err(|_| "Chunk inválido")?;
            remaining = &remaining[end + 2..];
            if size == 0 {
                break;
            }
            if size > remaining.len().saturating_sub(2) || decoded.len().saturating_add(size) > MAX
            {
                return Err("Chunk excede limite".into());
            }
            decoded.extend_from_slice(&remaining[..size]);
            if &remaining[size..size + 2] != b"\r\n" {
                return Err("Chunk inválido".into());
            }
            remaining = &remaining[size + 2..];
        }
        parse(&decoded)
    } else {
        parse(body)
    }
}
pub(crate) fn parse(bytes: &[u8]) -> Result<Vec<InstalledItem>, String> {
    if bytes.len() > MAX {
        return Err("snapd excede limite".into());
    }
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "JSON snapd inválido")?;
    if value["type"] != "sync" || value["status-code"] != 200 {
        return Err("Resposta snapd inválida".into());
    }
    let entries = value["result"]
        .as_array()
        .ok_or("Resultado snapd inválido")?;
    entries
        .iter()
        .map(|entry| {
            let name = entry["name"]
                .as_str()
                .filter(|s| valid_id(s))
                .ok_or("Nome Snap inválido")?;
            let rev = entry["revision"]
                .as_str()
                .filter(|s| valid_id(s))
                .ok_or("Revisão Snap inválida")?;
            let mut item = InstalledItem::new(Source::Snap, &format!("{name}@{rev}"), name);
            item.version = entry["version"].as_str().map(str::to_owned);
            item.size_bytes = entry["installed-size"].as_u64();
            if let Some(date) = entry["install-date"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .and_then(|d| u64::try_from(d.timestamp()).ok())
            {
                item.install_date = Some(date);
            }
            if matches!(entry["type"].as_str(), Some("base" | "os" | "snapd") | None)
                || name == "snapd"
                || name == "core"
                || name
                    .strip_prefix("core")
                    .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
            {
                item.protect();
            }
            if entry["status"] == "installed" {
                item.notes.push("Revisão desabilitada".into());
            }
            Ok(item)
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn api_types_disabled_sizes_and_hostile_names() {
        let bytes=br#"{"type":"sync","status-code":200,"result":[{"name":"foundation","revision":"1","type":"base","status":"installed","installed-size":1024},{"name":"editor","revision":"2","type":"app","status":"active"}]}"#;
        let items = parse(bytes).unwrap();
        assert!(items[0].protected);
        assert_eq!(items[0].size_bytes, Some(1024));
        assert!(items[0].notes[0].contains("desabilitada"));
        assert!(!items[1].protected);
        for name in ["-bad", "x y", "x;y", "$(id)", "../escape"] {
            let bytes=serde_json::to_vec(&serde_json::json!({"type":"sync","status-code":200,"result":[{"name":name,"revision":"1"}]})).unwrap();
            assert!(parse(&bytes).is_err());
        }
        assert!(parse(b"{}").is_err());
        assert!(parse(b"garbage").is_err());
    }
}
