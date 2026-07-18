#!/usr/bin/env bash
# QUIL_CURSOR_GATE — QUIL_TEXT_V3 mid-buffer editing, one boot.
# Sequence (in a fresh New Buffer):
#   type "abc" ⏎ "def"            → "abc\ndef"      cursor at end
#   Up (line 0, sticky col 3), Left (col 2)
#   type "x"                      → "abxc\ndef"     insert mid-line
#   Delete                        → "abx\ndef"      delete-at-cursor
#   Down (to end of "def"), Backspace ×4
#                                 → "abx"           joins across newline
#   Save → [quil.persist.save.ok] hash must equal FNV-1a("abx").
set -uo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  ./scripts/entrypoint_build.sh > /tmp/sexos_qc_build.log 2>&1 || { echo "ROW build FAIL"; exit 1; }
  echo "ROW build PASS"
else
  echo "ROW build SKIP"
fi
D=/tmp/sexos_qc
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
  echo "[qc] key miss key=$1 re=$2"
  return 1
}

qemu-system-x86_64 -M q35 -m 512M -cdrom sexos-v1.0.0.iso \
  -drive "if=none,id=nvm,file=$NVME,format=raw" \
  -device "nvme,serial=sexos01,drive=nvm" \
  -serial "file:$L" -qmp "unix:$D/q.sock,server=on,wait=off" \
  -display none -no-reboot -no-shutdown &
QPID=$!
trap 'kill $QPID 2>/dev/null; sleep 1; kill -9 $QPID 2>/dev/null' EXIT

wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[qc] WARN publish"
sleep 3
# open quil, New Buffer (palette row 3), then text mode
kv grave_accent '\[shell\.palette\.item\] idx=0' 8
kv j '\[shell\.palette\.select\] old=0 new=1' 8
kv ret '\[shell\.palette\.exec\] idx=1' 10
sleep 1
kv down '\[quil\.palette\.selected\] row=1' 8
kv down '\[quil\.palette\.selected\] row=2' 8
kv down '\[quil\.palette\.selected\] row=3' 8
kv ret '\[quil\.new\.ok\]' 8
kv esc '\[quil\.palette\.action\] kind=esc clear=1' 8
# abc ⏎ def
kv a '\[quil\.text\.append\] len=1' 6
kv b '\[quil\.text\.append\] len=2' 6
kv c '\[quil\.text\.append\] len=3' 6
kv ret '\[quil\.text\.enter\]' 6
kv d '\[quil\.text\.append\] len=5' 6
kv e '\[quil\.text\.append\] len=6' 6
kv f '\[quil\.text\.append\] len=7' 6
# Up → line 0 (sticky col 3), Left → col 2
kv up '\[quil\.cursor\.move\].*dir=up ok=1' 6
kv left '\[quil\.cursor\.move\].*dir=left ok=1' 6
# insert x mid-line → "abxc\ndef"
kv x '\[quil\.text\.append\] len=8 ch=120 at=2' 6
# Delete → removes 'c' → "abX\ndef"
kv delete '\[quil\.text\.delete\]' 6
# Down → end of "def" (goal col ≥3 clamps to len 3)
kv down '\[quil\.cursor\.move\].*dir=down ok=1' 6
# Backspace ×4 → "abX" (last one joins across the newline)
kv backspace '\[quil\.text\.backspace\] old=7 new=6' 6
kv backspace '\[quil\.text\.backspace\] old=6 new=5' 6
kv backspace '\[quil\.text\.backspace\] old=5 new=4' 6
kv backspace '\[quil\.text\.backspace\] old=4 new=3' 6
# dirty then save; hash must equal FNV-1a("abX")
kv esc 'kind=esc toggle_on=1' 8
kv up '\[quil\.palette\.selected\] row=2' 8
kv up '\[quil\.palette\.selected\] row=1' 8
k ret
wait_marker '\[quil\.persist\.save\.ok\]' "$L" 120 || echo "[qc] WARN save"
sleep 1

EXPECT=$(python3 - <<'PY'
h = 0xcbf29ce484222325
for b in b"abx":
    h ^= b
    h = (h * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
print(hex(h))
PY
)
FAILED=0
r() { echo "ROW $1 $2"; [[ "$2" == PASS* ]] || FAILED=1; }
grep -q '\[quil\.text\.append\] len=8 ch=120 at=2' "$L" && r insert_mid_line PASS || r insert_mid_line FAIL
grep -q '\[quil\.text\.delete\]' "$L" && r delete_at_cursor PASS || r delete_at_cursor FAIL
grep -qE '\[quil\.cursor\.move\].*dir=up ok=1' "$L" && r cursor_up PASS || r cursor_up FAIL
grep -qE '\[quil\.cursor\.move\].*dir=down ok=1' "$L" && r cursor_down PASS || r cursor_down FAIL
grep -q '\[quil\.text\.backspace\] old=4 new=3' "$L" && r backspace_joins_lines PASS || r backspace_joins_lines FAIL
grep -qE '\[quil\.text\.draw\.v2\].*dirty=1' "$L" && r dirty_during_edit PASS || r dirty_during_edit FAIL
if grep -q "\[quil\.persist\.save\.ok\] bytes=3 hash=$EXPECT" "$L"; then
  r exact_final_content PASS
else
  r exact_final_content "FAIL expect=$EXPECT got=$(grep -oE '\[quil\.persist\.save\.ok\][^ ]* [^ ]*' "$L" | tail -1)"
fi
grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT' "$L" && r fault_free FAIL || r fault_free PASS
if [[ "$FAILED" == "0" ]]; then echo "[quil.cursor.gate.result] PASS"; else echo "[quil.cursor.gate.result] FAIL"; exit 1; fi
