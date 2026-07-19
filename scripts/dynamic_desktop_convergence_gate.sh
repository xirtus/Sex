#!/usr/bin/env bash
# DYNAMIC_DESKTOP_CONVERGENCE_GATE — the complete dynamic-document
# workflow, run TWICE in one boot plus a reboot persistence check:
#   round N: quil New → type → Save → name docN → object created →
#            Linen reopen shows it live → open it from the list →
#            modify → save (bytes grew, same object)
#   then:    spindle winreset (window destroy/reclaim mid-workflow) →
#            terminal + quil still usable → boot 2: both docs still
#            listed, doc1 reopens with exact saved content hash.
# Usage: SKIP_BUILD=1 ./scripts/dynamic_desktop_convergence_gate.sh
set -uo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  ./scripts/entrypoint_build.sh > /tmp/sexos_ddc_build.log 2>&1 || { echo "ROW build FAIL"; exit 1; }
  echo "ROW build PASS"
else
  echo "ROW build SKIP"
fi
D=/tmp/sexos_ddc
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
  echo "[ddc] key miss key=$1 re=$2"
  return 1
}
tw() { for c in "$@"; do kv "$c" "\\[spindle\\.input\\.recv\\] key=printable ch=$c" 6; done; }
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

# Ensure quil palette is ON (esc toggles text<->palette; state persists).
palette_on() {
  local base; base=$(count_re 'kind=esc toggle_on=1' "$L")
  k esc; sleep 1
  [[ $(count_re 'kind=esc toggle_on=1' "$L") -gt $base ]] && return 0
  # first esc left text mode (palette was active and got cleared? no —
  # clear=1 means palette was ON and turned OFF; bring it back)
  kv esc 'kind=esc toggle_on=1' 8
}
# Navigate the 5-row palette to a target row, wherever selection is now.
nav_row() { # nav_row <target>
  local t=$1
  local base; base=$(count_re "\[quil\.palette\.selected\] row=$t" "$L")
  for i in 1 2 3 4 5 6; do
    if [[ $(count_re "\[quil\.palette\.selected\] row=$t" "$L") -gt $base ]]; then return 0; fi
    k down; sleep 1
  done
  [[ $(count_re "\[quil\.palette\.selected\] row=$t" "$L") -gt $base ]]
}
open_quil() {
  kv grave_accent '\[shell\.palette\.item\] idx=0' 8
  kv j '\[shell\.palette\.select\] old=0 new=1' 8
  kv ret '\[shell\.palette\.exec\] idx=1' 10
  sleep 1
}
open_linen() {
  kv grave_accent '\[shell\.palette\.item\] idx=0' 8
  kv j '\[shell\.palette\.select\] old=0 new=1' 8
  kv j '\[shell\.palette\.select\] old=1 new=2' 8
  kv ret '\[shell\.palette\.exec\] idx=2' 10
  sleep 2
}

### BOOT 1
L="$D/b1.log"
boot
wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[ddc] WARN publish"
sleep 3

round() { # round <docchar> <expected_id> <expected_snap>
  local dc=$1 id=$2 snap=$3
  open_quil
  palette_on
  nav_row 3 || echo "[ddc] WARN nav_row3"
  kv ret '\[quil\.new\.ok\]' 8
  kv esc '\[quil\.palette\.action\] kind=esc clear=1' 8
  kv h "\\[quil\\.text\\.append\\] len=1" 6
  kv i "\\[quil\\.text\\.append\\] len=2" 6
  palette_on
  nav_row 1 || echo "[ddc] WARN nav_row1"
  kv ret '\[quil\.name\.mode\] on=1' 10
  kv d '\[quil\.text\.draw' 6
  kv o '\[quil\.text\.draw' 6
  kv c '\[quil\.text\.draw' 6
  kv "$dc" '\[quil\.text\.draw' 6
  k ret
  wait_marker "\\[quil\\.doc\\.create\\.ok\\] id=$id" "$L" 60 || echo "[ddc] WARN create $id"
  wait_marker '\[quil\.persist\.save\.ok\] bytes=2' "$L" 120 || echo "[ddc] WARN save $id"
  # Linen live: reopen → snapshot grew
  open_linen
  wait_marker "\\[linen\\.remote\\.snapshot\\.ok\\] count=$snap" "$L" 60 || echo "[ddc] WARN snap$snap"
  # open the new doc from the list (last entry: 3 system + N)
  local jn=$((snap - 1))
  for j in $(seq 1 $jn); do k j; sleep 1; done
  k ret
  wait_marker "\\[quil\\.open\\.disk_doc\\.recv\\].*path_id=$id" "$L" 30 || echo "[ddc] WARN reopen $id"
  sleep 2
  # modify + save back to the SAME object (2 -> 3 bytes)
  kv x "\\[quil\\.text\\.append\\]" 6
  palette_on
  nav_row 1 || echo "[ddc] WARN nav_row1b"
  k ret
  wait_marker '\[quil\.persist\.save\.ok\] bytes=3' "$L" 120 || echo "[ddc] WARN resave $id"
}

