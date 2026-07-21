#!/usr/bin/env bash
# DISKFS_V4_READ_SIGNBIT_REGRESSION_GATE — Lane 3 starting point, not a
# Lane 1 pass/fail signal.
#
# OP_DISKFS_READ packs up to 8 raw content bytes directly into the reply
# u64; pdx_storage_call_bounded-style callers (quil, spindle) then treat
# any reply with bit 63 set as an error. A content byte >= 0x80 landing in
# the top position of an 8-byte chunk makes a legitimate read look like a
# negative status code and get rejected. This predates DISKFS_V4 (see
# docs/handoff/DISKFS_V4_GROWTH_V1.md) — it would affect any binary-ish
# content read through this path in any app, not just Quil or DiskFS; it
# just never manifested before because real content stayed ASCII.
#
# This gate exists to keep that bug reproducible in one command instead of
# rediscovering it by accident, and to give Lane 3 (separate reply status
# from payload data) a concrete starting compatibility test. It is NOT
# part of the DISKFS_V4 growth gate suite and does not block Lane 1 —
# filldoc/catdoc there deliberately stay on a 7-bit test pattern to avoid
# this exact bug.
#
# Expected result RIGHT NOW (bug present, unfixed): FAIL, with the
# specific row `read_signbit_bug_reproduced` — that is success for THIS
# gate's purpose (confirms the bug still reproduces exactly as documented).
# Once Lane 3 fixes the wire protocol, rerun this gate and expect it to
# flip to PASS with `read_signbit_bug_reproduced` becoming
# `read_signbit_bug_absent` — that is the signal the fix landed and this
# gate can be folded into the general reply-encoding regression suite.
#
# Usage: SKIP_BUILD=1 ./scripts/diskfs_v4_read_signbit_regression_gate.sh
set -uo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  ./scripts/entrypoint_build.sh > /tmp/sexos_signbit_build.log 2>&1 || { echo "ROW build FAIL"; exit 1; }
  echo "ROW build PASS"
else
  echo "ROW build SKIP"
fi
D=/tmp/sexos_signbit
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
  echo "[signbit] key miss key=$1 re=$2"
  return 1
}
tw() { for c in "$@"; do kv "$c" "\\[spindle\\.input\\.recv\\] key=printable ch=$c" 6; done; }
sp() { kv spc '\[spindle\.input\.recv\] key=printable ch= ' 6 || k spc; }
num() {
  local n="$1" d
  for ((i=0; i<${#n}; i++)); do
    d="${n:$i:1}"
    kv "$d" "\\[spindle\\.input\\.recv\\] key=printable ch=$d" 6
  done
}
sel3() { kv 3 '\[spindle\.input\.recv\] key=printable ch=3' 6; }

qemu-system-x86_64 -M q35 -m 512M -cdrom sexos-v1.0.0.iso \
  -drive "if=none,id=nvm,file=$NVME,format=raw" \
  -device "nvme,serial=sexos01,drive=nvm" \
  -serial "file:$L" -qmp "unix:$D/q.sock,server=on,wait=off" \
  -display none -no-reboot -no-shutdown &
QPID=$!
trap 'kill $QPID 2>/dev/null; sleep 1; kill -9 $QPID 2>/dev/null' EXIT

wait_marker '\[linen\.disk\.publish\.done\]' "$L" 120 || echo "[signbit] WARN publish"
sleep 3
kv scroll_lock 'reason=ToggleSpindle' 8; sleep 1

tw m k d o c; sp; tw s i g n b i t
kv ret '\[spindle\.mkdoc\] id=3 ok=1' 20

# 32 bytes: byte index 31 = (31*37+11)&0xFF = 0x86 (bit 7 set), landing as
# the top byte of the 4th 8-byte READ reply — deterministically triggers
# the bug on the first byte range that includes it.
tw f i l l d o c x; sp; sel3; sp; num 32
kv ret '\[spindle\.filldocx\] id=3 bytes=32 ok=1' 60

tw c a t d o c x; sp; sel3
kv ret '\[spindle\.catdocx\] id=3' 60

FAILED=0
r() { echo "ROW $1 $2"; [[ "$2" == FAIL* ]] && FAILED=1 || true; }

grep -q '\[spindle\.mkdoc\] id=3 ok=1' "$L" && r create PASS || r create FAIL
grep -q '\[spindle\.filldocx\] id=3 bytes=32 ok=1' "$L" && r fill_32_bytes PASS || r fill_32_bytes FAIL

if grep -q 'reason=read_signbit_bug' "$L"; then
  r read_signbit_bug_reproduced "FAIL (expected right now — see file header)"
elif grep -q '\[spindle\.catdocx\] id=3 size=32 ok=1' "$L"; then
  r read_signbit_bug_absent "PASS (protocol fixed — fold this into the general suite)"
else
  r read_signbit_bug_reproduced "FAIL inconclusive — neither the bug marker nor a clean VERIFIED appeared, see $L"
fi

grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT' "$L" && r fault_free FAIL || r fault_free PASS

if [[ "$FAILED" == "0" ]]; then
  echo "[diskfs.v4.read_signbit_regression.gate.result] PASS (bug fixed)"
else
  echo "[diskfs.v4.read_signbit_regression.gate.result] FAIL (bug reproduced — expected until Lane 3)"
  exit 1
fi
