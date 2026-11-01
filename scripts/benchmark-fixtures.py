#!/usr/bin/env python3
"""Measure the real read-only scanner on private, task-created Rust fixtures.

The benchmark never opens a user project: every --root is a fresh directory
owned by this script beneath tempfile.gettempdir(). No cleanup command is run.
"""
from __future__ import annotations

import hashlib
import json
import os
import platform
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "target/debug/inodune-cli"
SIZES = (100, 1_000, 10_000)


def main() -> None:
    if not CLI.is_file():
        raise SystemExit(f"Scanner binário ausente: {CLI}; execute cargo build -p inodune-cli")
    reports = []
    with tempfile.TemporaryDirectory(prefix="inodune-benchmark-fixtures-") as temp:
        base = Path(temp)
        for size in SIZES:
            projects = base / f"projects-{size}"
            projects.mkdir(mode=0o700)
            for index in range(size):
                target = projects / f"project-{index:05d}" / "target" / "debug"
                target.mkdir(parents=True)
                (target / "artifact.fixture").write_bytes(b"benchmark fixture; never executable\n")
                (target.parent.parent / "Cargo.toml").write_text(
                    "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\n", encoding="utf-8"
                )

            timing = base / f"time-{size}.txt"
            start = time.perf_counter()
            completed = subprocess.run(
                [
                    "/usr/bin/time", "-f", "%e %M", "-o", str(timing),
                    str(CLI), "scan", "--root", str(projects), "--scope", "projects", "--json",
                ],
                cwd=ROOT,
                env={**os.environ, "HOME": str(base), "XDG_CACHE_HOME": str(base / "xdg-cache")},
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                check=False,
            )
            elapsed = time.perf_counter() - start
            if completed.returncode != 0:
                raise RuntimeError(f"scan {size} falhou ({completed.returncode}): {completed.stderr[-2000:]}")
            view = json.loads(completed.stdout)
            found = len(view.get("candidates", []))
            if found != size:
                raise AssertionError(f"Esperados {size} alvos sintéticos, encontrados {found}")
            wall_s, max_rss_kib = timing.read_text(encoding="ascii").split()
            reports.append(
                {
                    "fixture_candidates": size,
                    "candidates_found": found,
                    "visited_entries": view["stats"]["visited"],
                    "skipped_entries": view["stats"]["skipped"],
                    "coverage_complete": view["complete"],
                    "wall_seconds_time(1)": float(wall_s),
                    "wall_seconds_python(2)": round(elapsed, 6),
                    "max_rss_kib": int(max_rss_kib),
                    "exit_code": completed.returncode,
                }
            )

    result = {
        "schema_version": 1,
        "kind": "read_only_scanner_fixture_benchmark",
        "generated_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "environment": {
            "os": platform.platform(),
            "kernel": platform.release(),
            "machine": platform.machine(),
            "python": platform.python_version(),
            "rustc": subprocess.check_output(
                [str(ROOT / ".tools/cargo/bin/rustc"), "--version"],
                cwd=ROOT,
                env={
                    **os.environ,
                    "RUSTUP_HOME": str(ROOT / ".tools/rustup"),
                    "CARGO_HOME": str(ROOT / ".tools/cargo"),
                },
                text=True,
            ).strip(),
            "binary": str(CLI.relative_to(ROOT)),
            "binary_sha256": subprocess.check_output(["sha256sum", str(CLI)], text=True).split()[0],
            "scanner_source_sha256": {
                path: hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
                for path in [
                    "crates/inodune-cli/src/main.rs",
                    "crates/inodune-core/src/scan.rs",
                    "crates/inodune-core/src/lib.rs",
                ]
            },
            "git_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
            "fixture_parent": "private TemporaryDirectory; exact path intentionally omitted",
        },
        "runs": reports,
        "safety": "Only task-created temporary directories were scanned. No cleanup command ran.",
        "limitations": [
            "Measures CLI scanner traversal, not Svelte rendering or representative desktop hardware.",
            "Wall time includes process startup and JSON serialization.",
            "Not a 60 fps claim; UI frame budget requires a representative GPU/CPU and native UI capture.",
        ],
    }
    out = ROOT / "docs/evidence/f4/scanner-benchmark.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
