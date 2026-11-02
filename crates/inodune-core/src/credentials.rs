//! Single content policy for credentials inside cleanup targets.
//!
//! Used by the scanner (manifest exclusions) and by the executor right before a
//! file is moved or removed, for every detector and both actions. Format checks
//! only: nothing is decrypted, logged or persisted. When in doubt, protect.
use std::path::Path;

/// First window read from every regular file.
pub const FIRST_WINDOW: usize = 64 * 1024;
/// Text files (no NUL byte in the first window) are read up to this size.
/// Content beyond it, or beyond the first window of a binary file, is not
/// inspected; name rules and unreadable/unstable files still fail closed.
pub const TEXT_LIMIT: usize = 1024 * 1024;

/// `-----BEGIN <anything> PRIVATE KEY...-----` at the start of a line, after
/// optional whitespace. The body is never required to be valid.
pub fn private_key_header(line: &str) -> bool {
    let Some(rest) = line.trim_start().strip_prefix("-----BEGIN ") else {
        return false;
    };
    rest.split_once("-----")
        .is_some_and(|(kind, _)| kind.contains("PRIVATE KEY"))
}

/// The whole value is an obvious marker, not a value that merely contains one.
pub fn placeholder(value: &str) -> bool {
    let value = clean_value(value);
    if value.is_empty() {
        return true;
    }
    let lower = value.to_ascii_lowercase();
    if lower.starts_with("your_") || lower.starts_with("your-") {
        return true;
    }
    if (value.starts_with('<') && value.ends_with('>'))
        || (value.starts_with("${") && value.ends_with('}'))
        || (value.starts_with("{{") && value.ends_with("}}"))
    {
        return true;
    }
    if let Some(name) = value.strip_prefix('$') {
        if !name.is_empty() && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
            return true;
        }
    }
    if value.chars().all(|c| matches!(c, 'x' | 'X' | '*' | '.')) {
        return true;
    }
    const MARKERS: [&str; 6] = [
        "changeme",
        "example",
        "placeholder",
        "dummy",
        "redacted",
        "your",
    ];
    // A pure word identifier built around a marker (EXAMPLE_AWS_SECRET_ACCESS_KEY,
    // your-token-here). Mixed digits or other symbols look like a key: protect.
    value
        .bytes()
        .all(|c| c.is_ascii_alphabetic() || c == b'_' || c == b'-')
        && lower.split(['_', '-']).any(|word| MARKERS.contains(&word))
}

fn clean_value(value: &str) -> &str {
    let value = value.split_once(" #").map_or(value, |(v, _)| v);
    value
        .trim()
        .trim_end_matches([',', ';'])
        .trim()
        .trim_matches(['\'', '"', '`'])
        .trim()
}

/// Code references (`os.environ[...]`, `None`, `options.secret()`), not values.
fn code_expression(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    value.contains(['(', ')', '[', ']', '{', '}', ' '])
        || ["none", "null", "nil", "undefined", "true", "false"].contains(&lower.as_str())
}

/// Keys that protect with any non-placeholder value, in every file type.
fn strict_key(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "_authtoken" | "aws_secret_access_key"
    )
}

/// `key = value`, `key: value`, `export key=value` or npmrc `//host/:key=value`
/// anchored at the line start. `conf["_authToken"]` is not an assignment.
fn assignment(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.starts_with('#') || line.starts_with(';') {
        return None;
    }
    let (name, value) = line.split_once('=').or_else(|| line.split_once(':'))?;
    let name = name.trim();
    let name = name.strip_prefix("export ").unwrap_or(name).trim();
    let name = name.trim_matches(['\'', '"']);
    let key = match name.rsplit_once(':') {
        Some((scope, key)) if scope.starts_with("//") => key,
        Some(_) => return None,
        None => name,
    };
    if key.is_empty() || !key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
        return None;
    }
    Some((key, value))
}

/// netrc: a `machine`/`default` entry with a `password` that has a real value.
fn netrc_password(text: &str) -> bool {
    const KEYWORDS: [&str; 6] = [
        "machine", "default", "login", "password", "account", "macdef",
    ];
    let mut in_entry = false;
    for line in text.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let Some(first) = tokens.first() else {
            continue;
        };
        if !KEYWORDS.contains(first) {
            in_entry = false;
            continue;
        }
        if matches!(*first, "machine" | "default") {
            in_entry = true;
        }
        if !in_entry {
            continue;
        }
        for pair in tokens.windows(2) {
            if pair[0] == "password" && !matches!(pair[1], "=" | ":") && !placeholder(pair[1]) {
                return true;
            }
        }
    }
    false
}

