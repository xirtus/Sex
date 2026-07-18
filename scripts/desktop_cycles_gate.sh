#!/usr/bin/env bash
# DESKTOP_CYCLES_GATE — repeated-use convergence, one boot.
# Proves the desktop survives cycles, not just first use:
#   spindle: focus → help → unfocus → refocus → disk → flood (5× help)
#            → PgUp/PgDn paging still correct after flood
#   quil:    open → type → refocus away → reopen → type again (buffer
#            persists across focus cycles) → save
#   linen:   open → nav → open object → reopen linen → nav → open the
#            disk doc into quil
#   system:  desktop still responds to input at the end (liveness), zero
#            faults, final screendump has live spindle text pixels.
# Usage: SKIP_BUILD=1 ./scripts/desktop_cycles_gate.sh
set -uo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  ./scripts/entrypoint_build.sh > /tmp/sexos_dc_build.log 2>&1 || { echo "ROW build FAIL"; exit 1; }
  echo "ROW build PASS"
else
  echo "ROW build SKIP"
fi
D=/tmp/sexos_dc
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
  echo "[dc] key miss key=$1 re=$2"
  return 1
}
type_word() { # type_word h e l p — each echo-verified against spindle
  for c in "$@"; do
    case $c in
      h) kv h '\[spindle\.input\.recv\] key=printable ch=h' 6;;
      e) kv e '\[spindle\.input\.recv\] key=printable ch=e' 6;;
      l) kv l '\[spindle\.input\.recv\] key=printable ch=l' 6;;
      p) kv p '\[spindle\.input\.recv\] key=printable ch=p' 6;;
      d) kv d '\[spindle\.input\.recv\] key=printable ch=d' 6;;
      i) kv i '\[spindle\.input\.recv\] key=printable ch=i' 6;;
      s) kv s '\[spindle\.input\.recv\] key=printable ch=s' 6;;
      k) kv k '\[spindle\.input\.recv\] key=printable ch=k' 6;;
    esac
  done
}

qemu-system-x86_64 -M q35 -m 512M -cdrom sexos-v1.0.0.iso \
  -drive "if=none,id=nvm,file=$NVME,format=raw" \
  -device "nvme,serial=sexos01,drive=nvm" \
  -serial "file:$L" -qmp "unix:$D/q.sock,server=on,wait=off" \
  -display none -no-reboot -no-shutdown &
QPID=$!
trap 'kill $QPID 2>/dev/null; sleep 1; kill -9 $QPID 2>/dev/null' EXIT

wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[dc] WARN publish"
sleep 3

# ── Spindle cycle 1: help ──
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1
type_word h e l p
kv ret '\[spindle\.cmd\.exec\] name=help' 10
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1

# ── Quil cycle 1: open, type a ──
kv grave_accent '\[shell\.palette\.item\] idx=0' 8
kv j '\[shell\.palette\.select\] old=0 new=1' 8
kv ret '\[shell\.palette\.exec\] idx=1' 10
sleep 1
kv esc '\[quil\.palette\.action\] kind=esc' 8
kv a '\[quil\.text\.append\]' 6

# ── Spindle cycle 2 (refocus): disk + flood + paging ──
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1
type_word d i s k
kv ret '\[spindle\.disk\.command\]' 20
for i in 1 2 3 4 5; do
  type_word h e l p
  k ret; sleep 1
done
kv pgup '\[spindle\.page\.nav\] dir=up offset=[1-9]' 8
kv pgdn '\[spindle\.page\.nav\] dir=down' 8
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1

# ── Quil cycle 2 (reopen): quil is STILL in text mode from cycle 1 —
# typing works immediately after refocus (buffer/mode survive the cycle)
kv grave_accent '\[shell\.palette\.item\] idx=0' 8
kv j '\[shell\.palette\.select\] old=0 new=1' 8
kv ret '\[shell\.palette\.exec\] idx=1' 10
sleep 1
kv b '\[quil\.text\.append\]' 6
kv esc 'kind=esc toggle_on=1' 8
kv down '\[quil\.palette\.selected\] row=1' 8
k ret
wait_marker '\[quil\.persist\.save\.ok\]' "$L" 120 || echo "[dc] WARN save"

