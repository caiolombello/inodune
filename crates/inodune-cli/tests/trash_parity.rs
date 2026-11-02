//! 0.1.11 QA: the Trash classifier marked library files (.d.ts, dist .js) as
//! credentials that the node_modules classifier had cleaned. Both must give
//! the same verdict. Fake secrets only; HOME/XDG point to a tempdir.
use std::{fs, path::Path, process::Command};

const OPENSSH: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAAFAKEQAFIXTURE\n-----END OPENSSH PRIVATE KEY-----\n";

/// (path inside node_modules, content)
fn cases() -> Vec<(&'static str, String)> {
    vec![
        // QA false positives
        (
            "@aws-sdk/credential-provider-ini/dist-types/resolveStaticCredentials.d.ts",
            "export interface StaticCredsProfile {\n  aws_secret_access_key: string;\n}\n".into(),
        ),
        (
            "@smithy/shared-ini-file-loader/dist-types/types.d.ts",
            "  aws_secret_access_key?: string;\n  aws_session_token?: string;\n".into(),
        ),
        (
            "@smithy/credential-provider-imds/dist-cjs/index.js",
            "module.exports = {\n  accessToken: \"eyJhbGciOiJIUzI1NiJ9fakeABCDEFGH0123\",\n};\n".into(),
        ),
        (
            "typescript/lib/lib.dom.d.ts",
            "interface PasswordCredential {\n    password: string;\n}\n".into(),
        ),
        (
            "typescript/dist/ast/scanner.js",
            "const kinds = {\n  token: \"SyntaxKindIdentifierTokenValue0123\",\n};\n".into(),
        ),
        (
            "@smithy/core/dist-types/setTokenFeature.d.ts",
            "export declare function setTokenFeature(token: string): void;\n".into(),
        ),
        // 0.1.10 baits
        ("pip-blob/blob7f3a", OPENSSH.into()),
        (
            "rsa/blobrsa",
            "-----BEGIN RSA PRIVATE KEY-----\nMIIFAKEQA\n-----END RSA PRIVATE KEY-----".into(),
        ),
        (
            "pkcs8/cd",
            "-----BEGIN PRIVATE KEY-----\nMIIFAKEQA\n-----END PRIVATE KEY-----".into(),
        ),
        (
            "aws-real/notes.txt",
            "aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
        ),
        ("aws-fake/notes.txt", "aws_secret_access_key = FAKE".into()),
        (
            "npm/01",
            "//registry.npmjs.org/:_authToken=npm_FAKEFAKEFAKEFAKEFAKEFAKEFAKEFAKE1234".into(),
        ),
        (
            "netrc/blobnetrc",
            "machine github.com login qa password ghp_FAKEFAKEFAKEFAKEFAKEFAKEFAKEFAKE1234".into(),
        ),
        (
            "left-pad/README.md",
            "# left-pad\n\n-----BEGIN RSA PRIVATE KEY-----\nMIIFAKEQA\n-----END RSA PRIVATE KEY-----\n".into(),
        ),
        ("dotenv-case/.env", "A=1\n".into()),
        ("ssh-case/id_ed25519", OPENSSH.into()),
    ]
}

fn scan(root: &Path) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_inodune-cli"))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", root)
        .env("XDG_CACHE_HOME", root.join(".cache"))
        .env("XDG_DATA_HOME", root.join(".local/share"))
        .env("XDG_STATE_HOME", root.join(".local/state"))
        .args(["scan", "--json", "--root"])
        .arg(root)
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}

fn protected_paths(scan: &serde_json::Value) -> Vec<String> {
    scan["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["risk"] == "protected" && c["detector_id"] == "protected.state")
        .map(|c| c["display_path"].as_str().unwrap().to_owned())
        .collect()
}

/// Verdict of the cache classifier: the file or its preserved directory.
fn cache_verdicts() -> Vec<bool> {
    cases()
        .iter()
        .enumerate()
        .map(|(i, (path, content))| {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path();
            fs::create_dir_all(root.join("p")).unwrap();
            fs::write(root.join("p/package.json"), "{}").unwrap();
            let modules = root.join("p/node_modules");
            fs::create_dir_all(modules.join("zz-safe")).unwrap();
            fs::write(modules.join("zz-safe/ordinary"), "rebuildable").unwrap();
            let file = modules.join(format!("case{i:02}")).join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, content).unwrap();
            let file = file.to_string_lossy().into_owned();
            protected_paths(&scan(root))
                .iter()
                .any(|p| file == *p || file.starts_with(&format!("{p}/")))
        })
        .collect()
}

fn encode(path: &str) -> String {
    path.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// Verdict of the Trash classifier for the same content, trashed as a file
/// entry with a random name. `info`: Some(original path) writes a .trashinfo.
fn trash_verdict(content: &str, info: Option<&str>) -> bool {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let trash = root.join(".local/share/Trash");
    fs::create_dir_all(trash.join("files")).unwrap();
    fs::create_dir_all(trash.join("info")).unwrap();
    let name = "inodune-0123456789abcdef0123456789abcdef";
    fs::write(trash.join("files").join(name), content).unwrap();
    if let Some(original) = info {
        fs::write(
            trash.join("info").join(format!("{name}.trashinfo")),
            format!(
                "[Trash Info]\nPath={}\nDeletionDate=2026-11-02T06:00:00\n",
                encode(original)
            ),
        )
        .unwrap();
    }
    let path = trash
        .join("files")
        .join(name)
        .to_string_lossy()
        .into_owned();
    protected_paths(&scan(root)).contains(&path)
}

#[test]
fn trash_and_node_modules_give_the_same_verdict() {
    let cache = cache_verdicts();
    let expected_protected = [
        false, false, false, false, false, false, true, true, true, true, true, true, true, true,
        true, true,
    ];
    for (i, (path, content)) in cases().iter().enumerate() {
        assert_eq!(cache[i], expected_protected[i], "cache verdict for {path}");
        let original = format!("/home/user/project/node_modules/case{i:02}/{path}");
        let trash = trash_verdict(content, Some(&original));
        assert_eq!(trash, cache[i], "Trash verdict differs for {path}");
    }
}

#[test]
fn missing_or_lying_trashinfo_never_hides_a_private_key() {
    let key = "-----BEGIN RSA PRIVATE KEY-----\nMIIFAKEQA\n-----END RSA PRIVATE KEY-----";
    assert!(trash_verdict(key, None), "no .trashinfo");
    assert!(
        trash_verdict(
            key,
            Some("/home/user/project/node_modules/@types/x/index.d.ts")
        ),
        "lying .d.ts path"
    );
    assert!(
        trash_verdict(
            key,
            Some("/home/user/project/node_modules/left-pad/README.md")
        ),
        "lying documentation path"
    );
    // Real values keep protecting under a TypeScript or documentation hint.
    assert!(trash_verdict(
        "aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
        Some("/home/user/project/node_modules/x/config.d.ts")
    ));
    assert!(trash_verdict(
        "//registry.npmjs.org/:_authToken=npm_FAKEFAKEFAKEFAKEFAKEFAKEFAKEFAKE1234",
        Some("/home/user/project/node_modules/x/index.js")
    ));
    // Name rules apply to the original name too (union with the Trash name).
    assert!(trash_verdict("A=1\n", Some("/home/user/project/.env")));
    assert!(trash_verdict("plain", Some("/home/user/.ssh/id_ed25519")));
    // Without a usable hint the stricter evaluation by the Trash name stays.
    let token = "token: \"SyntaxKindIdentifierTokenValue0123\",\n";
    assert!(trash_verdict(token, None));
}
