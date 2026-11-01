#!/usr/bin/env python3
"""Launch the packaged desktop in an empty, isolated XDG profile under Xvfb."""
from __future__ import annotations

import argparse
import json
import os
import pwd
import shutil
import signal
import subprocess
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path


def run(args: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, check=True, text=True, **kwargs)


def package_version() -> str:
    if Path("/usr/bin/dpkg-query").exists():
        return run(["dpkg-query", "-W", "-f=${Version}", "inodune"], capture_output=True).stdout.strip()
    return run(
        ["rpm", "-qf", "--qf", "%{VERSION}-%{RELEASE}", "/usr/bin/inodune-desktop"],
        capture_output=True,
    ).stdout.strip()


def check_dynamic_dependencies() -> None:
    result = run(["ldd", "/usr/bin/inodune-desktop"], capture_output=True)
    missing = [line.strip() for line in result.stdout.splitlines() if "not found" in line]
    if missing:
        raise RuntimeError("unresolved GUI runtime libraries: " + "; ".join(missing))


def runtime_package_versions() -> dict[str, str]:
    if Path("/usr/bin/dpkg-query").exists():
        result = run(
            [
                "dpkg-query",
                "-W",
                "-f=${binary:Package}=${Version}\\n",
                "libwebkit2gtk-4.1-0",
                "libgtk-3-0*",
                "xvfb",
                "x11-apps",
            ],
            capture_output=True,
        )
    else:
        result = run(
            [
                "rpm",
                "-q",
                "--qf",
                "%{NAME}=%{VERSION}-%{RELEASE}\\n",
                "webkit2gtk4.1",
                "gtk3",
                "xorg-x11-server-Xvfb",
                "xwd",
            ],
            capture_output=True,
        )
    return {
        line.split("=", 1)[0]: line.split("=", 1)[1]
        for line in result.stdout.splitlines()
        if "=" in line and line.split("=", 1)[1].strip()
    }


def visible_windows() -> list[str]:
    result = subprocess.run(
        ["xdotool", "search", "--onlyvisible", "--name", "Inodune"],
        capture_output=True,
        text=True,
        check=False,
    )
    return [line for line in result.stdout.splitlines() if line.isdigit()]


