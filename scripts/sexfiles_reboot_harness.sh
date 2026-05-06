#!/usr/bin/env bash
# ==============================================================================
# sexfiles_reboot_harness.sh — SexFiles Reboot Persistence Harness V1
# ==============================================================================
#
# HONEST STATUS: TRUE TWO-BOOT PERSISTENCE IS BLOCKED.
#
# The DiskFS backend is an in-memory scaffold with no real block device route.
# This harness currently exercises the single-boot journal replay roundtrip
# (format->create->snapshot->re-format->restore+replay->verify). The write and
# verify phases execute within a single QEMU invocation because no persistent
# writable media exists to preserve the DiskFS state across separate boots.
#
# When a real block device server + PDX ABI exist (see the BLOCKER contract
# below), the two-phase workflow would be:
#
#   Phase A (write):
#     SEXOS_SEXFILES_REBOOT_PROOF=write ./scripts/sexfiles_reboot_harness.sh phase-a
#     -> Boots QEMU with a persistent disk image
#     -> sexfiles formats the disk, creates known objects, commits journal
#     -> Emits [sexfiles.reboot.proof.write_commit] marker
#     -> QEMU shuts down cleanly
#
#   Phase B (verify):
#     SEXOS_SEXFILES_REBOOT_PROOF=verify ./scripts/sexfiles_reboot_harness.sh phase-b
#     -> Boots QEMU with the SAME persistent disk image from phase A
#     -> sexfiles reads superblock from disk, replays journal, recovers objects
#     -> Emits [sexfiles.reboot.proof.verify_mount], .verify_read, .match markers
#     -> Harness script compares object IDs/data from both phases
#
# BLOCKER CONTRACT:
#   docs/handoff/SEXFILES_REAL_BLOCK_BACKEND_V1.md
#   docs/handoff/SEXFILES_REBOOT_PERSISTENCE_HARNESS_V1.md
#
# Required environment:
#   SEXOS_SEXFILES_REBOOT_PROOF=write   (phase A)
#   SEXOS_SEXFILES_REBOOT_PROOF=verify  (phase B)
#
# Usage (single-boot mode only for now):
#   ./scripts/sexfiles_reboot_harness.sh
#
# ==============================================================================

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"
ISO="${ISO:-sexos-v1.0.0.iso}"
PROBE_SECONDS="${PROBE_SECONDS:-25}"
HARNESS_DIR="${HARNESS_DIR:-$ROOT_DIR/.harness_reboot}"
PERSIST_DISK="${PERSIST_DISK:-$HARNESS_DIR/sexfiles_persist.qcow2}"

MODE="${SEXOS_SEXFILES_REBOOT_PROOF:-single_boot}"
if [[ "$MODE" == "write" ]]; then PHASE="WRITE"
elif [[ "$MODE" == "verify" ]]; then PHASE="VERIFY"
else PHASE="SINGLE_BOOT"
fi

fail() { echo "[HARNESS FAIL] $1"; exit 1; }
has_marker() { grep -qE "$1" "$2" 2>/dev/null && echo 1 || echo 0; }

echo "============================================"
echo " SEXFILES REBOOT PERSISTENCE HARNESS V1"
echo " Phase: $PHASE"
echo "============================================"
echo ""

if ! command -v "$QEMU_BIN" &>/dev/null; then
    fail "QEMU binary not found: $QEMU_BIN"
fi
if [[ ! -f "$ISO" ]]; then
    fail "ISO not found: $ISO. Run ./scripts/entrypoint_build.sh first."
fi

mkdir -p "$HARNESS_DIR"

run_phase_write() {
    echo "[PHASE A] Write phase..."
    if [[ ! -f "$PERSIST_DISK" ]]; then
        echo "[PHASE A] Creating persistent disk image: $PERSIST_DISK"
        qemu-img create -f qcow2 "$PERSIST_DISK" 64M 2>/dev/null || true
    fi
    echo "[PHASE A] BLOCKER: DiskFS has no block I/O wiring - disk unused by OS."
    echo ""
    local LOG="$HARNESS_DIR/write_boot.log"
    rm -f "$LOG"
    set +e
    timeout "${PROBE_SECONDS}" "$QEMU_BIN" \
        -M q35 -m 512M -cpu max,+pku \
        -cdrom "$ISO" \
        -drive file="$PERSIST_DISK",format=qcow2,if=none,id=disk0 \
        -device ahci,id=ahci \
        -device ide-hd,drive=disk0,bus=ahci.0 \
        -device nec-usb-xhci,id=xhci \
        -device usb-tablet,bus=xhci.0 \
        -serial "file:$LOG" \
        -display none -no-reboot -no-shutdown || true
    set -e
    echo "[PHASE A] Log: $LOG ($(wc -l < "$LOG" 2>/dev/null || echo 0) lines)"
    local ok; ok=$(has_marker '\[sexfiles\.reboot\.proof\.write_commit\]\s+ok=1' "$LOG")
    if [[ "$ok" == "1" ]]; then
        echo "[PHASE A] PASS: write_commit marker found"
    else
        echo "[PHASE A] FAIL: write_commit marker missing"
        return 1
    fi
}

