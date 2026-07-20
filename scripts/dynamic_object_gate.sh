#!/usr/bin/env bash
# DYNAMIC_OBJECT_GATE — DISKFS_V3 dynamic object lifecycle, two boots.
# Boot 1 (spindle terminal): mkdoc alpha → disk lists ALPHA (id 3) →
#   mkdoc alpha again → NAME EXISTS → mvdoc 3 beta → disk lists BETA →
#   rmdoc 0 → SYSTEM OBJECT protected.
# Boot 2 (same NVMe): disk still lists BETA (manifest survived reboot) →
#   rmdoc 3 → disk no longer lists it → mkdoc gamma reuses slot 3.
# Usage: SKIP_BUILD=1 ./scripts/dynamic_object_gate.sh
set -uo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  ./scripts/entrypoint_build.sh > /tmp/sexos_dyo_build.log 2>&1 || { echo "ROW build FAIL"; exit 1; }
  echo "ROW build PASS"
else
  echo "ROW build SKIP"
fi
D=/tmp/sexos_dyo
mkdir -p "$D"
NVME="$D/nvme.img"; rm -f "$NVME"; dd if=/dev/zero of="$NVME" bs=512 count=2048 2>/dev/null

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
  echo "[dyo] key miss key=$1 re=$2"
  return 1
}
tw() { # type each char, verified by spindle printable echo
  for c in "$@"; do
    kv "$c" "\\[spindle\\.input\\.recv\\] key=printable ch=$c" 6
  done
}
sp() { kv spc '\[spindle\.input\.recv\] key=printable ch= ' 6 || k spc; }

boot() {
  rm -f "$D/q.sock"; : > "$L"
  qemu-system-x86_64 -M q35 -m 512M -cdrom sexos-v1.0.0.iso \
    -drive "if=none,id=nvm,file=$NVME,format=raw" \
    -device "nvme,serial=sexos01,drive=nvm" \
    -serial "file:$L" -qmp "unix:$D/q.sock,server=on,wait=off" \
    -display none -no-reboot -no-shutdown &
  QPID=$!
}
stop() { kill $QPID 2>/dev/null; sleep 1; kill -9 $QPID 2>/dev/null; }
trap 'stop' EXIT

### BOOT 1
L="$D/b1.log"
boot
wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[dyo] WARN publish b1"
sleep 3
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1
# mkdoc alpha
tw m k d o c; sp; tw a l p h a
kv ret '\[spindle\.mkdoc\] id=3 ok=1' 20
# duplicate name rejected
tw m k d o c; sp; tw a l p h a
kv ret '\[spindle\.mkdoc\] ok=0 err=-8' 20
# list contains it
tw d i s k
kv ret '\[spindle\.disk\.command\] found=4' 20
# rename 3 -> beta
tw m v d o c; sp; kv 3 '\[spindle\.input\.recv\] key=printable ch=3' 6; sp; tw b e t a
kv ret '\[spindle\.mvdoc\] id=3 ok=1' 20
# delete system slot rejected
tw r m d o c; sp; kv 0 '\[spindle\.input\.recv\] key=printable ch=0' 6
kv ret '\[spindle\.rmdoc\] id=0 ok=0 err=-6' 20
stop

### BOOT 2 — same NVMe
L="$D/b2.log"
boot
wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[dyo] WARN publish b2"
sleep 3
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1
# beta survived reboot
tw d i s k
kv ret '\[spindle\.disk\.command\] found=4' 20
# delete it
tw r m d o c; sp; kv 3 '\[spindle\.input\.recv\] key=printable ch=3' 6
kv ret '\[spindle\.rmdoc\] id=3 ok=1' 20
# gone from list
tw d i s k
kv ret '\[spindle\.disk\.command\] found=3 ok=1' 20
# slot reuse: gamma lands in slot 3 again
tw m k d o c; sp; tw g a m m a
kv ret '\[spindle\.mkdoc\] id=3 ok=1' 20
sleep 1
stop

### Rows
FAILED=0
r() { echo "ROW $1 $2"; [[ "$2" == FAIL* ]] && FAILED=1 || true; }
grep -q '\[spindle\.mkdoc\] id=3 ok=1' "$D/b1.log" && r create_named_object PASS || r create_named_object FAIL
grep -q '\[spindle\.mkdoc\] ok=0 err=-8' "$D/b1.log" && r duplicate_name_rejected PASS || r duplicate_name_rejected FAIL
grep -q '\[spindle\.disk\.command\] found=4' "$D/b1.log" && r list_contains_object PASS || r list_contains_object FAIL
grep -q '\[spindle\.mvdoc\] id=3 ok=1' "$D/b1.log" && r rename PASS || r rename FAIL
grep -q '\[spindle\.rmdoc\] id=0 ok=0 err=-6' "$D/b1.log" && r system_slot_protected PASS || r system_slot_protected FAIL
grep -q '\[spindle\.disk\.command\] found=4' "$D/b2.log" && r object_survives_reboot PASS || r object_survives_reboot FAIL
grep -q '\[spindle\.rmdoc\] id=3 ok=1' "$D/b2.log" && r delete PASS || r delete FAIL
grep -q '\[spindle\.disk\.command\] found=3 ok=1' "$D/b2.log" && r list_reflects_delete PASS || r list_reflects_delete FAIL
[[ $(count_re '\[spindle\.mkdoc\] id=3 ok=1' "$D/b2.log") -ge 1 ]] && r slot_reuse PASS || r slot_reuse FAIL
grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT' "$D/b1.log" "$D/b2.log" && r fault_free FAIL || r fault_free PASS
if [[ "$FAILED" == "0" ]]; then echo "[dynamic.object.gate.result] PASS"; else echo "[dynamic.object.gate.result] FAIL"; exit 1; fi
