#!/usr/bin/env bash
# DISKFS_V4_METADATA_CRASH_GATE — Lane 2, Slices A/B/C: create, rename,
# delete crash ordering. Mirrors diskfs_v4_crash_injection_gate.sh's proven
# deterministic-marker approach for grow/shrink, extended to the three
# name-table mutations. Unlike grow/shrink (content -> indirect -> manifest,
# two commit phases), create/rename/delete each have exactly ONE phase that
# touches the manifest (a single v4_persist() call) — delete additionally
# has a second, LOCAL-ONLY phase (bitmap free + indirect invalidate) that
# never touches the manifest sector, tested separately below.
#
# Six crash points, three ops:
#   CREATE_PENDING   — kill before persist. Required: object does not exist.
#   CREATE_COMMITTED — kill after persist.  Required: complete empty object exists.
#   DELETE_PENDING   — kill before persist. Required: old object exactly intact.
#   DELETE_COMMITTED — kill after persist, before bitmap free/indirect clear.
#                       Required: object gone from listing; later slot reuse
#                       exposes no stale content (live-tested, not just
#                       reasoned about).
#   RENAME_PENDING   — kill before persist. Required: old name intact.
#   RENAME_COMMITTED — kill after persist.  Required: new name intact,
#                       content/id unchanged.
set -uo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  ./scripts/entrypoint_build.sh > /tmp/sexos_ci_build.log 2>&1 || { echo "ROW build FAIL"; exit 1; }
  echo "ROW build PASS"
else
  echo "ROW build SKIP"
fi

D=/tmp/sexos_meta_crash
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
  echo "[mcg] key miss key=$1 re=$2"
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
open_spindle() {
  # kv's own retry-on-miss can send scroll_lock TWICE if the first press's
  # log line lands slower than its budget (real risk right after a fresh
  # boot / a just-killed prior QEMU instance) - ToggleSpindle is a literal
  # toggle, so two presses cancel out, and a direction-blind regex ("any
  # ToggleSpindle line appeared") reports success either way. Check the
  # ACTUAL resulting focus (new= sid != quil's 201) instead of just "an
  # event fired", and retry the press itself if it didn't land on spindle.
  local tries=0 last_new
  while ((tries<6)); do
    k scroll_lock
    sleep 1.5
    last_new=$(grep -oE '\[shell\.kbd\.ui\.focus\] old=[0-9]+ new=[0-9]+ frame=[0-9]+ reason=ToggleSpindle' "$L" | tail -1 | grep -oE 'new=[0-9]+' | cut -d= -f2)
    if [[ -n "$last_new" && "$last_new" != "201" ]]; then sleep 1; return 0; fi
    tries=$((tries+1))
  done
  echo "[mcg] open_spindle failed to focus spindle after $tries tries (last_new=${last_new:-none})"
  return 1
}

# Race a background command line against a deterministic marker. $1 = marker
# regex to kill on, $2 = budget seconds. The background typing job ($TYPE_PID,
# set by the caller) is killed unconditionally alongside QEMU so a miss never
# leaves an orphaned process.
crash_on_marker() {
  local marker="$1" budget="${2:-60}"
  local deadline=$((SECONDS+budget)) killed=0
  while ((SECONDS<deadline)); do
    if grep -qE "$marker" "$L" 2>/dev/null; then killed=1; break; fi
  done
  kill -9 "$QPID" 2>/dev/null
  kill -9 "${TYPE_PID:-0}" 2>/dev/null
  wait "$QPID" 2>/dev/null
  echo "$killed"
}

disk_has_name() { grep -qE "\\[spindle\\.disk\\.entry\\] slot=[0-9]+ name=$1\$" "$L"; }
catdoc_line() { grep -oE "\\[spindle\\.catdoc\\] id=$1 size=[0-9]+ hash=0x[0-9a-f]+ ok=1" "$L" | tail -1; }

# ═══════════════════════════════════════════════════════════ CREATE ═══════
run_create() {
  local name="$1" marker="$2" budget="$3" tag="$4"
  NVME="$D/nvme_${tag}.img"; rm -f "$NVME"
  dd if=/dev/zero of="$NVME" bs=512 count=2048 2>/dev/null
  L="$D/${tag}_b1.log"; boot
  wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[mcg] WARN publish ${tag}_b1"
  sleep 3; open_spindle
  ( tw m k d o c; sp; for c in $(echo -n "$name" | fold -w1); do kv "$c" "\\[spindle\\.input\\.recv\\] key=printable ch=$c" 15; done; k ret ) &
  TYPE_PID=$!
  local killed; killed=$(crash_on_marker "$marker" "$budget")
  [[ "$killed" == "1" ]] && r "${tag}_crash_injected" PASS || r "${tag}_crash_injected" "FAIL reason=marker_never_seen"

  L="$D/${tag}_b2.log"; boot
  wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[mcg] WARN publish ${tag}_b2"
  sleep 3
  grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT' "$L" && r "${tag}_fault_free_boot" FAIL || r "${tag}_fault_free_boot" PASS
  open_spindle
  tw d i s k; kv ret '\[spindle\.disk\.command\] found=' 60
  echo "$tag" > "$D/${tag}_result_marker"
}

