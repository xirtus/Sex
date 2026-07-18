#!/usr/bin/env bash
# QUIL_VIEWPORT_GATE — QUIL_TEXT_V3 follow-cursor scrolling, one boot.
# Builds a 26-line document (one char + newline per line), then:
#   bottom: typing pushed the view down       → [quil.view] top>0
#   top:    Home (cursor to byte 0)           → [quil.view] top=0
#   middle: Down x12 from top                 → intermediate top
# Screendumps at the three positions must be pairwise distinct in the
# quil surface region.
set -uo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  ./scripts/entrypoint_build.sh > /tmp/sexos_qv_build.log 2>&1 || { echo "ROW build FAIL"; exit 1; }
  echo "ROW build PASS"
else
  echo "ROW build SKIP"
fi
D=/tmp/sexos_qv
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
time.sleep(0.15)
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
  echo "[qv] key miss key=$1 re=$2"
  return 1
}

qemu-system-x86_64 -M q35 -m 512M -cdrom sexos-v1.0.0.iso \
  -drive "if=none,id=nvm,file=$NVME,format=raw" \
  -device "nvme,serial=sexos01,drive=nvm" \
  -serial "file:$L" -qmp "unix:$D/q.sock,server=on,wait=off" \
  -display none -no-reboot -no-shutdown &
QPID=$!
trap 'kill $QPID 2>/dev/null; sleep 1; kill -9 $QPID 2>/dev/null' EXIT

wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[qv] WARN publish"
sleep 3
kv grave_accent '\[shell\.palette\.item\] idx=0' 8
kv j '\[shell\.palette\.select\] old=0 new=1' 8
kv ret '\[shell\.palette\.exec\] idx=1' 10
sleep 1
kv down '\[quil\.palette\.selected\] row=1' 8
kv down '\[quil\.palette\.selected\] row=2' 8
kv down '\[quil\.palette\.selected\] row=3' 8
kv ret '\[quil\.new\.ok\]' 8
kv esc '\[quil\.palette\.action\] kind=esc clear=1' 8

# 26 lines: "a⏎" x 26 (52 bytes, 27 display lines)
for i in $(seq 1 26); do
  kv a "\\[quil\\.text\\.append\\] len=$((2*i-1))" 6
  kv ret "\\[quil\\.text\\.enter\\]" 6
done
sleep 1
dump "$D/bottom.ppm"
B_TOP=$(grep -oE '\[quil\.view\] top=[0-9]+' "$L" | tail -1 | grep -oE '[0-9]+$')

# Home → document start → view top 0
kv home '\[quil\.view\] top=0 cursor=0' 10
sleep 1
dump "$D/top.ppm"

# Down x12 → middle view (cursor line 12, still inside window until 22;
# keep pressing to 24 to force scroll)
for i in $(seq 1 24); do k down; sleep 0.3; done
sleep 2
dump "$D/mid.ppm"
M_TOP=$(grep -oE '\[quil\.view\] top=[0-9]+' "$L" | tail -1 | grep -oE '[0-9]+$')

FAILED=0
r() { echo "ROW $1 $2"; [[ "$2" == PASS* ]] || FAILED=1; }
[[ -n "${B_TOP:-}" && "${B_TOP}" -ge 4 ]] && r bottom_view_scrolled "PASS top=$B_TOP" || r bottom_view_scrolled "FAIL top=${B_TOP:-none}"
grep -q '\[quil\.view\] top=0 cursor=0' "$L" && r top_view_after_home PASS || r top_view_after_home FAIL
[[ -n "${M_TOP:-}" && "$M_TOP" -ge 1 && "$M_TOP" -lt "${B_TOP:-99}" ]] && r middle_view "PASS top=$M_TOP" || r middle_view "FAIL top=${M_TOP:-none}"
grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT' "$L" && r fault_free FAIL || r fault_free PASS

python3 - "$D/top.ppm" "$D/mid.ppm" "$D/bottom.ppm" > "$D/pix.txt" <<'PY'
import sys
def region(fn):
    with open(fn,'rb') as f: data=f.read()
    parts=data.split(b'\n',3)
    w,h=map(int,parts[1].split()); px=parts[3]
    out=bytearray()
    for y in range(56,min(720,h)):
        i=(y*w+1072)*3
        out+=px[i:i+200*3]
    return bytes(out)
a,b,c=[region(f) for f in sys.argv[1:4]]
print(f"ROW dumps_distinct {'PASS' if a!=b and b!=c and a!=c else 'FAIL'}")
PY
cat "$D/pix.txt"
grep -q " FAIL" "$D/pix.txt" && FAILED=1
if [[ "$FAILED" == "0" ]]; then echo "[quil.viewport.gate.result] PASS"; else echo "[quil.viewport.gate.result] FAIL"; exit 1; fi
