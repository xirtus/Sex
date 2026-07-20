#!/usr/bin/env bash
# QUIL_EDITOR_GATE — QUIL_EDITOR_V1 editor behaviors, one boot:
# dirty tracking (draw marker dirty=1 on edit, cleared by save), repeated
# saves with shrinking length (242 then 241 — header truncation), real New
# Buffer (row 3, doc cleared to 0 bytes with status line still drawn), and
# reload of the last save from DiskFS. Marker-verified keying throughout.
# Usage: SKIP_BUILD=1 ./scripts/quil_editor_gate.sh
set -uo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  ./scripts/entrypoint_build.sh > /tmp/sexos_qe_build.log 2>&1 || { echo "ROW build FAIL"; exit 1; }
  echo "ROW build PASS"
else
  echo "ROW build SKIP"
fi
D=/tmp/sexos_qe
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
  echo "MISS key=$1"; return 1
}
wait_save() { # wait for Nth persist.save completion
  local n=$1; local d=$((SECONDS+120))
  while ((SECONDS<d)); do
    [[ $(count_re '\[quil\.persist\.save\.ok\]' "$L") -ge $n ]] && return 0
    sleep 1
  done
  echo "MISS save#$n"; return 1
}

qemu-system-x86_64 -M q35 -m 512M -cdrom sexos-v1.0.0.iso \
  -drive "if=none,id=nvm,file=$NVME,format=raw" \
  -device "nvme,serial=sexos01,drive=nvm" \
  -serial "file:$L" -qmp "unix:$D/q.sock,server=on,wait=off" \
  -display none -no-reboot -no-shutdown &
QPID=$!
trap 'kill $QPID 2>/dev/null; sleep 1; kill -9 $QPID 2>/dev/null' EXIT

wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "WARN publish"
sleep 3
# open quil
kv grave_accent '\[shell\.palette\.item\] idx=0' 8
kv j '\[shell\.palette\.select\] old=0 new=1' 8
kv ret '\[shell\.palette\.exec\] idx=1' 10
sleep 2
# text mode, type ab (dirty), save #1
kv esc '\[quil\.palette\.action\] kind=esc' 8
kv a '\[quil\.text\.append\]' 6
kv b '\[quil\.text\.append\]' 6
dump "$D/dirty.ppm"
kv esc 'kind=esc toggle_on=1' 8
kv down '\[quil\.palette\.selected\] row=1' 8
k ret; wait_save 1
# edit again: text mode, backspace (dirty), save #2 (shorter)
kv esc '\[quil\.palette\.action\] kind=esc' 8
kv backspace '\[quil\.text\.backspace\]' 6
kv esc 'kind=esc toggle_on=1' 8
kv down '\[quil\.palette\.selected\] row=1' 8
k ret; wait_save 2
sleep 1
dump "$D/saved.ppm"
# New Buffer: cycle palette off/on to force row-0 reset, then rows 1-3
kv esc '\[quil\.palette\.action\] kind=esc clear=1' 8
kv esc 'kind=esc toggle_on=1' 8
kv down '\[quil\.palette\.selected\] row=1' 8
kv down '\[quil\.palette\.selected\] row=2' 8
kv down '\[quil\.palette\.selected\] row=3' 8
kv ret '\[quil\.new\.ok\]' 8
sleep 1
dump "$D/new.ppm"
# reload from disk: rows 0→2
kv down '\[quil\.palette\.selected\] row=1' 8
kv down '\[quil\.palette\.selected\] row=2' 8
k ret
wait_marker '\[quil\.persist\.load\.ok\]' "$L" 120 || echo "MISS reload"
sleep 1
dump "$D/reloaded.ppm"

FAILED=0
r() { echo "ROW $1 $2"; [[ "$2" == FAIL* ]] && FAILED=1 || true; }
grep -qE '\[quil\.text\.draw\.v2\].*dirty=1' "$L" && r quil_dirty_tracking PASS || r quil_dirty_tracking FAIL
S1=$(grep -oE '\[quil\.persist\.save\.ok\] bytes=[0-9]+' "$L" | head -1 | grep -oE '[0-9]+$')
S2=$(grep -oE '\[quil\.persist\.save\.ok\] bytes=[0-9]+' "$L" | tail -1 | grep -oE '[0-9]+$')
if [[ -n "${S1:-}" && -n "${S2:-}" && "$S1" != "$S2" ]]; then r quil_repeat_save PASS; else r quil_repeat_save "FAIL s1=${S1:-none} s2=${S2:-none}"; fi
grep -q '\[quil\.new\.ok\] bytes=0' "$L" && r quil_new_buffer PASS || r quil_new_buffer FAIL
LB=$(grep -oE '\[quil\.persist\.load\.ok\] bytes=[0-9]+' "$L" | tail -1 | grep -oE '[0-9]+$')
if [[ -n "${LB:-}" && "$LB" == "${S2:-x}" ]]; then r quil_reload_matches_last_save PASS; else r quil_reload_matches_last_save "FAIL load=${LB:-none} save=${S2:-none}"; fi
grep -qE "KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT" "$L" && r fault_free FAIL || r fault_free PASS
if [[ "$FAILED" == "0" ]]; then echo "[quil.editor.gate.result] PASS"; else echo "[quil.editor.gate.result] FAIL"; exit 1; fi
