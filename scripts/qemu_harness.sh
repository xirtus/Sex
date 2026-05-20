#!/usr/bin/env bash
set -euo pipefail

# QEMU_HARNESS_V1 — Canonical QEMU run harness for SexOS.
#
# Wraps dev.sh with timeout, deterministic serial log capture,
# and optional marker extraction.
#
# Usage:
#   ./scripts/qemu_harness.sh [--timeout N] [--markers] [--print-cmd]
#
# See docs/handoff/QEMU_HARNESS_V1.md

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$ROOT_DIR/logs"
LOG_FILE="$LOG_DIR/qemu-latest.log"
ISO="$ROOT_DIR/sexos-v1.0.0.iso"

# ---- defaults ---------------------------------------------------------------
TIMEOUT=""
DO_MARKERS=false
PRINT_CMD=false
DISPLAY="${SEXOS_QEMU_DISPLAY:-none}"
USB_DEV="${SEXUSB_QEMU_DEVICE:-tablet}"
QMP_SOCK="${SEXOS_QMP_SOCK:-}"

usb_qemu_arg() {
    case "$1" in
        mouse) echo "usb-mouse" ;;
        tablet) echo "usb-tablet" ;;
        kbd) echo "usb-kbd" ;;
        *) echo "$1" ;;
    esac
}

# ---- arg parse --------------------------------------------------------------
while [[ $# -gt 0 ]]; do
    case "$1" in
        -t|--timeout)  TIMEOUT="$2";  shift 2 ;;
        -m|--markers)  DO_MARKERS=true; shift ;;
        -p|--print-cmd) PRINT_CMD=true; shift ;;
        --display)     DISPLAY="$2";  shift 2 ;;
        --qmp)         QMP_SOCK="$2"; shift 2 ;;
        -h|--help)
            cat <<'HELP'
Usage: scripts/qemu_harness.sh [OPTIONS]

Canonical QEMU harness for SexOS development.

Options:
  -t, --timeout SEC    Kill QEMU after SEC seconds (exit 124)
  -m, --markers        Extract diagnostic markers from serial log
  -p, --print-cmd      Print canonical command and exit
  --display MODE       Display mode: none|sdl|gtk (default: none)
  --qmp PATH           Enable QMP and bind PATH as unix socket
  -h, --help           Show this message

Environment variables (passed to dev.sh):
  SEXOS_QEMU_DISPLAY   Display mode (default: none)
  SEXUSB_QEMU_DEVICE   USB device: tablet|mouse|kbd (default: tablet)
  QEMU_BIN             QEMU binary path override

Canonical config (the only path validated for USB/input testing):
  Machine:  q35
  CPU:      max,+pku
  Memory:   512M
  USB:      nec-usb-xhci + usb-tablet
  Serial:   stdio (captured to log)
  ISO:      sexos-v1.0.0.iso
  Display:  none (headless, serial-only)

Log path: logs/qemu-latest.log
HELP
            exit 0 ;;
        *)
            echo "error: unknown option: $1"
            echo "usage: $0 [--timeout SEC] [--markers] [--print-cmd] [--display MODE]"
            exit 1 ;;
    esac
done

# ---- preflight --------------------------------------------------------------
if [ ! -f "$ISO" ]; then
    echo "[FAIL] ISO not found: $ISO"
    echo "  Run: ./scripts/entrypoint_build.sh"
    exit 1
fi

# ---- export canonical env for dev.sh ----------------------------------------
export SEXOS_QEMU_DISPLAY="$DISPLAY"
export SEXUSB_QEMU_DEVICE="$USB_DEV"
if [ -n "$QMP_SOCK" ]; then
    export SEXOS_QEMU_QMP=1
    export SEXOS_QMP_SOCK="$QMP_SOCK"
fi

# ---- print-only mode --------------------------------------------------------
if [ "$PRINT_CMD" = true ]; then
    USB_QEMU_DEV="$(usb_qemu_arg "$USB_DEV")"
    echo "=== Canonical QEMU command ==="
    echo
    echo "  qemu-system-x86_64 \\"
    echo "      -M q35 \\"
    echo "      -m 512M \\"
    echo "      -cpu max,+pku \\"
    echo "      -cdrom sexos-v1.0.0.iso \\"
    echo "      -device nec-usb-xhci,id=xhci \\"
    echo "      -device ${USB_QEMU_DEV},bus=xhci.0 \\"
    echo "      -serial stdio \\"
    echo "      -display none \\"
    if [ -n "$QMP_SOCK" ]; then
        echo "      -qmp unix:${QMP_SOCK},server=on,wait=off"
    else
        echo "      # (no -qmp)"
    fi
    echo
    echo "USB device:  $USB_DEV"
    echo "USB arg:     -device ${USB_QEMU_DEV},bus=xhci.0"
    echo "Display:     $DISPLAY"
    echo
    echo "Single line:"
    echo "  qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku \\"
    echo "    -cdrom sexos-v1.0.0.iso \\"
    echo "    -device nec-usb-xhci,id=xhci \\"
    echo "    -device ${USB_QEMU_DEV},bus=xhci.0 \\"
    if [ -n "$QMP_SOCK" ]; then
        echo "    -serial stdio -display none -qmp unix:${QMP_SOCK},server=on,wait=off"
    else
        echo "    -serial stdio -display none"
    fi
    exit 0