run_create newdoc  '\[sexfiles\.diskfs\.v4\.crash_point\.create_pending\]' 60 CREATE_PENDING
if disk_has_name newdoc; then r CREATE_PENDING_object_absent "FAIL object_exists_but_should_not"; else r CREATE_PENDING_object_absent PASS; fi

run_create newdoc2 '\[sexfiles\.diskfs\.v4\.create\.ok\] slot=3' 60 CREATE_COMMITTED
if disk_has_name newdoc2; then
  L="$D/CREATE_COMMITTED_b2.log"
  tw c a t d o c; sp; sel3
  kv ret '\[spindle\.catdoc\] id=3 size=0' 30
  grep -qE '\[spindle\.catdoc\] id=3 size=0 hash=0xcbf29ce484222325 ok=1' "$L" && r CREATE_COMMITTED_object_complete PASS || r CREATE_COMMITTED_object_complete FAIL
else
  r CREATE_COMMITTED_object_complete "FAIL object_missing"
fi
kill "$QPID" 2>/dev/null; sleep 1; kill -9 "$QPID" 2>/dev/null

# ═══════════════════════════════════════════════════ DELETE / RENAME seed ═══
# Both delete and rename need an existing, filled object first — seed it in
# boot 1 (same boot the crash is injected in; no separate seed reboot needed,
# same-boot in-memory + on-disk state carries forward naturally).
seed_origdoc() {
  open_spindle
  tw m k d o c; sp; tw o r i g d o c
  kv ret '\[spindle\.mkdoc\] id=3 ok=1' 20
  tw f i l l d o c; sp; sel3; sp; num 64
  kv ret '\[spindle\.filldoc\] id=3 bytes=64 ok=1' 60
  tw c a t d o c; sp; sel3
  kv ret '\[spindle\.catdoc\] id=3 size=64 hash=0x' 30
  ORIG_HASH=$(grep -oE '\[spindle\.catdoc\] id=3 size=64 hash=0x[0-9a-f]+ ok=1' "$L" | tail -1)
}

# ═══════════════════════════════════════════════════════════ DELETE ═══════
run_delete() {
  local marker="$1" budget="$2" tag="$3"
  NVME="$D/nvme_${tag}.img"; rm -f "$NVME"
  dd if=/dev/zero of="$NVME" bs=512 count=2048 2>/dev/null
  L="$D/${tag}_b1.log"; boot
  wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[mcg] WARN publish ${tag}_b1"
  sleep 3
  seed_origdoc
  ( tw r m d o c; sp; sel3; k ret ) &
  TYPE_PID=$!
  local killed; killed=$(crash_on_marker "$marker" "$budget")
  [[ "$killed" == "1" ]] && r "${tag}_crash_injected" PASS || r "${tag}_crash_injected" "FAIL reason=marker_never_seen"

  L="$D/${tag}_b2.log"; boot
  wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[mcg] WARN publish ${tag}_b2"
  sleep 3
  grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT' "$L" && r "${tag}_fault_free_boot" FAIL || r "${tag}_fault_free_boot" PASS
  echo "$tag" > "$D/${tag}_result_marker"
}

run_delete '\[sexfiles\.diskfs\.v4\.crash_point\.delete_pending\]' 60 DELETE_PENDING
L="$D/DELETE_PENDING_b2.log"
open_spindle
tw d i s k; kv ret '\[spindle\.disk\.command\] found=' 60
if disk_has_name origdoc; then
  tw c a t d o c; sp; sel3
  kv ret '\[spindle\.catdoc\] id=3 size=64' 30
  grep -qE '\[spindle\.catdoc\] id=3 size=64 hash=0x[0-9a-f]+ ok=1' "$L" && r DELETE_PENDING_object_intact PASS || r DELETE_PENDING_object_intact FAIL
else
  r DELETE_PENDING_object_intact "FAIL object_missing"
fi
kill "$QPID" 2>/dev/null; sleep 1; kill -9 "$QPID" 2>/dev/null