run_phase_verify() {
    echo "[PHASE B] Verify phase..."
    if [[ ! -f "$PERSIST_DISK" ]]; then
        fail "Persistent disk not found: $PERSIST_DISK"
    fi
    echo "[PHASE B] BLOCKER: DiskFS cannot read disk image."
    echo ""
    local LOG="$HARNESS_DIR/verify_boot.log"
    rm -f "$LOG"
    set +e
    timeout "${PROBE_SECONDS}" "$QEMU_BIN" \
        -M q35 -m 512M -cpu max,+pku \
        -cdrom "$ISO" \
        -drive file="$PERSIST_DISK",format=qcow2,if=none,id=disk0 \
        -device ahci,id=ahci \
        -device ide-hd,drive=disk0,bus=ahci.0 \
        -device nec-usb-xhci,id=xhci \
        -device usb-tablet,bus=xhci.0 \
        -serial "file:$LOG" \
        -display none -no-reboot -no-shutdown || true
    set -e
    echo "[PHASE B] Log: $LOG ($(wc -l < "$LOG" 2>/dev/null || echo 0) lines)"
    local m_ok r_ok x_ok
    m_ok=$(has_marker '\[sexfiles\.reboot\.proof\.verify_mount\]\s+ok=1' "$LOG")
    r_ok=$(has_marker '\[sexfiles\.reboot\.proof\.verify_read\]\s+ok=1' "$LOG")
    x_ok=$(has_marker '\[sexfiles\.reboot\.proof\.match\]\s+ok=1' "$LOG")
    echo "[PHASE B] verify_mount: $( [[ "$m_ok" == "1" ]] && echo PASS || echo FAIL )"
    echo "[PHASE B] verify_read:  $( [[ "$r_ok" == "1" ]] && echo PASS || echo FAIL )"
    echo "[PHASE B] match:        $( [[ "$x_ok" == "1" ]] && echo PASS || echo FAIL )"
    if [[ "$m_ok" == "1" && "$r_ok" == "1" && "$x_ok" == "1" ]]; then
        echo "[PHASE B] PASS"
        return 0
    else
        echo "[PHASE B] FAIL"
        return 1
    fi
}

run_single_boot() {
    echo "[SINGLE-BOOT] Combined write+verify in single QEMU invocation."
    echo "[SINGLE-BOOT] Reboot simulated via journal snapshot+replay."
    echo ""
    echo "[SINGLE-BOOT] Building with SEXOS_SEXFILES_REBOOT_PROOF=1..."
    SEXOS_SEXFILES_REBOOT_PROOF=1 \
    SEXOS_TRACE_ACTIVE=1 \
    SEXOS_ENTRYPOINT_ACTIVE=1 \
    SEXOS_BUILD_ROOT="$ROOT_DIR/scripts/entrypoint_build.sh" \
    SEXOS_CONTRACT_SNAPSHOT="$ROOT_DIR/.sexos_snapshot/contract.snapshot.toml" \
    SEXOS_ABI_SNAPSHOT="$ROOT_DIR/.sexos_snapshot/abi.snapshot.lock" \
    SEXOS_BUILD_SPEC="$ROOT_DIR/sexos_build_spec.toml" \
    bash scripts/sexos_build_trace.sh sexos_build_spec.toml

    rm -f "$HARNESS_DIR/serial.log"
    GATE_DIR="$HARNESS_DIR" LOG_PATH="$HARNESS_DIR/serial.log" \
        bash scripts/master_runtime_gate.sh --skip-build --probe "$PROBE_SECONDS" --keep-log

    echo ""
    echo "[SINGLE-BOOT] Relevant markers:"
    grep -E "sexfiles\.reboot" "$HARNESS_DIR/serial.log" 2>/dev/null || echo "  (none)"
    local all_ok
    all_ok=$(has_marker '\[sexfiles\.reboot\.proof\.match\]\s+ok=1' "$HARNESS_DIR/serial.log")
    if [[ "$all_ok" == "1" ]]; then
        echo "============================================"
        echo " HARNESS RESULT: SINGLE-BOOT PROVEN"
        echo " TRUE TWO-BOOT: BLOCKED"
        echo " Log: $HARNESS_DIR/serial.log"
        echo " Handoff: docs/handoff/SEXFILES_REBOOT_PERSISTENCE_HARNESS_V1.md"
        echo " Blocker: docs/handoff/SEXFILES_REAL_BLOCK_BACKEND_V1.md"
        echo "============================================"
        return 0
    else
        echo "HARNESS RESULT: FAILED"
        return 1
    fi
}

case "$MODE" in
    write) run_phase_write ;;
    verify) run_phase_verify ;;
    single_boot|*) run_single_boot ;;
esac
