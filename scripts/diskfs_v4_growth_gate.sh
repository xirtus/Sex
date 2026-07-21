#!/usr/bin/env bash
# DISKFS_V4_GROWTH_GATE — variable-length object lifecycle, two boots.
#
# Sizes are deliberately modest: OP_DISKFS_WRITE/READ move 16/8 bytes per
# PDX round-trip (a real, understood protocol characteristic of this
# system, not a bug — see docs/handoff/DISKFS_V4_GROWTH_V1.md), so a
# multi-KB fill costs hundreds of real synchronous IPC+disk round-trips.
# Every size below is the SMALLEST that still proves its specific
# behavior; the largest fill (4112B) is chosen to clear the ">4KiB, spans
# multiple blocks" bar by exactly one block, not to stress-test throughput.
#
# Boot 1 (spindle terminal, via filldoc/truncdoc/catdoc debug commands):
#   mkdoc biggo -> filldoc 4112B (2 blocks) -> catdoc VERIFIED (exact
#   content+length) -> truncdoc 500B (shrink below a block boundary) ->
#   catdoc VERIFIED (no stale trailing bytes) -> truncdoc 0B (truncate to
#   zero) -> filldoc 600B (regrow after truncate-to-zero, reusing freed
#   blocks) -> catdoc VERIFIED -> rmdoc -> mkdoc reuses the freed slot ->
#   filldoc 400B -> catdoc VERIFIED.
# Boot 2 (same NVMe): disk lists the survivor -> catdoc VERIFIED with the
#   SAME hash as before reboot (exact persistence).
#
# Usage: SKIP_BUILD=1 ./scripts/diskfs_v4_growth_gate.sh
# Runs long (~20-25 min) due to the round-trip cost above — run in the
# background.
set -uo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  ./scripts/entrypoint_build.sh > /tmp/sexos_v4g_build.log 2>&1 || { echo "ROW build FAIL"; exit 1; }
  echo "ROW build PASS"
else
  echo "ROW build SKIP"
fi
D=/tmp/sexos_v4g
mkdir -p "$D"
NVME="$D/nvme.img"; rm -f "$NVME"; dd if=/dev/zero of="$NVME" bs=512 count=2048 2>/dev/null
rm -f "$D/.done" "$D/.timed_out" "$D/.activelog" "$D/watchdog.log"