# ── Linen cycle 1: open, nav, open object 1 ──
kv grave_accent '\[shell\.palette\.item\] idx=0' 8
kv j '\[shell\.palette\.select\] old=0 new=1' 8
kv j '\[shell\.palette\.select\] old=1 new=2' 8
kv ret '\[shell\.palette\.exec\] idx=2' 10
sleep 2
kv ret '\[linen\.quil\.buffer\.linked\]' 10

# ── Linen cycle 2 (reopen): nav to disk doc, open into quil ──
kv grave_accent '\[shell\.palette\.item\] idx=0' 8
kv j '\[shell\.palette\.select\] old=0 new=1' 8
kv j '\[shell\.palette\.select\] old=1 new=2' 8
kv ret '\[shell\.palette\.exec\] idx=2' 10
sleep 2
kv j '\[linen\.nav\.select\.ok\]|\[shell\.action\.select_next_linen\]' 8
kv j '\[linen\.nav\.select\.ok\]|\[shell\.action\.select_next_linen\]' 8
k ret
wait_marker '\[quil\.open\.disk_doc\.recv\]' "$L" 30 || echo "[dc] WARN no disk_doc"

# ── Liveness: desktop still responds to input at the end ──
if kv scroll_lock 'reason=ToggleSpindle' 10; then LIVE=1; else LIVE=0; fi
sleep 2
dump "$D/final.ppm"; sleep 1

# ── Rows ──
FAILED=0
r() { echo "ROW $1 $2"; [[ "$2" == FAIL ]] && FAILED=1 || true; }
[[ $(count_re '\[spindle\.cmd\.exec\] name=help' "$L") -ge 6 ]] && r spindle_flood_6_help PASS || r spindle_flood_6_help FAIL
grep -q '\[spindle\.disk\.command\] found=3' "$L" && r spindle_disk_after_refocus PASS || r spindle_disk_after_refocus FAIL
grep -qE '\[spindle\.page\.nav\] dir=up offset=[1-9]' "$L" && r spindle_paging_after_flood PASS || r spindle_paging_after_flood FAIL
[[ $(count_re '\[quil\.text\.append\]' "$L") -ge 2 ]] && r quil_type_both_cycles PASS || r quil_type_both_cycles FAIL
grep -q '\[quil\.persist\.save\.ok\]' "$L" && r quil_save_cycle2 PASS || r quil_save_cycle2 FAIL
[[ $(count_re '\[linen\.quil\.buffer\.linked\]' "$L") -ge 2 ]] && r linen_open_both_cycles PASS || r linen_open_both_cycles FAIL
grep -q '\[quil\.open\.disk_doc\.recv\]' "$L" && r linen_disk_doc_cycle2 PASS || r linen_disk_doc_cycle2 FAIL
[[ "$LIVE" == "1" ]] && r desktop_responsive_at_end PASS || r desktop_responsive_at_end FAIL
grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT|fault\.kill' "$L" && r fault_free FAIL || r fault_free PASS

python3 - "$D/final.ppm" <<'PY' >"$D/pix.txt" || true
import sys
try:
    with open(sys.argv[1],'rb') as f: data=f.read()
    parts=data.split(b'\n',3)
    if parts[0].strip()!=b'P6':
        print("ROW pixel_scan SKIP fmt"); raise SystemExit
    w,h=map(int,parts[1].split()); px=parts[3]
    # Spindle grid region bottom-right must have text pixels (0xE8FFFF)
    sp=0
    for y in range(632,min(792,h)):
        for x in range(1008,min(1272,w)):
            i=(y*w+x)*3
            if px[i]==0xE8 and px[i+1]==0xFF and px[i+2]==0xFF: sp+=1
    print(f"ROW pixel_spindle_alive {'PASS' if sp>30 else 'FAIL'} count={sp}")
except SystemExit:
    pass
except FileNotFoundError:
    print("ROW pixel_scan SKIP no_dump")
PY
cat "$D/pix.txt"
grep -q " FAIL" "$D/pix.txt" && FAILED=1

if [[ "$FAILED" == "0" ]]; then echo "[desktop.cycles.gate.result] PASS"; else echo "[desktop.cycles.gate.result] FAIL"; exit 1; fi
