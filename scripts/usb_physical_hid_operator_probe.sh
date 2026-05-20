#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_FILE="/tmp/usb_physical_hid_operator_probe.log"
MODE="mouse"
TIMEOUT_SECS="45"
DISPLAY_MODE="gtk"
AUTO_BUILD=0

usage() {
    cat <<'HELP'
Usage: scripts/usb_physical_hid_operator_probe.sh [mouse|tablet] [--build] [--timeout SEC]

Operator-only proof lane for physical HID input.

Arguments:
  mouse|tablet      Select emulated USB HID device (default: mouse)

Options:
  --build           Run ./scripts/entrypoint_build.sh before probe
  --timeout SEC     QEMU run timeout in seconds (default: 45)
  -h, --help        Show this help

Exit codes:
  0 PASS: sexusb.hid.report.nonzero observed
  2 SKIP: no nonzero report and no fault markers
  1 FAIL: build/run failure or fault marker observed
HELP
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        mouse|tablet)
            MODE="$1"
            shift
            ;;
        --build)
            AUTO_BUILD=1
            shift
            ;;
        --timeout)
            TIMEOUT_SECS="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "error: unknown argument: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if ! [[ "$TIMEOUT_SECS" =~ ^[0-9]+$ ]]; then
    echo "error: --timeout must be an integer" >&2
    exit 1
fi

cd "$ROOT_DIR"

if [[ "$AUTO_BUILD" -eq 1 ]]; then
    echo "[operator] building ISO via ./scripts/entrypoint_build.sh"
    ./scripts/entrypoint_build.sh || {
        echo "[result] FAIL build failed"
        exit 1
    }
elif [[ ! -f "$ROOT_DIR/sexos-v1.0.0.iso" ]]; then
    echo "[operator] ISO missing; run with --build or build manually"
    echo "[result] FAIL missing ISO"
    exit 1
fi

rm -f "$LOG_FILE"

echo "[operator] Mode: $MODE"
echo "[operator] Display: $DISPLAY_MODE"
echo "[operator] Log: $LOG_FILE"
echo "[operator] Move and click inside the QEMU window for at least 10 seconds once boot is visible."
echo "[operator] Command: SEXOS_QEMU_DISPLAY=$DISPLAY_MODE SEXUSB_QEMU_DEVICE=$MODE ./scripts/qemu_harness.sh --timeout $TIMEOUT_SECS --display $DISPLAY_MODE"

set +e
SEXOS_QEMU_DISPLAY="$DISPLAY_MODE" \
SEXUSB_QEMU_DEVICE="$MODE" \
./scripts/qemu_harness.sh --timeout "$TIMEOUT_SECS" --display "$DISPLAY_MODE" > "$LOG_FILE" 2>&1
HARNESS_RC=$?
set -e

if [[ "$HARNESS_RC" -ne 0 && "$HARNESS_RC" -ne 124 ]]; then
    echo "[result] FAIL harness rc=$HARNESS_RC"
    exit 1
fi

echo "[operator] Marker grep results"
rg -n "sexusb\.hid\.report\.nonzero|sexusb\.hid\.report\.idle|sexusb\.hid\.report\.timeout|sexusb\.route\.sexinput\.ready|#PF|#GP|panic|fault\.kill" "$LOG_FILE" || true

if rg -q "#PF|#GP|panic|fault\.kill" "$LOG_FILE"; then
    echo "[result] FAIL fault markers present"
    exit 1
fi

if rg -q "sexusb\.hid\.report\.nonzero" "$LOG_FILE"; then
    echo "[result] PASS sexusb.hid.report.nonzero observed"
    exit 0
fi

echo "[result] SKIP no sexusb.hid.report.nonzero (no fault markers)"
exit 2
