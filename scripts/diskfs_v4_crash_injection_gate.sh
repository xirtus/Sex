#!/usr/bin/env bash
# DISKFS_V4_CRASH_INJECTION_GATE — Lane 2 (crash-aware metadata publication
# and recovery), first slice.
#
# Every prior DISKFS_V4 reboot test killed QEMU cleanly BETWEEN operations
# (after a save fully completed). That proves nothing about the crash-safety
# ORDERING documented in DISKFS_V4_GROWTH_V1.md (grow: content -> indirect
# -> manifest) — that ordering exists specifically to make a crash DURING an
# operation safe, and until now it was never tested mid-operation.
#
# Uses two deterministic log markers added to handle_diskfs_write
# (servers/sexfiles/src/vfs.rs) for exactly this purpose, rather than
# inferring the crash window from NVMe LBA/timing guesses:
#   [sexfiles.diskfs.v4.crash_point.extent_committed] — new extent is on
#     disk and indirect-referenced; manifest NOT yet updated.
#   [sexfiles.diskfs.v4.crash_point.manifest_committed] — manifest now
#     publishes the new size.
#
# Two crash points are tested:
#   A. Kill right after extent_committed, before manifest_committed.
#      Required: reboot resolves to the OLD version (empty, size=0)
#      exactly — no partial growth visible, no duplicate/overlapping
#      allocation, and a subsequent normal grow still succeeds.
#   B. Kill right after manifest_committed.
#      Required: reboot resolves to the COMPLETE NEW version exactly.
#
# Together: recovery exposes either the complete old version or the
# complete new version, never a mixture.
set -uo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  ./scripts/entrypoint_build.sh > /tmp/sexos_ci_build.log 2>&1 || { echo "ROW build FAIL"; exit 1; }
  echo "ROW build PASS"
else
  echo "ROW build SKIP"
fi

