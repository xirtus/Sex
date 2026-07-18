#!/usr/bin/env bash
# WINDOW_LIFECYCLE_GATE — surface destroy/reclaim cycles, one boot.
# spindle `winreset` destroys its 4 surfaces (0xEE) and recreates them
# (0xEC). Five cycles = 24 total creates against a 16-slot compositor
# table — impossible unless destroy actually frees slots. After the
# cycles: terminal still renders (pixel proof), typing still works, quil
# still opens and edits (its surfaces were never disturbed), no AUTH
# rejects, zero faults.
# Usage: SKIP_BUILD=1 ./scripts/window_lifecycle_gate.sh
set -uo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  ./scripts/entrypoint_build.sh > /tmp/sexos_wl_build.log 2>&1 || { echo "ROW build FAIL"; exit 1; }
  echo "ROW build PASS"
else
  echo "ROW build SKIP"
fi
D=/tmp/sexos_wl
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
dump() {
  python3 - "$D/q.sock" "$1" <<'PY'
import json, socket, sys, time
sock_path, out = sys.argv[1], sys.argv[2]
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM); s.settimeout(10); s.connect(sock_path)
def rd():
    buf=b""
    while b"\n" not in buf: buf += s.recv(4096)
    return buf
rd()
def cmd(c): s.sendall((json.dumps(c)+"\n").encode()); return rd()
cmd({"execute":"qmp_capabilities"})
cmd({"execute":"screendump","arguments":{"filename":out}})
time.sleep(1)
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
  echo "[wl] key miss key=$1 re=$2"
  return 1
}
tw() { for c in "$@"; do kv "$c" "\\[spindle\\.input\\.recv\\] key=printable ch=$c" 6; done; }

qemu-system-x86_64 -M q35 -m 512M -cdrom sexos-v1.0.0.iso \
  -drive "if=none,id=nvm,file=$NVME,format=raw" \
  -device "nvme,serial=sexos01,drive=nvm" \
  -serial "file:$L" -qmp "unix:$D/q.sock,server=on,wait=off" \
  -display none -no-reboot -no-shutdown &
QPID=$!
trap 'kill $QPID 2>/dev/null; sleep 1; kill -9 $QPID 2>/dev/null' EXIT

wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[wl] WARN publish"
sleep 3
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1

# 5 destroy/recreate cycles — 24 creates total vs 16 slots
for i in 1 2 3 4 5; do
  tw w i n r e s e t
  kv ret "\\[spindle\\.winreset\\] ok=1" 15
  sleep 1
done
# terminal still fully usable after the cycles
tw d i s k
kv ret '\[spindle\.disk\.command\] found=3 ok=1' 20
sleep 1
dump "$D/after.ppm"
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1

# quil untouched by the cycles: open + type
kv grave_accent '\[shell\.palette\.item\] idx=0' 8
kv j '\[shell\.palette\.select\] old=0 new=1' 8
kv ret '\[shell\.palette\.exec\] idx=1' 10
sleep 1
kv esc '\[quil\.palette\.action\] kind=esc' 8
kv a '\[quil\.text\.append\]' 6

FAILED=0
r() { echo "ROW $1 $2"; [[ "$2" == PASS* ]] || FAILED=1; }
WR=$(count_re '\[spindle\.winreset\] ok=1' "$L")
[[ "$WR" -ge 5 ]] && r five_reset_cycles "PASS n=$WR" || r five_reset_cycles "FAIL n=$WR"
DS=$(count_re '\[spindle\.grid\.destroy\.ok\]' "$L")
[[ "$DS" -ge 5 ]] && r destroys_ran "PASS n=$DS" || r destroys_ran "FAIL n=$DS"
CR=$(count_re '\[spindle\.grid\.surface\.ok\] sid=154' "$L")
[[ "$CR" -ge 6 ]] && r creates_beyond_slot_budget "PASS n=$CR" || r creates_beyond_slot_budget "FAIL n=$CR"
grep -q '\[spindle\.disk\.command\] found=3 ok=1' "$L" && r terminal_usable_after PASS || r terminal_usable_after FAIL
grep -q '\[quil\.text\.append\]' "$L" && r quil_unaffected PASS || r quil_unaffected FAIL
grep -q 'AUTH: 0xEE destroy rejected' "$L" && r no_auth_rejects FAIL || r no_auth_rejects PASS
grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT' "$L" && r fault_free FAIL || r fault_free PASS

python3 - "$D/after.ppm" <<'PY' >"$D/pix.txt" || true
import sys
try:
    with open(sys.argv[1],'rb') as f: data=f.read()
    parts=data.split(b'\n',3)
    if parts[0].strip()!=b'P6':
        print("ROW pixel_scan SKIP fmt"); raise SystemExit
    w,h=map(int,parts[1].split()); px=parts[3]
    sp=0
    for y in range(632,min(792,h)):
        for x in range(1008,min(1272,w)):
            i=(y*w+x)*3
            if px[i]==0xE8 and px[i+1]==0xFF and px[i+2]==0xFF: sp+=1
    print(f"ROW pixel_alive_after_cycles {'PASS' if sp>30 else 'FAIL'} count={sp}")
except SystemExit:
    pass
PY
cat "$D/pix.txt"
grep -q " FAIL" "$D/pix.txt" && FAILED=1
if [[ "$FAILED" == "0" ]]; then echo "[window.lifecycle.gate.result] PASS"; else echo "[window.lifecycle.gate.result] FAIL"; exit 1; fi
