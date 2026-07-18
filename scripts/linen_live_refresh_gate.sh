#!/usr/bin/env bash
# LINEN_LIVE_REFRESH_GATE — live object discovery without reboot, one boot.
#   1. initial Linen open: snapshot count=3 (system objects)
#   2. spindle mkdoc zeta → reopen Linen → count=4 (new doc appeared live)
#   3. nav to zeta, Enter → quil switches to it (disk_doc path_id=3)
#   4. spindle mvdoc 3 omega → reopen Linen → count=4, no duplicates,
#      linen rescan reports renamed=1
#   5. spindle rmdoc 3 → reopen Linen → count=3 (deletion propagated)
# Usage: SKIP_BUILD=1 ./scripts/linen_live_refresh_gate.sh
set -uo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  ./scripts/entrypoint_build.sh > /tmp/sexos_llr_build.log 2>&1 || { echo "ROW build FAIL"; exit 1; }
  echo "ROW build PASS"
else
  echo "ROW build SKIP"
fi
D=/tmp/sexos_llr
mkdir -p "$D"
NVME="$D/nvme.img"; rm -f "$NVME"; dd if=/dev/zero of="$NVME" bs=512 count=2048 2>/dev/null
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
  echo "[llr] key miss key=$1 re=$2"
  return 1
}
tw() { for c in "$@"; do kv "$c" "\\[spindle\\.input\\.recv\\] key=printable ch=$c" 6; done; }
sp() { kv spc '\[spindle\.input\.recv\] key=printable ch= ' 6 || k spc; }
open_linen() {
  kv grave_accent '\[shell\.palette\.item\] idx=0' 8
  kv j '\[shell\.palette\.select\] old=0 new=1' 8
  kv j '\[shell\.palette\.select\] old=1 new=2' 8
  kv ret '\[shell\.palette\.exec\] idx=2' 10
  sleep 2
}

qemu-system-x86_64 -M q35 -m 512M -cdrom sexos-v1.0.0.iso \
  -drive "if=none,id=nvm,file=$NVME,format=raw" \
  -device "nvme,serial=sexos01,drive=nvm" \
  -serial "file:$L" -qmp "unix:$D/q.sock,server=on,wait=off" \
  -display none -no-reboot -no-shutdown &
QPID=$!
trap 'kill $QPID 2>/dev/null; sleep 1; kill -9 $QPID 2>/dev/null' EXIT

wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[llr] WARN publish"
sleep 3

# 1. initial linen open → count=3
open_linen
wait_marker '\[linen\.remote\.snapshot\.ok\] count=3' "$L" 60 || echo "[llr] WARN snap3"

# 2. spindle mkdoc zeta → reopen linen → count=4
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1
tw m k d o c; sp; tw z e t a
kv ret '\[spindle\.mkdoc\] id=3 ok=1' 20
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1
open_linen
wait_marker '\[linen\.remote\.snapshot\.ok\] count=4' "$L" 60 || echo "[llr] WARN snap4"

# 3. nav to zeta (4th entry: j j j), Enter → quil disk_doc path 3
kv j '\[linen\.nav\.select\.ok\]|\[shell\.action\.select_next_linen\]' 8
kv j '\[linen\.nav\.select\.ok\]|\[shell\.action\.select_next_linen\]' 8
kv j '\[linen\.nav\.select\.ok\]|\[shell\.action\.select_next_linen\]' 8
k ret
wait_marker '\[quil\.open\.disk_doc\.recv\].*path_id=3' "$L" 30 || echo "[llr] WARN open zeta"
sleep 2

# 4. rename via spindle → reopen linen → rescan renamed=1
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1
tw m v d o c; sp; kv 3 '\[spindle\.input\.recv\] key=printable ch=3' 6; sp; tw o m e g a
kv ret '\[spindle\.mvdoc\] id=3 ok=1' 20
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1
open_linen
wait_marker '\[linen\.disk\.rescan\.ok\].*renamed=1.*live=4' "$L" 60 || echo "[llr] WARN rename rescan"

# 5. delete → reopen → count=3
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1
tw r m d o c; sp; kv 3 '\[spindle\.input\.recv\] key=printable ch=3' 6
kv ret '\[spindle\.rmdoc\] id=3 ok=1' 20
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1
open_linen
sleep 2

FAILED=0
r() { echo "ROW $1 $2"; [[ "$2" == PASS* ]] || FAILED=1; }
grep -q '\[linen\.remote\.snapshot\.ok\] count=3' "$L" && r initial_list_3 PASS || r initial_list_3 FAIL
grep -q '\[linen\.remote\.snapshot\.ok\] count=4' "$L" && r new_doc_appears_live PASS || r new_doc_appears_live FAIL
grep -qE '\[quil\.open\.disk_doc\.recv\].*path_id=3' "$L" && r open_new_doc_from_list PASS || r open_new_doc_from_list FAIL
grep -qE '\[linen\.disk\.rescan\.ok\].*renamed=1.*live=4' "$L" && r rename_propagates PASS || r rename_propagates FAIL
LAST_SNAP=$(grep -oE '\[linen\.remote\.snapshot\.ok\] count=[0-9]+' "$L" | tail -1)
[[ "$LAST_SNAP" == "[linen.remote.snapshot.ok] count=3" ]] && r delete_propagates PASS || r delete_propagates "FAIL last=$LAST_SNAP"
# no duplicates: no snapshot ever exceeded 4
grep -qE '\[linen\.remote\.snapshot\.ok\] count=([5-9]|[0-9]{2,})' "$L" && r no_duplicates FAIL || r no_duplicates PASS
grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT' "$L" && r fault_free FAIL || r fault_free PASS
if [[ "$FAILED" == "0" ]]; then echo "[linen.live.refresh.gate.result] PASS"; else echo "[linen.live.refresh.gate.result] FAIL"; exit 1; fi