fn lower_components(path: &Path) -> Vec<String> {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect()
}

/// Package metadata and package documentation that quote credential formats.
/// Opaque cache blobs and files without a documentation extension are not exempt.
pub fn documentation_exempt(path: &Path) -> bool {
    let parts = lower_components(path);
    let Some(name) = parts.last() else {
        return false;
    };
    let parent = parts.len().checked_sub(2).map(|i| parts[i].as_str());
    if ["metadata", "pkg-info", "record"].contains(&name.as_str())
        && parent.is_some_and(|p| p.ends_with(".dist-info") || p.ends_with(".egg-info"))
    {
        return true;
    }
    let in_package = parts.iter().any(|p| {
        matches!(
            p.as_str(),
            "node_modules" | "site-packages" | "dist-packages"
        )
    }) || parts
        .windows(2)
        .any(|w| w[0] == ".cargo" && w[1] == "registry");
    if !in_package {
        return false;
    }
    let doc_ext = [".md", ".markdown", ".rst", ".txt", ".adoc"]
        .iter()
        .any(|ext| name.ends_with(ext));
    let top_level_doc = ["readme", "changelog", "changes", "history"]
        .iter()
        .any(|prefix| name.starts_with(prefix));
    let in_docs = parts[..parts.len() - 1]
        .iter()
        .any(|p| matches!(p.as_str(), "doc" | "docs" | "documentation"));
    top_level_doc && (doc_ext || !name.contains('.')) || (in_docs && doc_ext)
}

const CODE_EXTENSIONS: [&str; 14] = [
    "rs", "js", "jsx", "ts", "tsx", "py", "go", "java", "c", "h", "cpp", "css", "html", "map",
];

/// Credential signatures in already-read bytes. See the module docs.
pub fn credential_bytes(path: &Path, bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    // A private-key header protects everywhere, documentation included.
    if text.lines().any(private_key_header) {
        return true;
    }
    // The documentation exemption only covers the value-based rules below.
    if documentation_exempt(path) {
        return false;
    }
    if text.lines().any(|line| {
        assignment(line).is_some_and(|(key, value)| {
            let value = clean_value(value);
            strict_key(key) && !placeholder(value) && !code_expression(value)
        })
    }) {
        return true;
    }
    if netrc_password(&text) {
        return true;
    }
    let code = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| CODE_EXTENSIONS.contains(&e));
    if code {
        return false;
    }
    if serde_json::from_slice(bytes)
        .ok()
        .as_ref()
        .is_some_and(json_credentials)
    {
        return true;
    }
    text.lines().any(|line| {
        assignment(line).is_some_and(|(key, value)| secret_key(key) && plausible_secret(value))
    })
}

fn plausible_secret(value: &str) -> bool {
    let value = clean_value(value);
    value.len() >= 20
        && value.len() <= 2048
        && !placeholder(value)
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_+/=-.".contains(&c))
        && value
            .bytes()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            >= 8
}

fn secret_key(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "_authtoken"
            | "_auth"
            | "token"
            | "auth_token"
            | "api_key"
            | "apikey"
            | "aws_secret_access_key"
            | "aws_session_token"
            | "access_token"
            | "refresh_token"
            | "client_secret"
            | "accountkey"
            | "secret_access_key"
            | "secretaccesskey"
            | "clientsecret"
            | "accesstoken"
            | "refreshtoken"
            | "password"
    )
}

