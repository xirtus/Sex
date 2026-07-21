#!/usr/bin/env bash
# DISKFS_V4_MANIFEST_VALIDATION_GATE — Lane 2, Slice E: mount-time handling
# of corrupt on-disk metadata.
#
# Code-inspection finding that motivated this gate: before this slice,
# v4_ensure() (servers/sexfiles/src/vfs.rs) treated ANY manifest sector that
# didn't match the current magic+version as "recognized V3 or blank" and
# unconditionally bootstrapped a fresh V4 manifest over it — including a
# manifest sector that was genuinely corrupted (torn write, bad sector,
# foreign data) rather than actually blank. That is exactly the
# unacceptable behavior called out for this slice: treating corrupt storage
# as empty and silently overwriting it.
#
# Fixed by distinguishing three cases instead of two:
#   - magic matches, version < current  -> real migration (unchanged).
#   - sector is all-zero                -> genuinely unformatted (unchanged).
#   - anything else (non-matching magic on a non-blank sector)
#       -> ERR_CORRUPT. Mount refuses. Every v4_ensure() caller already
#          propagates Err(e) as a hard error, so every DISKFS_V4 op just
#          fails cleanly instead of a panic or a silent wipe.
#
# Separately, v4_ensure()'s PER-ENTRY checksum check (pre-existing, not
# changed by this slice) already implements "reject the corrupt object
# while preserving other valid objects" at finer granularity: a single
# entry whose stored checksum doesn't match gets dropped in isolation, the
# rest of the manifest loads normally. Verified here, not just asserted.
#
# This gate does NOT attempt live crash-timing injection (there is no
# meaningful "crash mid-corruption" — a bad sector or torn write is a
# storage-layer event, not one this codebase can trigger from inside a
# guest). Instead it directly corrupts bytes in the raw disk image between
# boots, which is the actual real-world failure this code path exists to
# survive.
set -uo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  ./scripts/entrypoint_build.sh > /tmp/sexos_ci_build.log 2>&1 || { echo "ROW build FAIL"; exit 1; }
  echo "ROW build PASS"
else
  echo "ROW build SKIP"
fi

D=/tmp/sexos_manifest_val
mkdir -p "$D"
MANIFEST_LBA=2046
MANIFEST_BYTE_OFF=$((MANIFEST_LBA * 512))

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
  echo "[mvg] key miss key=$1 re=$2"
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
sel4() { kv 4 '\[spindle\.input\.recv\] key=printable ch=4' 15; }

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
  # See diskfs_v4_metadata_crash_gate.sh for why a direction-aware check is
  # required here instead of a direction-blind "any ToggleSpindle fired".
  local tries=0 last_new
  while ((tries<6)); do
    k scroll_lock
    sleep 1.5
    last_new=$(grep -oE '\[shell\.kbd\.ui\.focus\] old=[0-9]+ new=[0-9]+ frame=[0-9]+ reason=ToggleSpindle' "$L" | tail -1 | grep -oE 'new=[0-9]+' | cut -d= -f2)
    if [[ -n "$last_new" && "$last_new" != "201" ]]; then sleep 1; return 0; fi
    tries=$((tries+1))
  done
  echo "[mvg] open_spindle failed to focus spindle after $tries tries (last_new=${last_new:-none})"
  return 1
}
shutdown_clean() { kill "$QPID" 2>/dev/null; sleep 1; kill -9 "$QPID" 2>/dev/null; wait "$QPID" 2>/dev/null; }

# ── Seed a disk with two real, verified objects (slot 3 "keepdoc", slot 4
# "targetdoc"), then shut down cleanly (no crash-timing involved here).
BASE="$D/base.img"
rm -f "$BASE"
dd if=/dev/zero of="$BASE" bs=512 count=2048 2>/dev/null
NVME="$BASE"
L="$D/seed.log"
boot
wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[mvg] WARN publish seed"
sleep 3
open_spindle
tw m k d o c; sp; tw k e e p d o c
kv ret '\[spindle\.mkdoc\] id=3 ok=1' 20
tw f i l l d o c; sp; sel3; sp; num 48
kv ret '\[spindle\.filldoc\] id=3 bytes=48 ok=1' 30
tw m k d o c; sp; tw t a r g e t d o c
kv ret '\[spindle\.mkdoc\] id=4 ok=1' 20
tw f i l l d o c; sp; sel4; sp; num 32
kv ret '\[spindle\.filldoc\] id=4 bytes=32 ok=1' 30
tw c a t d o c; sp; sel3
kv ret '\[spindle\.catdoc\] id=3 size=48' 30
KEEP_HASH=$(grep -oE '\[spindle\.catdoc\] id=3 size=48 hash=0x[0-9a-f]+ ok=1' "$L" | tail -1)
shutdown_clean
[[ -n "$KEEP_HASH" ]] && r seed_setup PASS || r seed_setup FAIL

