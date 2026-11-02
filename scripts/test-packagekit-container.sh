#!/bin/sh
# Host: build helper first; all package installation/removal occurs in disposable container.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
helper=${1:-"$root/target/release/inodune-uninstall-helper"}
test -x "$helper"
output=$(mktemp "${TMPDIR:-/tmp}/inodune-pk-test.XXXXXX")
trap 'rm -f "$output"' EXIT HUP INT TERM
status=0
podman run -i --rm --network=bridge --security-opt=label=disable \
  -v "$helper:/opt/inodune-uninstall-helper:ro" ubuntu:26.04 bash -se >"$output" <<'CONTAINER' || status=$?
set -eu
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y packagekit polkitd systemd dbus hello python3 util-linux libglib2.0-0t64 base-files sed login mawk
useradd -m --user-group -u 23456 inodunetest
mkdir -p /run/dbus /etc/polkit-1/rules.d
cat > /etc/polkit-1/rules.d/49-inodune-container.rules <<'RULE'
polkit.addRule(function(action, subject) {
  if (subject.user == "inodunetest" && action.id == "org.freedesktop.packagekit.package-remove") {
    return polkit.Result.YES;
  }
});
RULE
dbus-daemon --system --fork
/usr/lib/polkit-1/polkitd --no-debug &
sleep 2
for package in base-files sed login mawk systemd polkitd; do
  test "$(dpkg-query -W -f='${Status}' "$package")" = 'install ok installed'
done
for package in base-files sed login mawk; do
  metadata=$(dpkg-query -W -f='${Essential} ${Priority}' "$package")
  case "$metadata" in yes\ *|*\ required) ;; *) echo "FAIL: $package is not Essential/required" >&2; exit 1;; esac
  printf '%s: %s\n' "$package" "$metadata"
done
# PackageKit starts by D-Bus activation. Helper remains non-root, as on desktop.
setpriv --reuid=23456 --regid=23456 --init-groups python3 - <<'PY'
import json, subprocess
helper = '/opt/inodune-uninstall-helper'
def request(name, remove=False):
    p = subprocess.Popen([helper], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
    p.stdin.write(json.dumps({'source':'apt','id':name,'ids':[name]})+'\n'); p.stdin.flush()
    line = p.stdout.readline()
    print(name+': '+line.strip(), flush=True)
    reply = json.loads(line)
    if remove:
        assert 'error' not in reply, reply
        p.stdin.write(json.dumps(reply['id'])+'\n'); p.stdin.flush()
        p.stdin.close()
        result = p.stdout.read(); print(result, flush=True)
        assert p.wait(timeout=600) == 0, result
        assert 'completed' in result
    else:
        assert 'error' in reply, reply
        p.stdin.close()
        assert p.wait(timeout=60) != 0
for name in ['bash','dpkg','hello;id','coreutils','rust-coreutils','coreutils-from-uutils','packagekit','linux-image-fixture','kernel-core-fixture','systemd','polkitd','base-files','sed','login','mawk','apt','sudo','polkit','inodune']:
    request(name)
request('hello', True)
PY
if dpkg-query -W -f='${Status}' hello 2>/dev/null | grep -q 'install ok installed'; then
  echo 'FAIL: hello remains installed'; exit 1
fi
echo HELLO_REMOVED_VIA_PACKAGEKIT
CONTAINER
cat "$output"
[ "$status" -eq 0 ] || exit "$status"
if ! grep -Fxq 'HELLO_REMOVED_VIA_PACKAGEKIT' "$output"; then
  echo 'FAIL: container did not report HELLO_REMOVED_VIA_PACKAGEKIT' >&2
  exit 1
fi
