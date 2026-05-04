#!/usr/bin/env bash
# QEMU_MARKERS_V1 — Extract SexOS diagnostic markers from QEMU serial log.
# Usage: scripts/qemu_markers.sh [LOGFILE]
# Default: logs/qemu-latest.log
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG="${1:-$ROOT_DIR/logs/qemu-latest.log}"

if [ ! -f "$LOG" ]; then
    echo "error: log not found: $LOG"
    echo "usage: $0 [path-to-qemu-serial-log]"
    exit 1
fi

echo "=== QEMU_MARKERS_V1 ==="
echo "Log: $LOG"
echo ""

extract() {
    local label="$1"
    local pattern="$2"
    local count
    count=$(rg -c "$pattern" "$LOG" 2>/dev/null || echo 0)
    echo "[$label] $count hit(s)"
    rg -n "$pattern" "$LOG" 2>/dev/null | head -10 || true
    echo ""
}

extract "USB_HOST_CONTROLLER"  "usb\.host\.controller|DevMgr.*XHCI|xhci.*found|\[usb\."
extract "USB_HID_INPUT"        "sexusb\.forward|HID.*report|input.*event|\[sexusb\]|usb-tablet"
extract "SHELL_SILK"           "shell.*ready|silk.*frame|chrome.*swap|chrome.*template|scene.*render"
extract "KERNEL_BOOT"          "SexOS.*init|Kernel.*init|ACPI.*ready|PCI.*enumerate|smp.*init|apic.*init"
extract "DISPLAY_FRAMEBUFFER"  "sexdisplay|framebuffer|FB.*Phys|fb.*init|pixel"
extract "ERROR_WARN_PANIC"     "panic|ERROR|WARN|FAIL|fault|trap"

echo "=== Log tail (last 20 lines) ==="
tail -20 "$LOG"