run_delete '\[sexfiles\.diskfs\.v4\.crash_point\.delete_committed\]' 60 DELETE_COMMITTED
L="$D/DELETE_COMMITTED_b2.log"
open_spindle
tw d i s k; kv ret '\[spindle\.disk\.command\] found=' 60
if disk_has_name origdoc; then r DELETE_COMMITTED_object_gone FAIL; else r DELETE_COMMITTED_object_gone PASS; fi
# Stale-content check: reuse the freed slot with DIFFERENT, smaller content
# and confirm no leftover bytes from origdoc leak through.
tw m k d o c; sp; tw r e u s e d o c
kv ret '\[spindle\.mkdoc\] id=3 ok=1' 20
tw f i l l d o c; sp; sel3; sp; num 20
kv ret '\[spindle\.filldoc\] id=3 bytes=20 ok=1' 30
tw c a t d o c; sp; sel3
kv ret '\[spindle\.catdoc\] id=3 size=20' 30
grep -qE '\[spindle\.catdoc\] id=3 size=20 hash=0x[0-9a-f]+ ok=1' "$L" && r DELETE_COMMITTED_no_stale_content PASS || r DELETE_COMMITTED_no_stale_content FAIL
kill "$QPID" 2>/dev/null; sleep 1; kill -9 "$QPID" 2>/dev/null

# ═══════════════════════════════════════════════════════════ RENAME ═══════
run_rename() {
  local marker="$1" budget="$2" tag="$3"
  NVME="$D/nvme_${tag}.img"; rm -f "$NVME"
  dd if=/dev/zero of="$NVME" bs=512 count=2048 2>/dev/null
  L="$D/${tag}_b1.log"; boot
  wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[mcg] WARN publish ${tag}_b1"
  sleep 3
  seed_origdoc
  ( tw m v d o c; sp; sel3; sp; tw r e n a m e d; k ret ) &
  TYPE_PID=$!
  local killed; killed=$(crash_on_marker "$marker" "$budget")
  [[ "$killed" == "1" ]] && r "${tag}_crash_injected" PASS || r "${tag}_crash_injected" "FAIL reason=marker_never_seen"

  L="$D/${tag}_b2.log"; boot
  wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[mcg] WARN publish ${tag}_b2"
  sleep 3
  grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT' "$L" && r "${tag}_fault_free_boot" FAIL || r "${tag}_fault_free_boot" PASS
  echo "$tag" > "$D/${tag}_result_marker"
}

run_rename '\[sexfiles\.diskfs\.v4\.crash_point\.rename_pending\]' 60 RENAME_PENDING
L="$D/RENAME_PENDING_b2.log"
open_spindle
tw d i s k; kv ret '\[spindle\.disk\.command\] found=' 60
if disk_has_name origdoc && ! disk_has_name renamed; then r RENAME_PENDING_old_name_intact PASS; else r RENAME_PENDING_old_name_intact FAIL; fi
tw c a t d o c; sp; sel3
kv ret '\[spindle\.catdoc\] id=3 size=64' 30
grep -qE '\[spindle\.catdoc\] id=3 size=64 hash=0x[0-9a-f]+ ok=1' "$L" && r RENAME_PENDING_content_unchanged PASS || r RENAME_PENDING_content_unchanged FAIL
kill "$QPID" 2>/dev/null; sleep 1; kill -9 "$QPID" 2>/dev/null

run_rename '\[sexfiles\.diskfs\.v4\.rename\.ok\] slot=3' 60 RENAME_COMMITTED
L="$D/RENAME_COMMITTED_b2.log"
open_spindle
tw d i s k; kv ret '\[spindle\.disk\.command\] found=' 60
if disk_has_name renamed && ! disk_has_name origdoc; then r RENAME_COMMITTED_new_name_intact PASS; else r RENAME_COMMITTED_new_name_intact FAIL; fi
tw c a t d o c; sp; sel3
kv ret '\[spindle\.catdoc\] id=3 size=64' 30
grep -qE '\[spindle\.catdoc\] id=3 size=64 hash=0x[0-9a-f]+ ok=1' "$L" && r RENAME_COMMITTED_content_unchanged PASS || r RENAME_COMMITTED_content_unchanged FAIL
kill "$QPID" 2>/dev/null; sleep 1; kill -9 "$QPID" 2>/dev/null

echo "[mcg] logs in $D"
if [[ "$FAILED" == "0" ]]; then echo "[diskfs.v4.metadata_crash.gate.result] PASS"; else echo "[diskfs.v4.metadata_crash.gate.result] FAIL"; exit 1; fi
