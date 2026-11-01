"""Bind-mount adversarial checks in private namespaces, on new TempDirs only."""
import json,os,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[1]
env=dict(os.environ)
local=root/'.tools/cargo/bin/cargo'
if local.exists():
    env.update(RUSTUP_HOME=str(root/'.tools/rustup'),CARGO_HOME=str(root/'.tools/cargo'))
    cargo=str(local)
else:cargo='cargo'
checks=[('inodune-clean','lib','executor::tests::bind_mount_replacement_is_not_traversed'),('inodune-core','test','refuses_bind_mounts_in_private_user_namespace')]
for package,kind,test in checks:
    result=subprocess.run([cargo,'test','--locked','-p',package,'--no-run','--message-format=json'],cwd=root,capture_output=True,text=True,env=env,check=True)
    artifacts=[json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
    binary=next(value['executable'] for value in artifacts if value.get('reason')=='compiler-artifact' and value.get('executable') and value['target']['kind']==[kind])
    subprocess.run(['unshare','--user','--map-root-user','--mount','--propagation','private',binary,'--ignored','--exact',test],check=True,cwd=root)
print('PASS: both adversarial mount tests; task-created temporary fixtures in private user/mount namespaces, no sudo or host mount changes.')
