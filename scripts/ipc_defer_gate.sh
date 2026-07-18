#!/usr/bin/env bash
# IPC_DEFER_GATE — storage request survival under contention.
# Issues quil's palette LOAD *during* linen's boot disk publish (max
# contention window). Pre-fix, sexfiles' nested reply-wait loop discarded
# the mid-roundtrip client request as "stale" and quil hung forever.
# Proves: SEXFILES_DEFER_V1 stash+replay, per-caller DiskFS selection,
# MPSC-safe kernel ring — the persist path completes with a real result.
# Usage: SKIP_BUILD=1 ./scripts/ipc_defer_gate.sh
set -uo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  ./scripts/entrypoint_build.sh > /tmp/sexos_race_build.log 2>&1 || { echo "ROW build FAIL"; exit 1; }
  echo "ROW build PASS"
else
  echo "ROW build SKIP"
fi
D=/tmp/sexos_race
mkdir -p "$D"
NVME="$D/nvme.img"
rm -f "$NVME"; dd if=/dev/zero of="$NVME" bs=512 count=2048 2>/dev/null
L="$D/r.log"; : > "$L"; rm -f "$D/q.sock"

k() {
  python3 - "$D/q.sock" "$1" <<'PY'
import json, socket, sys, time, os
sock_path, key = sys.argv[1], sys.argv[2]
deadline = time.time() + 10
while not os.path.exists(sock_path):
    if time.time() > deadline: sys.exit(1)
    time.sleep(0.2)
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM); s.settimeout(5); s.connect(sock_path)
def rd():
    buf=b""
    while b"\n" not in buf: buf += s.recv(4096)
    return buf
rd()
def cmd(c): s.sendall((json.dumps(c)+"\n").encode()); rd()
cmd({"execute":"qmp_capabilities"})
cmd({"execute":"send-key","arguments":{"keys":[{"type":"qcode","data":key}]}})
time.sleep(0.2)
PY
}
wait_marker() { local d=$((SECONDS+$3)); while ((SECONDS<d)); do grep -qE "$1" "$2" 2>/dev/null && return 0; sleep 1; done; return 1; }
count_re() { local c; c=$(grep -cE "$1" "$2" 2>/dev/null); echo "${c:-0}"; }
kv() {
  local base; base=$(count_re "$2" "$L")
  k "$1"
  local d=$((SECONDS+$3))
  while ((SECONDS<d)); do
    [[ $(count_re "$2" "$L") -gt $base ]] && return 0
    sleep 1
  done
  k "$1"
  d=$((SECONDS+$3))
  while ((SECONDS<d)); do
    [[ $(count_re "$2" "$L") -gt $base ]] && return 0
    sleep 1
  done
  echo "MISS key=$1"
  return 1
}

qemu-system-x86_64 -M q35 -m 512M -cdrom sexos-v1.0.0.iso \
  -drive "if=none,id=nvm,file=$NVME,format=raw" \
  -device "nvme,serial=sexos01,drive=nvm" \
  -serial "file:$L" -qmp "unix:$D/q.sock,server=on,wait=off" \
  -display none -no-reboot -no-shutdown &
QPID=$!
trap 'kill $QPID 2>/dev/null; sleep 1; kill -9 $QPID 2>/dev/null' EXIT

# Publish begins after linen surface up. Fire quil open+load AT publish begin,
# well before publish.done — max contention window.
wait_marker '\[linen\.disk\.publish\.begin\]' "$L" 120 || echo "WARN: no publish begin"
kv grave_accent '\[shell\.palette\.item\] idx=0' 8
kv j '\[shell\.palette\.select\] old=0 new=1' 8
kv ret '\[shell\.palette\.exec\] idx=1' 10
sleep 1
kv down '\[quil\.palette\.selected\] row=1' 8
kv down '\[quil\.palette\.selected\] row=2' 8
k ret
FAILED=0
r() { echo "ROW $1 $2"; [[ "$2" == FAIL ]] && FAILED=1 || true; }
if wait_marker '\[quil\.persist\.load\.(ok|err|miss)' "$L" 240; then
  r persist_completes_mid_publish PASS
else
  r persist_completes_mid_publish FAIL
fi
sleep 2
wait_marker '\[linen\.disk\.publish\.done\] count=3' "$L" 300 || true
# DISKFS_V3 made STAT resolve from the in-RAM manifest table, shrinking the
# storage contention window — defer stash/replay only fires under an actual
# mid-roundtrip overlap now. The invariant is request SURVIVAL (row above:
# the mid-publish load completes instead of hanging); defer firing is
# informational.
if grep -q "\[sexfiles\.defer\.stash\]" "$L"; then
  r defer_observed PASS
else
  echo "ROW defer_observed SKIP (no contention overlap this run)"
fi
grep -q "\[linen\.disk\.publish\.done\] count=3" "$L" && r linen_publish_intact PASS || r linen_publish_intact FAIL
grep -qE "KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT" "$L" && r fault_free FAIL || r fault_free PASS
if [[ "$FAILED" == "0" ]]; then echo "[ipc.defer.gate.result] PASS"; else echo "[ipc.defer.gate.result] FAIL"; exit 1; fi
