#!/usr/bin/env bash
# QUIL_READ_V2_GATE — Lane 3 slice 2: Quil's real content-restore path
# (quil_persist_load, used by both the palette Load command and Linen's
# open-disk-document intent) migrated from OP_DISKFS_READ to
# OP_DISKFS_READ_V2. Proves exact byte transport for the class of content
# that used to break: bytes with the high bit set.
#
# Requires a build with SEXOS_QUIL_READ_V2_HIGHBIT_PROOF=1 so
# run_quil_read_v2_highbit_proof() compiles in and runs automatically at
# quil startup (option_env! reads the flag at compile time, not runtime -
# a plain SKIP_BUILD=1 rerun of an existing ISO would NOT have this proof
# in it). This script always rebuilds; it does not honor SKIP_BUILD.
set -uo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

export SEXOS_QUIL_READ_V2_HIGHBIT_PROOF=1
./scripts/entrypoint_build.sh > /tmp/sexos_qrv2_build.log 2>&1 || { echo "ROW build FAIL"; tail -40 /tmp/sexos_qrv2_build.log; exit 1; }
echo "ROW build PASS"

D=/tmp/sexos_qrv2
mkdir -p "$D"
NVME="$D/nvme.img"; rm -f "$NVME"; dd if=/dev/zero of="$NVME" bs=512 count=2048 2>/dev/null
L="$D/r.log"; : > "$L"; rm -f "$D/q.sock"

trap 'pkill -9 -f "qemu-system-x86_64.*${D}/nvme" 2>/dev/null' EXIT

qemu-system-x86_64 -M q35 -m 512M -cdrom sexos-v1.0.0.iso \
  -drive "if=none,id=nvm,file=$NVME,format=raw" \
  -device "nvme,serial=sexos01,drive=nvm" \
  -serial "file:$L" -qmp "unix:$D/q.sock,server=on,wait=off" \
  -display none -no-reboot -no-shutdown &
QPID=$!

wait_marker() { local d=$((SECONDS+$3)); while ((SECONDS<d)); do grep -qE "$1" "$2" 2>/dev/null && return 0; sleep 0.5; done; return 1; }
wait_marker '\[quil\.read_v2\.highbit\.done\]' "$L" 90 || echo "[qrv2] WARN proof marker never appeared"
sleep 2

kill "$QPID" 2>/dev/null; sleep 1; kill -9 "$QPID" 2>/dev/null; wait "$QPID" 2>/dev/null

FAILED=0
r() { echo "ROW $1 $2"; [[ "$2" == PASS* ]] || FAILED=1; }

grep -qE 'KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT' "$L" && r fault_free FAIL || r fault_free PASS

for label in 0x00 0x7f 0x80 0xff; do
  if grep -qE "\\[quil\\.read_v2\\.highbit\\.byte\\] label=${label} ok=1" "$L"; then
    r "byte_${label}_exact" PASS
  else
    r "byte_${label}_exact" FAIL
  fi
done

RESULT_LINE=$(grep -oE '\[quil\.read_v2\.highbit\.result\] len=[0-9]+ expected_len=[0-9]+ hash=0x[0-9a-f]+ ok=[01]' "$L" | tail -1)
if echo "$RESULT_LINE" | grep -q 'ok=1'; then
  r exact_transport_and_hash PASS
else
  r exact_transport_and_hash "FAIL got=[${RESULT_LINE:-none}]"
fi

NEG_LINE=$(grep -oE '\[quil\.read_v2\.highbit\.negative\] reported_failure=[01] buffer_intact=[01] dirty_preserved=[01] ok=[01]' "$L" | tail -1)
if echo "$NEG_LINE" | grep -q 'ok=1'; then
  r failed_reload_preserves_state PASS
else
  r failed_reload_preserves_state "FAIL got=[${NEG_LINE:-none}]"
fi

# No-status/data collision: the whole point of V2 is that a >=0x80 byte
# never causes a read_v2.err with a spurious status. Only check the
# portion of the log BEFORE highbit.result - the negative-path check
# after it deliberately forces one real error (invalid path_id) to prove
# failures are reported and handled, which is expected, not spurious.
if awk '/\[quil\.read_v2\.highbit\.result\]/{exit} 1' "$L" | grep -qE '\[quil\.persist\.load\.err\]|\[quil\.read_v2\.highbit\.err\]'; then
  r no_spurious_read_errors FAIL
else
  r no_spurious_read_errors PASS
fi

echo "[qrv2] log: $L"
if [[ "$FAILED" == "0" ]]; then echo "[quil.read_v2.gate.result] PASS"; else echo "[quil.read_v2.gate.result] FAIL"; exit 1; fi