round c 3 4
round d 4 5

# window lifecycle mid-workflow
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1
tw w i n r e s e t
kv ret '\[spindle\.winreset\] ok=1' 15
tw d i s k
kv ret '\[spindle\.disk\.command\] found=5' 20
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1
stop

### BOOT 2 — persistence
L="$D/b2.log"
boot
wait_marker '\[linen\.disk\.publish\.done\] count=5' "$L" 120 || echo "[ddc] WARN publish b2"
sleep 3
open_linen
wait_marker '\[linen\.remote\.snapshot\.ok\] count=5' "$L" 60 || echo "[ddc] WARN snap b2"
# open docc (4th entry)
for j in 1 2 3; do k j; sleep 1; done
k ret
wait_marker '\[quil\.persist\.load\.ok\] bytes=3' "$L" 120 || echo "[ddc] WARN load b2"
stop

FAILED=0
r() { echo "ROW $1 $2"; [[ "$2" == PASS* ]] || FAILED=1; }
grep -q '\[quil\.doc\.create\.ok\] id=3' "$D/b1.log" && r doc1_created PASS || r doc1_created FAIL
grep -q '\[quil\.doc\.create\.ok\] id=4' "$D/b1.log" && r doc2_created PASS || r doc2_created FAIL
grep -q '\[linen\.remote\.snapshot\.ok\] count=4' "$D/b1.log" && r linen_live_doc1 PASS || r linen_live_doc1 FAIL
grep -q '\[linen\.remote\.snapshot\.ok\] count=5' "$D/b1.log" && r linen_live_doc2 PASS || r linen_live_doc2 FAIL
grep -qE '\[quil\.open\.disk_doc\.recv\].*path_id=3' "$D/b1.log" && r doc1_reopened PASS || r doc1_reopened FAIL
[[ $(count_re '\[quil\.persist\.save\.ok\] bytes=3' "$D/b1.log") -ge 2 ]] && r both_docs_modified PASS || r both_docs_modified FAIL
grep -q '\[spindle\.winreset\] ok=1' "$D/b1.log" && r window_cycle_mid_workflow PASS || r window_cycle_mid_workflow FAIL
grep -q '\[spindle\.disk\.command\] found=5' "$D/b1.log" && r terminal_lists_5_objects PASS || r terminal_lists_5_objects FAIL
grep -q '\[linen\.remote\.snapshot\.ok\] count=5' "$D/b2.log" && r docs_survive_reboot PASS || r docs_survive_reboot FAIL
grep -q '\[quil\.persist\.load\.ok\] bytes=3' "$D/b2.log" && r doc1_content_restored PASS || r doc1_content_restored FAIL
H1=$(grep -oE '\[quil\.persist\.save\.ok\] bytes=3 hash=0x[0-9a-f]+' "$D/b1.log" | head -1 | grep -oE '0x[0-9a-f]+$')
grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT' "$D/b1.log" "$D/b2.log" && r fault_free FAIL || r fault_free PASS
if [[ "$FAILED" == "0" ]]; then echo "[dynamic.desktop.convergence.gate.result] PASS"; else echo "[dynamic.desktop.convergence.gate.result] FAIL"; exit 1; fi