# Hard-timeout watchdog. This gate legitimately runs 20-30+ minutes (real
# WRITE/READ round-trips at ~1-2s each — see the file header) so a plain
# wall-clock cap would false-positive on a healthy slow run. Two
# independent triggers instead:
#   STALL_SECS       — the ACTIVE boot's serial log hasn't grown at all
#                       (nothing happening, not even slowly) for this long.
#   HARD_TIMEOUT_SECS — absolute ceiling regardless of activity, so a
#                       "technically still growing" pathological case
#                       can't run forever either.
# Either trigger captures process tree, qemu state, and serial output
# before killing qemu, so a real hang fails visibly with evidence instead
# of the caller just waiting indefinitely.
STALL_SECS=${STALL_SECS:-180}
HARD_TIMEOUT_SECS=${HARD_TIMEOUT_SECS:-2700}
watchdog() {
  local start=$SECONDS
  local last_size=-1
  local last_growth=$SECONDS
  while true; do
    [[ -f "$D/.done" ]] && return 0
    if (( SECONDS - start >= HARD_TIMEOUT_SECS )); then
      echo "[v4g] WATCHDOG: hard timeout (${HARD_TIMEOUT_SECS}s) exceeded" >> "$D/watchdog.log"
      break
    fi
    local active_log=""
    [[ -f "$D/.activelog" ]] && active_log="$(cat "$D/.activelog" 2>/dev/null)"
    if [[ -n "$active_log" && -f "$active_log" ]]; then
      local size; size=$(stat -c '%s' "$active_log" 2>/dev/null || echo 0)
      if [[ "$size" != "$last_size" ]]; then
        last_size=$size
        last_growth=$SECONDS
      elif (( SECONDS - last_growth >= STALL_SECS )); then
        echo "[v4g] WATCHDOG: no serial log growth for ${STALL_SECS}s (stalled) log=$active_log" >> "$D/watchdog.log"
        break
      fi
    fi
    sleep 5
  done
  {
    echo "=== ps -ef ==="; ps -ef
    echo "=== qemu processes ==="; pgrep -af qemu-system-x86_64
    echo "=== active log path ==="; cat "$D/.activelog" 2>/dev/null
  } > "$D/timeout_process_state.txt" 2>&1
  [[ -f "$D/.activelog" ]] && cp "$(cat "$D/.activelog" 2>/dev/null)" "$D/timeout_serial.log" 2>/dev/null
  pkill -9 -f "qemu-system-x86_64.*${NVME}" 2>/dev/null
  touch "$D/.timed_out"
}
watchdog &
WATCHDOG_PID=$!

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
  echo "[v4g] key miss key=$1 re=$2"
  return 1
}
tw() { # type each char, verified by spindle printable echo
  # 15s (not 6s): confirmed root cause was v4_zero_tail (servers/sexfiles/
  # src/vfs.rs) zeroing a truncated object's tail 16 bytes at a time through
  # the full disk round-trip path — a preceding truncdoc could take long
  # enough to stall keystroke-echo processing that the next character's echo
  # missed a 6s window even though it landed fine, and kv()'s blind
  # resend-on-timeout then double-sent it, garbling the in-flight command
  # line (observed: "catdoc 3" -> "doc 3" after a resent 'c','a','t'). Fixed
  # at the source (v4_zero_tail now zeros at 512-byte sector granularity,
  # ~32x fewer round-trips) — this wider budget is kept as defensive margin,
  # not a workaround for an open bug.
  for c in "$@"; do
    kv "$c" "\\[spindle\\.input\\.recv\\] key=printable ch=$c" 15
  done
}
sp() { kv spc '\[spindle\.input\.recv\] key=printable ch= ' 15 || k spc; }
# Type a decimal number one digit at a time (digit qcodes == the digit itself).
num() {
  local n="$1" d
  for ((i=0; i<${#n}; i++)); do
    d="${n:$i:1}"
    kv "$d" "\\[spindle\\.input\\.recv\\] key=printable ch=$d" 15
  done
}
sel3() { kv 3 '\[spindle\.input\.recv\] key=printable ch=3' 15; }

boot() {
  rm -f "$D/q.sock"; : > "$L"
  echo "$L" > "$D/.activelog"
  qemu-system-x86_64 -M q35 -m 512M -cdrom sexos-v1.0.0.iso \
    -drive "if=none,id=nvm,file=$NVME,format=raw" \
    -device "nvme,serial=sexos01,drive=nvm" \
    -serial "file:$L" -qmp "unix:$D/q.sock,server=on,wait=off" \
    -display none -no-reboot -no-shutdown &
  QPID=$!
}
stop() { kill $QPID 2>/dev/null; sleep 1; kill -9 $QPID 2>/dev/null; }
trap 'stop; kill "${WATCHDOG_PID:-}" 2>/dev/null' EXIT

### BOOT 1
L="$D/b1.log"
boot
wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[v4g] WARN publish b1"
sleep 3
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1

# mkdoc biggo -> expect slot 3 (first free non-system slot on a blank disk)
tw m k d o c; sp; tw b i g g o
kv ret '\[spindle\.mkdoc\] id=3 ok=1' 20

# filldoc 3 4112 -> grows across 2 blocks (4096B each), clears >4KiB.
tw f i l l d o c; sp; sel3; sp; num 4112
kv ret '\[spindle\.filldoc\] id=3 bytes=4112 ok=1' 900

# catdoc 3 -> exact content + length verified byte-by-byte
tw c a t d o c; sp; sel3
kv ret '\[spindle\.catdoc\] id=3 size=4112' 900

# truncdoc 3 500 -> shrink below a block boundary
tw t r u n c d o c; sp; sel3; sp; num 500
kv ret '\[spindle\.truncdoc\] id=3 new_len=500 ok=1' 60

# no stale trailing bytes: the kept prefix must still match the pattern
tw c a t d o c; sp; sel3
kv ret '\[spindle\.catdoc\] id=3 size=500' 400

# truncdoc 3 0 -> truncate to zero
tw t r u n c d o c; sp; sel3; sp; num 0
kv ret '\[spindle\.truncdoc\] id=3 new_len=0 ok=1' 60

# regrow after truncate-to-zero (reuses reclaimed blocks)
tw f i l l d o c; sp; sel3; sp; num 600
kv ret '\[spindle\.filldoc\] id=3 bytes=600 ok=1' 300

tw c a t d o c; sp; sel3
kv ret '\[spindle\.catdoc\] id=3 size=600' 400

# delete, then reuse the freed slot
tw r m d o c; sp; sel3
kv ret '\[spindle\.rmdoc\] id=3 ok=1' 20

tw m k d o c; sp; tw s u r v i v e
kv ret '\[spindle\.mkdoc\] id=3 ok=1' 20

tw f i l l d o c; sp; sel3; sp; num 400
kv ret '\[spindle\.filldoc\] id=3 bytes=400 ok=1' 200

tw c a t d o c; sp; sel3
kv ret '\[spindle\.catdoc\] id=3 size=400' 400
sleep 1
stop

### BOOT 2 — same NVMe: reboot persistence
L="$D/b2.log"
boot
wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[v4g] WARN publish b2"
sleep 3
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1

tw d i s k
kv ret '\[spindle\.disk\.command\] found=4' 20

tw c a t d o c; sp; sel3
kv ret '\[spindle\.catdoc\] id=3 size=400' 400
sleep 1
stop

touch "$D/.done"
kill "${WATCHDOG_PID:-}" 2>/dev/null

### Rows
FAILED=0
r() { echo "ROW $1 $2"; [[ "$2" == FAIL* ]] && FAILED=1 || true; }

if [[ -f "$D/.timed_out" ]]; then
  r watchdog_timeout "FAIL see $D/watchdog.log $D/timeout_process_state.txt $D/timeout_serial.log"
fi

grep -q '\[spindle\.mkdoc\] id=3 ok=1' "$D/b1.log" && r create_empty_object PASS || r create_empty_object FAIL
grep -q '\[spindle\.filldoc\] id=3 bytes=4112 ok=1' "$D/b1.log" && r grow_multi_block PASS || r grow_multi_block FAIL
grep -q '\[spindle\.catdoc\] id=3 size=4112 hash=' "$D/b1.log" && r exact_content_4112 PASS || r exact_content_4112 FAIL
grep -q '\[spindle\.truncdoc\] id=3 new_len=500 ok=1' "$D/b1.log" && r shrink_below_block PASS || r shrink_below_block FAIL
grep -q '\[spindle\.catdoc\] id=3 size=500 hash=' "$D/b1.log" && r no_stale_trailing_bytes PASS || r no_stale_trailing_bytes FAIL
grep -q '\[spindle\.truncdoc\] id=3 new_len=0 ok=1' "$D/b1.log" && r truncate_to_zero PASS || r truncate_to_zero FAIL
grep -q '\[spindle\.filldoc\] id=3 bytes=600 ok=1' "$D/b1.log" && r regrow_after_zero PASS || r regrow_after_zero FAIL
grep -q '\[spindle\.catdoc\] id=3 size=600 hash=' "$D/b1.log" && r regrow_content_exact PASS || r regrow_content_exact FAIL
grep -q '\[spindle\.rmdoc\] id=3 ok=1' "$D/b1.log" && r delete_reclaim PASS || r delete_reclaim FAIL
[[ $(count_re '\[spindle\.mkdoc\] id=3 ok=1' "$D/b1.log") -ge 2 ]] && r slot_reuse PASS || r slot_reuse FAIL
grep -q '\[spindle\.catdoc\] id=3 size=400 hash=' "$D/b1.log" && r reused_slot_content_exact PASS || r reused_slot_content_exact FAIL

HASH_B1=$(grep -oE '\[spindle\.catdoc\] id=3 size=400 hash=0x[0-9a-f]+' "$D/b1.log" | tail -1 | grep -oE '0x[0-9a-f]+$')
HASH_B2=$(grep -oE '\[spindle\.catdoc\] id=3 size=400 hash=0x[0-9a-f]+' "$D/b2.log" | tail -1 | grep -oE '0x[0-9a-f]+$')
grep -q '\[spindle\.disk\.command\] found=4' "$D/b2.log" && r reboot_object_listed PASS || r reboot_object_listed FAIL
if [[ -n "${HASH_B1:-}" && "${HASH_B1:-x}" == "${HASH_B2:-y}" ]]; then
  r reboot_survival_exact_hash "PASS hash=${HASH_B1}"
else
  r reboot_survival_exact_hash "FAIL b1=${HASH_B1:-none} b2=${HASH_B2:-none}"
fi
grep -qE 'MISMATCH' "$D/b1.log" "$D/b2.log" && r no_content_mismatch FAIL || r no_content_mismatch PASS
grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT' "$D/b1.log" "$D/b2.log" && r fault_free FAIL || r fault_free PASS

if [[ "$FAILED" == "0" ]]; then echo "[diskfs.v4.growth.gate.result] PASS"; else echo "[diskfs.v4.growth.gate.result] FAIL"; exit 1; fi
