#!/usr/bin/env python3
"""Build and run isolated GUI smoke images for the local DEB and RPM packages."""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import tempfile
import uuid
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PLATFORMS = {
    "ubuntu-24.04": {
        "package": ROOT / "target/packages/Inodune_0.1.0_amd64.deb",
        "containerfile": ROOT / "packaging/Containerfile.smoke.ubuntu",
        "image": "docker.io/library/ubuntu@sha256:78d4744ead1f5b19b04d0df558ab182a5300903900755e515b2ca4d10eb2044d",
        "copy_name": "Inodune.deb",
    },
    "fedora-44": {
        "package": ROOT / "target/packages/Inodune-0.1.0-1.x86_64.rpm",
        "containerfile": ROOT / "packaging/Containerfile.smoke.fedora",
        "image": "registry.fedoraproject.org/fedora@sha256:7a04a2f9d11bba3ff8016ae79cc01ba49965578746c930702e141a3be1f8d4de",
        "copy_name": "Inodune.rpm",
    },
}


def run(
    args: list[str],
    *,
    capture: bool = False,
    timeout: int = 1200,
    cwd: Path | None = None,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        check=True,
        text=True,
        capture_output=capture,
        timeout=timeout,
        cwd=cwd,
    )


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def smoke(platform: str, output_root: Path, *, rebuild: bool, source_revision: str) -> None:
    config = PLATFORMS[platform]
    package = config["package"]
    if not package.is_file():
        raise SystemExit(f"Package not found: {package}")
    package_hash = sha256(package)
    tag = f"localhost/inodune-runtime-smoke:{platform}-{package_hash[:12]}"
    evidence_dir = output_root / platform
    evidence_dir.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix=f"inodune-{platform}-build-") as temporary:
        context = Path(temporary)
        shutil.copy2(config["containerfile"], context / "Containerfile")
        shutil.copy2(package, context / config["copy_name"])
        shutil.copy2(ROOT / "scripts/runtime-smoke.py", context / "runtime-smoke.py")
        build_args = ["podman", "build", "--pull=always", "--tag", tag, "--file", "Containerfile"]
        if rebuild:
            build_args.append("--no-cache")
        build_args.append(".")
        print(f"Building {platform} runtime image from {config['image']} for package sha256:{package_hash}", flush=True)
        try:
            build_result = run(build_args, capture=True, timeout=1800, cwd=context)
        except subprocess.CalledProcessError as error:
            output = (error.stdout or "") + (error.stderr or "")
            print("\n".join(output.splitlines()[-100:]), flush=True)
            raise
        print("\n".join(build_result.stdout.splitlines()[-8:]), flush=True)

    image_id = run(["podman", "image", "inspect", "--format", "{{.Id}}", tag], capture=True).stdout.strip()
    container = f"inodune-smoke-{platform}-{uuid.uuid4().hex[:10]}"
    run_args = [
        "podman",
        "run",
        "--name",
        container,
        "--network=none",
        "--userns=keep-id",
        "--user=1000:1000",
        tag,
    ]
    try:
        print(f"Launching packaged GUI in {platform} with no network and no host mounts", flush=True)
        try:
            run_result = subprocess.run(run_args, text=True, capture_output=True, timeout=180)
        except subprocess.TimeoutExpired as error:
            print("Container GUI launch exceeded 180 seconds", flush=True)
            partial = error.stdout or b""
            if isinstance(partial, bytes):
                partial = partial.decode(errors="replace")
            if partial:
                print(partial, flush=True)
            diagnostics = evidence_dir / "runtime-smoke-debug.log"
            subprocess.run(
                ["podman", "cp", f"{container}:/tmp/inodune-smoke-output/desktop.log", str(diagnostics)],
                check=False,
                capture_output=True,
                text=True,
            )
            if diagnostics.exists():
                print("\n".join(diagnostics.read_text(errors="replace").splitlines()[-80:]), flush=True)
            raise
        if run_result.stdout:
            print(run_result.stdout, end="")
        if run_result.returncode != 0:
            raise RuntimeError(f"GUI smoke failed in {platform} (exit {run_result.returncode}):\n{run_result.stderr}")
        for name in ("runtime-smoke.json", "desktop.log", "inodune-desktop.png"):
            run(["podman", "cp", f"{container}:/tmp/inodune-smoke-output/{name}", str(evidence_dir / name)])
    finally:
        subprocess.run(["podman", "rm", "--force", container], check=False, capture_output=True, text=True)

    runtime = json.loads((evidence_dir / "runtime-smoke.json").read_text(encoding="utf-8"))
    runtime.update(
        {
            "platform": platform,
            "baseImage": config["image"],
            "localImageId": image_id,
            "packageFile": package.name,
            "packageSha256": package_hash,
            "sourceRevision": source_revision,
            "hostMounts": [],
            "hostInstallation": False,
        }
    )
    (evidence_dir / "runtime-smoke.json").write_text(json.dumps(runtime, indent=2) + "\n", encoding="utf-8")
    print(f"PASS {platform}: {runtime['gui']['windowTitle']}, package {runtime['packageVersion']}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", choices=["all", *PLATFORMS], default="all")
    parser.add_argument("--output-dir", type=Path, default=ROOT / "docs/evidence/f4")
    parser.add_argument("--source-revision", required=True, help="source commit used to build the packaged application")
    parser.add_argument("--rebuild", action="store_true", help="ignore cached smoke image layers")
    args = parser.parse_args()
    selected = list(PLATFORMS) if args.platform == "all" else [args.platform]
    for platform in selected:
        smoke(platform, args.output_dir, rebuild=args.rebuild, source_revision=args.source_revision)
    summary = {
        "checkedAtUtc": datetime.now(timezone.utc).isoformat(),
        "platforms": selected,
        "sourceRevision": args.source_revision,
        "results": {platform: "PASS" for platform in selected},
        "scope": "installed GUI process and visible window under Xvfb, isolated container, empty XDG home, network disabled",
    }
    args.output_dir.mkdir(parents=True, exist_ok=True)
    (args.output_dir / "runtime-smoke-summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"Evidence written under {args.output_dir}")


if __name__ == "__main__":
    main()
