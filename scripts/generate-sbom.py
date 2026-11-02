#!/usr/bin/env python3
"""Create unsigned SPDX 2.3 inventories from the locked Rust and npm graphs."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import quote

ROOT = Path(__file__).resolve().parents[1]
VERSION = json.loads((ROOT / "apps/desktop/src-tauri/tauri.conf.json").read_text())["version"]
OUT = ROOT / "target" / "packages" / VERSION


def run_json(args: list[str], *, cwd: Path, env: dict[str, str] | None = None) -> dict:
    result = subprocess.run(args, cwd=cwd, env=env, check=True, text=True, capture_output=True)
    return json.loads(result.stdout)


def spdx_id(prefix: str, value: str) -> str:
    digest = hashlib.sha256(value.encode("utf-8")).hexdigest()[:20]
    return f"SPDXRef-{prefix}-{digest}"


def create_document(name: str, packages: list[dict], relationships: list[dict]) -> dict:
    canonical = json.dumps({"packages": packages, "relationships": relationships}, sort_keys=True)
    namespace = hashlib.sha256(canonical.encode("utf-8")).hexdigest()
    return {
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": name,
        "documentNamespace": f"https://spdx.org/spdxdocs/{name}-{namespace}",
        "creationInfo": {
            "created": datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
            "creators": ["Tool: inodune-generate-sbom.py"],
        },
        "documentDescribes": [package["SPDXID"] for package in packages if package.get("primaryPackagePurpose") == "APPLICATION"],
        "packages": packages,
        "relationships": relationships,
    }


def generate_frontend() -> Path:
    result = run_json(
        [
            "npm",
            "sbom",
            "--sbom-format",
            "spdx",
            "--workspace",
            "@inodune/desktop",
            "--omit=dev",
            "--package-lock-only",
        ],
        cwd=ROOT,
    )
    result["name"] = "Inodune frontend production dependency graph"
    path = OUT / "inodune-frontend.spdx.json"
    path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def generate_rust() -> Path:
    env = os.environ.copy()
    local_cargo_home = ROOT / ".tools" / "cargo"
    local_rustup_home = ROOT / ".tools" / "rustup"
    cargo = shutil.which("cargo") or str(local_cargo_home / "bin" / "cargo")
    if (local_cargo_home / "registry").is_dir():
        env["CARGO_HOME"] = str(local_cargo_home)
    if local_rustup_home.is_dir():
        env["RUSTUP_HOME"] = str(local_rustup_home)
    env["RUSTUP_TOOLCHAIN"] = "1.93.0"
    metadata = run_json(
        [
            cargo,
            "metadata",
            "--locked",
            "--offline",
            "--format-version",
            "1",
        ],
        cwd=ROOT,
        env=env,
    )
    lock = __import__("tomllib").loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
    lock_checksums = {
        (package["name"], package["version"]): package.get("checksum")
        for package in lock.get("package", [])
    }
    all_packages = {package["id"]: package for package in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    roots = [
        package["id"]
        for package in metadata["packages"]
        if package["name"] in ("inodune-desktop", "inodune-cli")
    ]
    reachable: set[str] = set()
    pending = list(roots)
    while pending:
        current = pending.pop()
        if current in reachable:
            continue
        reachable.add(current)
        pending.extend(dependency["pkg"] for dependency in nodes.get(current, {}).get("deps", []))

    ids = {package_id: spdx_id("Cargo", package_id) for package_id in reachable}
    packages: list[dict] = []
    relationships = [
        {"spdxElementId": "SPDXRef-DOCUMENT", "relatedSpdxElement": ids[root], "relationshipType": "DESCRIBES"}
        for root in roots
        if root in ids
    ]
    for package_id in sorted(reachable):
        package = all_packages[package_id]
        entry = {
            "name": package["name"],
            "SPDXID": ids[package_id],
            "versionInfo": package["version"],
            "downloadLocation": "NOASSERTION",
            "filesAnalyzed": False,
            "licenseDeclared": package.get("license") or "NOASSERTION",
            "externalRefs": [
                {
                    "referenceCategory": "PACKAGE-MANAGER",
                    "referenceType": "purl",
                    "referenceLocator": f"pkg:cargo/{quote(package['name'], safe='-_.')}@{package['version']}",
                }
            ],
        }
        checksum = lock_checksums.get((package["name"], package["version"]))
        if checksum:
            entry["checksums"] = [{"algorithm": "SHA256", "checksumValue": checksum}]
        if package_id in roots:
            entry["primaryPackagePurpose"] = "APPLICATION"
        packages.append(entry)
        for dependency in nodes.get(package_id, {}).get("deps", []):
            dependency_id = dependency["pkg"]
            if dependency_id in ids:
                relationships.append(
                    {
                        "spdxElementId": ids[package_id],
                        "relatedSpdxElement": ids[dependency_id],
                        "relationshipType": "DEPENDS_ON",
                    }
                )

    document = create_document("Inodune Rust dependency graph", packages, relationships)
    path = OUT / "inodune-rust.spdx.json"
    path.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def write_checksums() -> Path:
    paths = []
    for suffix in ("*.deb", "*.rpm", "*.tar.gz"):
        found = sorted(OUT.glob(suffix))
        if len(found) != 1:
            raise RuntimeError(f"Expected one artifact matching {suffix}, got {len(found)}")
        paths.extend(found)
    paths.extend(OUT / name for name in ("install-user.py", "inodune-frontend.spdx.json", "inodune-rust.spdx.json"))
    entries = [f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}" for path in paths]
    sums = OUT / "SHA256SUMS"
    sums.write_text("\n".join(entries) + "\n", encoding="utf-8")
    return sums


def main() -> None:
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--frontend-only", action="store_true")
    group.add_argument("--rust-only", action="store_true")
    parser.add_argument("--checksums", action="store_true")
    args = parser.parse_args()
    if (not args.frontend_only or args.checksums) and os.environ.get("INODUNE_PACKAGER") != "1":
        raise SystemExit("Rust SBOM/checksums must run inside the packager through package-linux.py")
    OUT.mkdir(parents=True, exist_ok=True)
    paths = []
    if not args.rust_only:
        paths.append(generate_frontend())
    if not args.frontend_only:
        paths.append(generate_rust())
    if args.checksums:
        paths.append(write_checksums())
    for path in paths:
        print(f"Wrote {path} ({path.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