fn json_credentials(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => map.iter().any(|(key, value)| {
            (secret_key(key) && value.as_str().is_some_and(plausible_secret))
                || (strict_key(key)
                    && value
                        .as_str()
                        .is_some_and(|v| !placeholder(v) && !code_expression(clean_value(v))))
                || (key == "private_key"
                    && value
                        .as_str()
                        .is_some_and(|s| s.lines().any(private_key_header)))
                || json_credentials(value)
        }),
        serde_json::Value::Array(items) => items.iter().any(json_credentials),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_at_line_start_is_enough_and_literals_are_not() {
        for kind in ["", "RSA ", "EC ", "DSA ", "OPENSSH ", "ENCRYPTED ", "PGP "] {
            let line = format!("  -----BEGIN {kind}PRIVATE KEY-----");
            assert!(private_key_header(&line), "{line}");
        }
        assert!(private_key_header(
            "-----BEGIN PGP PRIVATE KEY BLOCK-----\r"
        ));
        assert!(!private_key_header("-----BEGIN PUBLIC KEY-----"));
        assert!(!private_key_header("-----BEGIN CERTIFICATE-----"));
        assert!(!private_key_header(
            r#"const K = "-----BEGIN PRIVATE KEY-----\nMII";"#
        ));
    }

    #[test]
    fn placeholders_are_whole_markers_only() {
        for value in [
            "",
            "  ",
            "YOUR_SECRET",
            "your-token-here",
            "<token>",
            "${AWS_SECRET}",
            "$AWS_SECRET",
            "xxxxxxxx",
            "****",
            "...",
            "changeme",
            "EXAMPLE",
            "Placeholder",
            "dummy",
            "REDACTED",
            "EXAMPLE_AWS_SECRET_ACCESS_KEY",
            "\"YOUR_SECRET\"",
        ] {
            assert!(placeholder(value), "{value:?}");
        }
        for value in [
            "FAKE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            "npm_FAKEFAKEFAKEFAKEFAKEFAKEFAKEFAKE1234",
            "ghp_FAKEFAKEFAKEFAKEFAKEFAKEFAKEFAKE1234",
            "TEST_FIXTURE_123",
            "s3cr3t",
        ] {
            assert!(!placeholder(value), "{value:?}");
        }
    }

    #[test]
    fn identifiers_and_code_references_are_not_assignments_of_secrets() {
        let path = Path::new("auth.js");
        assert!(!credential_bytes(
            path,
            b"const v = conf[\"_authToken\"];\n"
        ));
        assert!(!credential_bytes(
            path,
            b"const option = \"_authToken\"; export default option;"
        ));
        assert!(!credential_bytes(
            Path::new("settings.py"),
            b"aws_secret_access_key = os.environ[\"AWS_SECRET_ACCESS_KEY\"]\naws_secret_access_key = None\n"
        ));
        assert!(credential_bytes(Path::new("blob"), b"_authToken=abc"));
        assert!(credential_bytes(
            Path::new("blob"),
            b"aws_secret_access_key: FAKE"
        ));
    }

    #[test]
    fn netrc_needs_an_entry_and_a_password_value() {
        let path = Path::new("blob");
        assert!(credential_bytes(path, b"machine h login u password p4ss"));
        assert!(credential_bytes(
            path,
            b"machine h\n  login u\n  password p4ss\n"
        ));
        assert!(!credential_bytes(
            path,
            b"machine h login u password YOUR_PASSWORD"
        ));
        assert!(!credential_bytes(path, b"password = secret_from_prompt()"));
        assert!(!credential_bytes(path, b"Enter the machine password here"));
    }

    #[test]
    fn private_key_protects_even_in_exempt_documentation() {
        let key =
            b"Docs\n-----BEGIN RSA PRIVATE KEY-----\nMIIFAKEQA\n-----END RSA PRIVATE KEY-----\n";
        for path in [
            "node_modules/left-pad/README.md",
            "node_modules/pkg/CHANGELOG",
            "node_modules/netrc/docs/doc.txt",
            "site-packages/pkg-1.0.dist-info/METADATA",
            ".cargo/registry/src/x/crate/docs/keys.md",
        ] {
            assert!(documentation_exempt(Path::new(path)), "{path}");
            assert!(credential_bytes(Path::new(path), key), "{path}");
        }
        // Value-based rules stay exempt in documentation.
        assert!(!credential_bytes(
            Path::new("node_modules/aws-sdk/README.md"),
            b"aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
        ));
    }

    #[test]
    fn documentation_exemptions_are_narrow() {
        for path in [
            "site-packages/boto3-1.0.dist-info/METADATA",
            "x/pkg.egg-info/PKG-INFO",
            "node_modules/netrc/docs/doc.txt",
            "node_modules/aws-sdk/README.md",
            "node_modules/pkg/CHANGELOG",
            ".cargo/registry/src/x/crate/docs/keys.md",
        ] {
            assert!(documentation_exempt(Path::new(path)), "{path}");
        }
        for path in [
            ".cache/pip/http/blob7f3a",
            "node_modules/lodash/blob.js",
            "node_modules/pkg/notes.txt",
            "target/debug/notes.txt",
            "docs/README.md",
            "node_modules/pkg/docs/key",
        ] {
            assert!(!documentation_exempt(Path::new(path)), "{path}");
        }
    }
}