def capture_screenshot(output: Path, window_id: str) -> int:
    xwd_path = output / "inodune-desktop.xwd"
    with xwd_path.open("wb") as capture:
        run(["xwd", "-id", window_id, "-silent"], stdout=capture)
    converter = shutil.which("magick") or shutil.which("convert")
    if converter is None:
        raise RuntimeError("ImageMagick conversion command is missing")
    png_path = output / "inodune-desktop.png"
    run([converter, str(xwd_path), str(png_path)], capture_output=True)
    xwd_path.unlink()
    colors = int(run(["identify", "-format", "%k", str(png_path)], capture_output=True).stdout)
    if colors < 100:
        raise RuntimeError(f"GUI screenshot looks blank ({colors} colors)")
    return colors


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    output = args.output_dir
    output.mkdir(parents=True, exist_ok=True)

    user = pwd.getpwuid(os.getuid())
    print("checking packaged ELF dependencies", flush=True)
    check_dynamic_dependencies()
    display = ":99"
    os.environ["DISPLAY"] = display
    server_log_path = output / "xvfb.log"
    process: subprocess.Popen[bytes] | None = None
    server: subprocess.Popen[bytes] | None = None
    with tempfile.TemporaryDirectory(prefix="inodune-runtime-smoke-") as home:
        home_path = Path(home)
        runtime = home_path / "runtime"
        runtime.mkdir(mode=0o700)
        env = os.environ.copy()
        env.update(
            {
                "HOME": str(home_path),
                "XDG_CONFIG_HOME": str(home_path / ".config"),
                "XDG_CACHE_HOME": str(home_path / ".cache"),
                "XDG_DATA_HOME": str(home_path / ".local" / "share"),
                "XDG_STATE_HOME": str(home_path / ".local" / "state"),
                "XDG_RUNTIME_DIR": str(runtime),
                "LIBGL_ALWAYS_SOFTWARE": "1",
                "WEBKIT_DISABLE_COMPOSITING_MODE": "1",
            }
        )
        for key in ("XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_DATA_HOME", "XDG_STATE_HOME"):
            Path(env[key]).mkdir(parents=True, exist_ok=True)

        with server_log_path.open("wb") as server_log:
            server = subprocess.Popen(
                ["Xvfb", display, "-screen", "0", "1600x1000x24", "-nolisten", "tcp", "-ac"],
                stdin=subprocess.DEVNULL,
                stdout=server_log,
                stderr=subprocess.STDOUT,
            )
            try:
                deadline = time.monotonic() + 10
                while time.monotonic() < deadline:
                    if server.poll() is not None:
                        raise RuntimeError(f"Xvfb exited early with status {server.returncode}")
                    display_ready = subprocess.run(
                        ["xdotool", "getdisplaygeometry"], capture_output=True, text=True, check=False
                    )
                    if display_ready.returncode == 0:
                        break
                    time.sleep(0.2)
                else:
                    raise RuntimeError("Xvfb did not accept local X11 connections within 10 seconds")

                log_path = output / "desktop.log"
                with log_path.open("wb") as log:
                    print("starting packaged desktop under Xvfb", flush=True)
                    process = subprocess.Popen(
                        ["/usr/bin/inodune-desktop"],
                        env=env,
                        stdin=subprocess.DEVNULL,
                        stdout=log,
                        stderr=subprocess.STDOUT,
                    )
                    window_id: str | None = None
                    try:
                        deadline = time.monotonic() + 60
                        while time.monotonic() < deadline:
                            if process.poll() is not None:
                                raise RuntimeError(f"desktop exited early with status {process.returncode}")
                            windows = visible_windows()
                            if windows:
                                window_id = windows[0]
                                print("visible Inodune window appeared", flush=True)
                                break
                            time.sleep(0.5)
                        if window_id is None:
                            raise RuntimeError("no visible Inodune window appeared within 60 seconds")
                        title = run(["xdotool", "getwindowname", window_id], capture_output=True).stdout.strip()
                        if "Inodune" not in title:
                            raise RuntimeError(f"unexpected desktop window title: {title!r}")
                        time.sleep(10)
                        print("capturing X11 screenshot", flush=True)
                        screenshot_colors = capture_screenshot(output, window_id)
                        if process.poll() is not None:
                            raise RuntimeError("desktop exited after its window appeared")
                    finally:
                        if process.poll() is None:
                            process.send_signal(signal.SIGTERM)
                            try:
                                process.wait(timeout=10)
                            except subprocess.TimeoutExpired:
                                process.kill()
                                process.wait(timeout=5)
            finally:
                if server.poll() is None:
                    server.send_signal(signal.SIGTERM)
                    try:
                        server.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        server.kill()
                        server.wait(timeout=3)

    os_release = Path("/etc/os-release").read_text(encoding="utf-8")
    evidence = {
        "checkedAtUtc": datetime.now(timezone.utc).isoformat(),
        "distribution": os_release,
        "packageVersion": package_version(),
        "installedRuntimePackages": runtime_package_versions(),
        "runtimeUser": {"name": user.pw_name, "uid": os.getuid(), "home": user.pw_dir},
        "network": "disabled by Podman (--network=none)",
        "xdgHome": "empty TemporaryDirectory, removed after app exit",
        "gui": {
            "executable": "/usr/bin/inodune-desktop",
            "visibleWindow": True,
            "windowTitle": title,
            "processRemainedLiveAfterWindowAppeared": True,
            "screenshot": "inodune-desktop.png",
            "screenshotColors": screenshot_colors,
            "graphicsMode": "test-only software compositing fallback (WEBKIT_DISABLE_COMPOSITING_MODE=1)",
        },
        "dynamicLibraries": "ldd found no unresolved dependencies",
        "scanRequested": False,
        "cleanupRequested": False,
    }
    (output / "runtime-smoke.json").write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"packageVersion": evidence["packageVersion"], "windowTitle": title, "screenshot": str(output / "inodune-desktop.png")}))


if __name__ == "__main__":
    main()
