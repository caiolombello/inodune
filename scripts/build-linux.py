"""Build compatible Linux binaries in a rootless, offline Debian 12 container.

Only the source snapshot and task-owned output/cache directories are mounted.
The builder image is local; this command never publishes anything.
"""
import os,shutil,subprocess,tempfile
from pathlib import Path
root=Path(__file__).resolve().parents[1]
image='localhost/inodune-builder:rust1.93-bookworm'
if subprocess.run(['podman','image','exists',image]).returncode:
    subprocess.run(['podman','build','--http-proxy=false','-f',str(root/'packaging/Containerfile.builder'),'-t',image,str(root/'packaging')],check=True)
# Frontend and lockfile checks are separate gates, run before entering the builder.
out=root/'target/compat';out.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='inodune-build-source-') as temporary:
    snapshot=Path(temporary)
    for name in ['Cargo.toml','Cargo.lock','rust-toolchain.toml','LICENSE']:
        shutil.copyfile(root/name,snapshot/name)
    shutil.copytree(root/'crates',snapshot/'crates')
    shutil.copytree(root/'tests/fixtures',snapshot/'tests/fixtures')
    shutil.copytree(root/'assets/brand',snapshot/'assets/brand')
    shutil.copytree(root/'apps/desktop/src-tauri',snapshot/'apps/desktop/src-tauri',ignore=shutil.ignore_patterns('target','gen'))
    shutil.copytree(root/'apps/desktop/dist',snapshot/'apps/desktop/dist')
    prefix=['podman','run','--rm','--userns=keep-id:uid=1000,gid=1000','--network=none','--cap-drop=all','--security-opt=no-new-privileges','--security-opt=label=disable',
        '--env','RUSTUP_TOOLCHAIN=1.93.0','--env','CARGO_BUILD_JOBS=3','--env','RUSTFLAGS=--remap-path-prefix=/workspace=.',
        '--volume',f'{snapshot}:/workspace:rw','--volume',f'{out}:/out:rw','--volume',f'{root}/.tools/cargo/registry:/cache/cargo/registry:ro',image]
    subprocess.run(prefix+['build','--frozen','--release','--target-dir','/out','-p','inodune-desktop','--features','custom-protocol'],check=True)
    subprocess.run(prefix+['build','--frozen','--release','--target-dir','/out','-p','inodune-cli'],check=True)
print(f'Built standalone production binaries in {out}/release (no fixture-tests).')
