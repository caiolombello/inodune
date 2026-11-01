//! The Tauri handler, build manifest and the permissions granted to `main` stay aligned.
use std::{fs, path::PathBuf};

fn source(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn quoted_commands(list_body: &str) -> Vec<String> {
    list_body
        .split('"')
        .enumerate()
        .filter(|(index, _)| index % 2 == 1)
        .map(|(_, part)| part.to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

fn handler_commands() -> Vec<String> {
    let src = source("src/main.rs");
    let start = src
        .find("generate_handler!")
        .expect("generate_handler! missing");
    let body = &src[start..];
    let open = body.find('[').expect("handler list missing");
    let close = body[open..].find(']').expect("handler list unclosed");
    body[open + 1..open + close]
        .split(',')
        .filter_map(|part| {
            let part = part.split("//").next().unwrap_or("").trim();
            if part.is_empty() {
                None
            } else {
                Some(part.rsplit("::").next().unwrap_or(part).trim().to_string())
            }
        })
        .collect()
}

fn manifest_commands() -> Vec<String> {
    let src = source("build.rs");
    let marker = "commands(&[";
    let start = src.find(marker).expect("AppManifest commands missing");
    let body = &src[start + marker.len() - 1..];
    let close = body.find(']').expect("manifest command list unclosed");
    quoted_commands(&body[..=close])
}

fn granted_permissions() -> Vec<String> {
    let src = source("capabilities/main.json");
    assert!(src.contains("\"identifier\": \"main\""));
    let marker = "\"permissions\"";
    let start = src.find(marker).expect("capability permissions missing");
    let body = &src[start..];
    let open = body.find('[').unwrap();
    let close = body[open..].find(']').unwrap();
    quoted_commands(&body[open..=open + close])
}

fn allowed_commands(granted: &[String]) -> Vec<String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("permissions");
    let mut allowed = Vec::new();
    for entry in fs::read_dir(&root).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            for nested in fs::read_dir(&path).unwrap() {
                let nested = nested.unwrap().path();
                if nested.extension().is_some_and(|ext| ext == "toml") {
                    allowed.extend(allows_in(&fs::read_to_string(&nested).unwrap(), granted));
                }
            }
        } else if path.extension().is_some_and(|ext| ext == "toml") {
            allowed.extend(allows_in(&fs::read_to_string(&path).unwrap(), granted));
        }
    }
    allowed.sort();
    allowed.dedup();
    allowed
}

fn allows_in(toml: &str, granted: &[String]) -> Vec<String> {
    toml.split("[[permission]]")
        .skip(1)
        .filter_map(|block| {
            let identifier = string_after(block, "identifier")?;
            if !granted.iter().any(|name| name == &identifier) {
                return None;
            }
            let allow = block.split("commands.allow").nth(1)?;
            let open = allow.find('[')?;
            let close = allow[open..].find(']')?;
            Some(quoted_commands(&allow[open..=open + close]))
        })
        .flatten()
        .collect()
}

fn string_after(block: &str, key: &str) -> Option<String> {
    let rest = block.split(key).nth(1)?;
    let open = rest.find('"')?;
    let close = rest[open + 1..].find('"')?;
    Some(rest[open + 1..open + 1 + close].to_string())
}

#[test]
fn every_handler_command_is_built_and_granted_and_nothing_extra_is_allowed() {
    let mut handler = handler_commands();
    let mut manifest = manifest_commands();
    handler.sort();
    manifest.sort();
    assert!(
        handler.contains(&"list_installed_apps".to_string()),
        "handler commands: {handler:?}"
    );
    assert_eq!(
        handler, manifest,
        "build.rs commands must match the invoke handler"
    );
    let granted = granted_permissions();
    assert!(
        granted
            .iter()
            .any(|name| name == "allow-installed-apps-readonly"),
        "capability grants: {granted:?}"
    );
    let mut allowed = allowed_commands(&granted);
    allowed.sort();
    assert_eq!(
        handler, allowed,
        "commands allowed by permissions granted to capability main"
    );
}