fi

# ---- prepare log ------------------------------------------------------------
mkdir -p "$LOG_DIR"
rm -f "$LOG_FILE"

# ---- banner -----------------------------------------------------------------
echo "============================================"
echo " QEMU_HARNESS_V1"
echo "============================================"
USB_QEMU_DEV="$(usb_qemu_arg "$USB_DEV")"
echo " ISO:      $ISO"
echo " Size:     $(du -h "$ISO" | cut -f1)"
echo " USB dev:  $USB_DEV"
echo " USB arg:  -device ${USB_QEMU_DEV},bus=xhci.0"
echo " Display:  $DISPLAY"
if [ -n "$QMP_SOCK" ]; then
    echo " QMP sock: $QMP_SOCK"
    echo " QMP arg:  -qmp unix:${QMP_SOCK},server=on,wait=off"
fi
echo " Log:      $LOG_FILE"
[ -n "$TIMEOUT" ] && echo " Timeout:  ${TIMEOUT}s"
echo "--------------------------------------------"

# ---- launch QEMU ------------------------------------------------------------
cd "$ROOT_DIR"
set +e

if [ -n "$TIMEOUT" ]; then
    (timeout "$TIMEOUT" ./dev.sh run-nographic) 2>&1 | tee "$LOG_FILE"
else
    (./dev.sh run-nographic) 2>&1 | tee "$LOG_FILE"
fi

EXIT_CODE=${PIPESTATUS[0]}
set -e

echo "--------------------------------------------"
echo " QEMU exit:  $EXIT_CODE"
echo " Log:        $LOG_FILE"
echo " Log lines:  $(wc -l < "$LOG_FILE" 2>/dev/null || echo 0)"

# ---- marker extraction (--markers or after timeout) -------------------------
if [ "$DO_MARKERS" = true ] || [ -n "$TIMEOUT" ]; then
    echo ""
    echo "=== Marker summary ==="

    # USB/XHCI discovery
    usb_hits=$(rg -c "usb\.host\.controller|DevMgr.*XHCI|xhci.*found|USB.*registered|usb_xhci" "$LOG_FILE" 2>/dev/null || echo 0)
    echo " USB/XHCI:          $usb_hits marker(s)"

    # Input forwarding
    input_hits=$(rg -c "sexusb\.forward|HID.*report|input.*event|usb-tablet|mouse" "$LOG_FILE" 2>/dev/null || echo 0)
    echo " Input/HID:         $input_hits marker(s)"

    # Shell/silk lifecycle
    shell_hits=$(rg -c "shell.*ready|silk.*frame|chrome.*swap|chrome.*template|scene.*render" "$LOG_FILE" 2>/dev/null || echo 0)
    echo " Shell/Silk:        $shell_hits marker(s)"

    # Boot / init sequence
    boot_hits=$(rg -c "SexOS.*init|Kernel.*init|ACPI.*ready|PCI.*enumerate|smp.*init|apic.*init" "$LOG_FILE" 2>/dev/null || echo 0)
    echo " Boot/Init:         $boot_hits marker(s)"

    # Show relevant snippets
    echo ""
    echo "--- USB/XHCI snippets (up to 5) ---"
    rg -n "usb\.host\.controller|DevMgr.*XHCI|xhci.*found|USB.*registered|\[usb\." "$LOG_FILE" 2>/dev/null | head -5 || echo " (none)"
    echo ""
    echo "--- Input snippets (up to 5) ---"
    rg -n "sexusb\.forward|HID.*report|input.*event|\[sexusb\]" "$LOG_FILE" 2>/dev/null | head -5 || echo " (none)"
    echo ""
    echo "--- Boot log tail (last 10 lines) ---"
    tail -10 "$LOG_FILE" 2>/dev/null || true
fi

exit "$EXIT_CODE"