# ═══════════════════════════ Test A: whole-sector garbage (real corruption) ═
IMG_A="$D/nvme_corrupt_whole.img"
cp "$BASE" "$IMG_A"
python3 - "$IMG_A" "$MANIFEST_BYTE_OFF" <<'PY'
import sys
path, off = sys.argv[1], int(sys.argv[2])
with open(path, "r+b") as f:
    f.seek(off)
    f.write(bytes([0xAA]) * 512)
PY
BEFORE_HASH_A=$(python3 -c "
import hashlib
with open('$IMG_A','rb') as f:
    f.seek($MANIFEST_BYTE_OFF)
    print(hashlib.sha256(f.read(512)).hexdigest())
")
NVME="$IMG_A"; L="$D/A_boot.log"
boot
wait_marker '\[sexfiles\.diskfs\.v4\.mount\.err\]|\[sexfiles\.diskfs\.v4\.load\.ok\]|\[sexfiles\.diskfs\.v4\.migrate\.ok\]' "$L" 60 || echo "[mvg] WARN no mount outcome A"
sleep 5
grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT' "$L" && r A_fault_free_boot FAIL || r A_fault_free_boot PASS
grep -qE '\[sexfiles\.diskfs\.v4\.mount\.err\] reason=corrupt_manifest' "$L" && r A_mount_refused PASS || r A_mount_refused FAIL
grep -qE '\[sexfiles\.diskfs\.v4\.load\.ok\]|\[sexfiles\.diskfs\.v4\.migrate\.ok\]' "$L" && r A_did_not_silently_load FAIL || r A_did_not_silently_load PASS
shutdown_clean
AFTER_HASH_A=$(python3 -c "
import hashlib
with open('$IMG_A','rb') as f:
    f.seek($MANIFEST_BYTE_OFF)
    print(hashlib.sha256(f.read(512)).hexdigest())
")
if [[ "$BEFORE_HASH_A" == "$AFTER_HASH_A" ]]; then r A_manifest_not_overwritten PASS; else r A_manifest_not_overwritten "FAIL before=$BEFORE_HASH_A after=$AFTER_HASH_A"; fi

# ═══════════════════════════ Test B: genuinely blank disk (positive control) ═
IMG_B="$D/nvme_blank.img"
rm -f "$IMG_B"
dd if=/dev/zero of="$IMG_B" bs=512 count=2048 2>/dev/null
NVME="$IMG_B"; L="$D/B_boot.log"
boot
wait_marker '\[sexfiles\.diskfs\.v4\.migrate\.ok\]' "$L" 60 && r B_blank_bootstraps PASS || r B_blank_bootstraps FAIL
grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT' "$L" && r B_fault_free_boot FAIL || r B_fault_free_boot PASS
shutdown_clean

# ═══════════════════ Test C: single entry checksum corruption ═══════════════
# Flip only slot 4's stored checksum bytes (off = 16 + 4*32 + 28, len 2).
# Magic/version/other entries untouched -> the whole manifest must still
# load; ONLY slot 4 gets dropped; slot 3 must survive with EXACT content.
IMG_C="$D/nvme_entry_checksum.img"
cp "$BASE" "$IMG_C"
ENTRY4_CS_OFF=$((MANIFEST_BYTE_OFF + 16 + 4*32 + 28))
python3 - "$IMG_C" "$ENTRY4_CS_OFF" <<'PY'
import sys
path, off = sys.argv[1], int(sys.argv[2])
with open(path, "r+b") as f:
    f.seek(off)
    cur = f.read(2)
    f.seek(off)
    f.write(bytes([cur[0] ^ 0xFF, cur[1] ^ 0xFF]))
PY
NVME="$IMG_C"; L="$D/C_boot.log"
boot
wait_marker '\[sexfiles\.diskfs\.v4\.load\.ok\]' "$L" 60 && r C_manifest_still_loads PASS || r C_manifest_still_loads FAIL
grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT' "$L" && r C_fault_free_boot FAIL || r C_fault_free_boot PASS
grep -qE '\[sexfiles\.diskfs\.v4\.load\.drop\] slot=4 reason=checksum' "$L" && r C_bad_entry_dropped PASS || r C_bad_entry_dropped FAIL
sleep 3
open_spindle
tw c a t d o c; sp; sel3
kv ret '\[spindle\.catdoc\] id=3 size=48' 30
C_KEEP_HASH=$(grep -oE '\[spindle\.catdoc\] id=3 size=48 hash=0x[0-9a-f]+ ok=1' "$L" | tail -1)
if [[ -n "$C_KEEP_HASH" && "$C_KEEP_HASH" == "$KEEP_HASH" ]]; then
  r C_other_object_survives_exact PASS
else
  r C_other_object_survives_exact "FAIL expected=[$KEEP_HASH] got=[$C_KEEP_HASH]"
fi
tw d i s k; k ret; sleep 2
grep -qE '\[spindle\.disk\.entry\] slot=4 name=targetdoc' "$L" && r C_dropped_object_absent FAIL || r C_dropped_object_absent PASS
shutdown_clean

# ═══════════════════ Test D: recovery idempotence (Slice F) ═════════════════
# Two consecutive clean boots of the SAME valid, already-seeded image with NO
# mutation in between. By construction v4_ensure() only calls v4_persist()
# (which is the only thing that bumps V4_GENERATION) on an actual mutation
# or a dropped-entry repair; a clean load neither. Confirm that's really
# true end to end: same generation both boots, same object hash, and no
# extra manifest write (checked directly against the raw image bytes, not
# just the log).
IMG_D="$D/nvme_idempotence.img"
cp "$BASE" "$IMG_D"
MID_HASH_1=$(python3 -c "
import hashlib
with open('$IMG_D','rb') as f:
    f.seek($MANIFEST_BYTE_OFF)
    print(hashlib.sha256(f.read(512)).hexdigest())
")
NVME="$IMG_D"; L="$D/D1_boot.log"
boot
wait_marker '\[sexfiles\.diskfs\.v4\.load\.ok\]' "$L" 60 && r D1_clean_load PASS || r D1_clean_load FAIL
GEN_1=$(grep -oE '\[sexfiles\.diskfs\.v4\.load\.ok\] live=[0-9]+ generation=[0-9]+' "$L" | tail -1)
sleep 3
open_spindle
tw c a t d o c; sp; sel3
kv ret '\[spindle\.catdoc\] id=3 size=48' 30
HASH_1=$(grep -oE '\[spindle\.catdoc\] id=3 size=48 hash=0x[0-9a-f]+ ok=1' "$L" | tail -1)
shutdown_clean
MID_HASH_2=$(python3 -c "
import hashlib
with open('$IMG_D','rb') as f:
    f.seek($MANIFEST_BYTE_OFF)
    print(hashlib.sha256(f.read(512)).hexdigest())
")
if [[ "$MID_HASH_1" == "$MID_HASH_2" ]]; then r D_no_write_on_clean_load PASS; else r D_no_write_on_clean_load "FAIL before=$MID_HASH_1 after=$MID_HASH_2"; fi

L="$D/D2_boot.log"
boot
wait_marker '\[sexfiles\.diskfs\.v4\.load\.ok\]' "$L" 60 && r D2_clean_load PASS || r D2_clean_load FAIL
GEN_2=$(grep -oE '\[sexfiles\.diskfs\.v4\.load\.ok\] live=[0-9]+ generation=[0-9]+' "$L" | tail -1)
sleep 3
open_spindle
tw c a t d o c; sp; sel3
kv ret '\[spindle\.catdoc\] id=3 size=48' 30
HASH_2=$(grep -oE '\[spindle\.catdoc\] id=3 size=48 hash=0x[0-9a-f]+ ok=1' "$L" | tail -1)
if [[ -n "$GEN_1" && "$GEN_1" == "$GEN_2" ]]; then r D_generation_stable PASS; else r D_generation_stable "FAIL boot1=[$GEN_1] boot2=[$GEN_2]"; fi
if [[ -n "$HASH_1" && "$HASH_1" == "$HASH_2" ]]; then r D_content_stable PASS; else r D_content_stable "FAIL boot1=[$HASH_1] boot2=[$HASH_2]"; fi
# Now perform ONE real mutation and reboot a third time — confirm the
# system stays usable (idempotence isn't masking a frozen/stuck state).
tw f i l l d o c; sp; sel4; sp; num 40
kv ret '\[spindle\.filldoc\] id=4 bytes=40 ok=1' 30
shutdown_clean
L="$D/D3_boot.log"
boot
wait_marker '\[sexfiles\.diskfs\.v4\.load\.ok\]' "$L" 60
sleep 3
open_spindle
tw c a t d o c; sp; sel4
kv ret '\[spindle\.catdoc\] id=4 size=40' 30
grep -qE '\[spindle\.catdoc\] id=4 size=40 hash=0x[0-9a-f]+ ok=1' "$L" && r D_post_idempotence_mutation_succeeds PASS || r D_post_idempotence_mutation_succeeds FAIL
shutdown_clean

echo "[mvg] logs in $D"
if [[ "$FAILED" == "0" ]]; then echo "[diskfs.v4.manifest_validation.gate.result] PASS"; else echo "[diskfs.v4.manifest_validation.gate.result] FAIL"; exit 1; fi