D=/tmp/sexos_crash_inj
mkdir -p "$D"

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
wait_marker() { local d=$((SECONDS+$3)); while ((SECONDS<d)); do grep -qE "$1" "$2" 2>/dev/null && return 0; sleep 0.5; done; return 1; }
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
  echo "[cig] key miss key=$1 re=$2"
  return 1
}
tw() { for c in "$@"; do kv "$c" "\\[spindle\\.input\\.recv\\] key=printable ch=$c" 15; done; }
sp() { kv spc '\[spindle\.input\.recv\] key=printable ch= ' 15 || k spc; }
num() {
  local n="$1" d
  for ((i=0; i<${#n}; i++)); do
    d="${n:$i:1}"
    kv "$d" "\\[spindle\\.input\\.recv\\] key=printable ch=$d" 15
  done
}
sel3() { kv 3 '\[spindle\.input\.recv\] key=printable ch=3' 15; }

FAILED=0
r() { echo "ROW $1 $2"; [[ "$2" == PASS* ]] || FAILED=1; }

# Safety net: whatever else goes wrong, never leave a QEMU instance running
# past this script's own exit.
trap 'pkill -9 -f "qemu-system-x86_64.*${D}/nvme_" 2>/dev/null' EXIT

boot() {
  rm -f "$D/q.sock"; : > "$L"
  qemu-system-x86_64 -M q35 -m 512M -cdrom sexos-v1.0.0.iso \
    -drive "if=none,id=nvm,file=$NVME,format=raw" \
    -device "nvme,serial=sexos01,drive=nvm" \
    -serial "file:$L" -qmp "unix:$D/q.sock,server=on,wait=off" \
    -display none -no-reboot -no-shutdown &
  QPID=$!
}

run_crash_point() {
  # $1 = crash point name (A or B), $2 = marker regex to kill on,
  # $3 = busy-wait budget in seconds (A's marker fires early in the write
  # loop; B's requires the full ~263-call loop plus the truncate call to
  # complete first -- observed rate ~11 bytes/sec of real disk I/O means
  # a 4200-byte fill needs ~380s alone, so B needs a much larger budget
  # than A or the injection silently lands mid-loop instead of at the
  # intended post-truncate commit).
  local name="$1" marker="$2" budget="${3:-90}"
  NVME="$D/nvme_${name}.img"; rm -f "$NVME"
  dd if=/dev/zero of="$NVME" bs=512 count=2048 2>/dev/null

  # ── Boot 1: create doc, trigger grow, kill on the marker ─────────────────
  L="$D/${name}_b1.log"
  boot
  wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[cig] WARN publish ${name}_b1"
  sleep 3
  kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1
  tw m k d o c; sp; tw c r a s h d o c
  kv ret '\[spindle\.mkdoc\] id=3 ok=1' 20

  # Fire filldoc's keystrokes in the background; race a tight busy-wait
  # for the crash-point marker in parallel.
  (
    tw f i l l d o c; sp; sel3; sp; num 4200
    k ret
  ) &
  TYPE_PID=$!

  local deadline=$((SECONDS+budget)) killed=0
  while ((SECONDS<deadline)); do
    if grep -qE "$marker" "$L" 2>/dev/null; then
      killed=1
      break
    fi
  done
  # Unconditional: a timeout must not leave QEMU running unkilled -- that's
  # an orphaned process, not evidence of anything about recovery.
  kill -9 $QPID 2>/dev/null
  kill -9 $TYPE_PID 2>/dev/null
  wait $QPID 2>/dev/null
  if [[ "$killed" == "1" ]]; then
    r "${name}_crash_injected" PASS
  else
    r "${name}_crash_injected" "FAIL reason=marker_never_seen"
  fi

  # ── Boot 2: verify recovery ───────────────────────────────────────────────
  L="$D/${name}_b2.log"
  boot
  wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[cig] WARN publish ${name}_b2"
  sleep 3
  grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT' "$L" && r "${name}_fault_free_boot" FAIL || r "${name}_fault_free_boot" PASS
  kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1
  tw c a t d o c; sp; sel3
  kv ret '\[spindle\.catdoc\] id=3 size=' 600
  echo "$name" > "$D/${name}_result_marker"
}

# ── Crash point A: kill after extent_committed, before manifest_committed.
# Required outcome: object still shows size=0 (the pre-grow, OLD version) —
# the grow never became visible, not even partially.
run_crash_point A '\[sexfiles\.diskfs\.v4\.crash_point\.extent_committed\] slot=3' 90
A_MANIFEST_BEFORE_KILL=$(count_re '\[sexfiles\.diskfs\.v4\.crash_point\.manifest_committed\] slot=3' "$D/A_b1.log")
if [[ "$A_MANIFEST_BEFORE_KILL" == "0" ]]; then
  r A_landed_before_manifest_commit PASS
else
  r A_landed_before_manifest_commit "FAIL manifest_already_committed=${A_MANIFEST_BEFORE_KILL}"
fi
check_old_version() { # $1 = observed size; shared by the real assertion and its negative control
  [[ "$1" == "0" ]] && echo "PASS" || echo "FAIL size=$1"
}
A_SIZE_AFTER=$(grep -oE '\[spindle\.catdoc\] id=3 size=[0-9]+' "$D/A_b2.log" | tail -1 | grep -oE '[0-9]+$')
r A_old_version_exact "$(check_old_version "${A_SIZE_AFTER:-none}")"
# No duplicate/overlapping allocation: the layout self-check runs at every
# v4_ensure() and logs .fail on any overlap — confirm it never fired.
grep -qE '\[sexfiles\.diskfs\.v4\.layout\.fail\]' "$D/A_b2.log" && r A_no_layout_overlap FAIL || r A_no_layout_overlap PASS
# The abandoned extent (allocated+zeroed, indirect-referenced, but manifest
# never grew): prove it's not a leak — a normal grow on the same slot must
# succeed and reuse storage safely (whether the allocator reclaims it or
# rewrites over it is an implementation detail; what matters is no failure
# and no ever-growing unreachable allocation).
L="$D/A_b2.log"
tw f i l l d o c; sp; sel3; sp; num 400
kv ret '\[spindle\.filldoc\] id=3 bytes=400 ok=1' 60
tw c a t d o c; sp; sel3
kv ret '\[spindle\.catdoc\] id=3 size=400' 60
A_REGROW=$(grep -oE '\[spindle\.catdoc\] id=3 size=400 hash=0x[0-9a-f]+ ok=1' "$D/A_b2.log" | tail -1)
[[ -n "$A_REGROW" ]] && r A_later_grow_succeeds PASS || r A_later_grow_succeeds FAIL
kill $QPID 2>/dev/null; sleep 1; kill -9 $QPID 2>/dev/null

# ── Crash point B: kill right after manifest_committed.
# Required outcome: object shows the COMPLETE new version (size=4200,
# correct content hash), not a truncated/partial one.
# manifest_committed fires once per WRITE call (size grows 16 -> 32 -> ...
# -> 4200 as filldoc's loop progresses) -- each one is its own genuinely
# atomic, individually-correct checkpoint, not a torn state. Target the
# specific commit that publishes the FULL 4200-byte size, matching what
# "complete new version" means for this crash point.
run_crash_point B '\[sexfiles\.diskfs\.v4\.crash_point\.manifest_committed\] slot=3 size=4200' 600
B_SIZE_AFTER=$(grep -oE '\[spindle\.catdoc\] id=3 size=[0-9]+ hash=0x[0-9a-f]+ ok=1' "$D/B_b2.log" | tail -1)
if echo "$B_SIZE_AFTER" | grep -q 'size=4200.*ok=1'; then
  r B_new_version_exact PASS
else
  r B_new_version_exact "FAIL got=${B_SIZE_AFTER:-none}"
fi
grep -qE '\[sexfiles\.diskfs\.v4\.layout\.fail\]' "$D/B_b2.log" && r B_no_layout_overlap FAIL || r B_no_layout_overlap PASS
kill $QPID 2>/dev/null; sleep 1; kill -9 $QPID 2>/dev/null

# ── Negative control on the repaired harness itself: the SAME comparison
# function used for the real A_old_version_exact assertion above, run
# against a deliberately wrong size, must report FAIL — proving the check
# actually discriminates instead of passing by construction. Run against
# the real observed A_SIZE_AFTER too, confirming PASS reproduces exactly
# what the live assertion already reported.
NEG_RESULT=$(check_old_version "7")
POS_RESULT=$(check_old_version "${A_SIZE_AFTER:-none}")
if [[ "$NEG_RESULT" == FAIL* && "$POS_RESULT" == PASS* ]]; then
  r negative_control_detects_mismatch PASS
else
  r negative_control_detects_mismatch "FAIL neg=${NEG_RESULT} pos=${POS_RESULT}"
fi

echo "[cig] logs: $D/A_b1.log $D/A_b2.log $D/B_b1.log $D/B_b2.log"
if [[ "$FAILED" == "0" ]]; then echo "[diskfs.v4.crash_injection.gate.result] PASS"; else echo "[diskfs.v4.crash_injection.gate.result] FAIL"; exit 1; fi
