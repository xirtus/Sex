#!/usr/bin/env bash
# usb_pointer_real_report_operator_probe.sh — USB Pointer Real Report Operator Probe V1
#
# Unblocks USB HID pointer reports by feeding real host input events into QEMU
# via evdev passthrough (-object input-linux).  This bypasses the QMP/HMP
# injection barrier (which routes to PS/2 only) and delivers real mouse/tablet
# events to the USB HID layer.
#
# Modes:
#   evdev       Default.  Passes host input device via -object input-linux.
#               Requires read access to /dev/input/eventX (input group).
#   gtk         Uses -display gtk.  Operator must move mouse in QEMU window.
#   usb-host    Uses -device usb-host.  Requires physical USB device + IDs.
#
# Usage:
#   ./scripts/usb_pointer_real_report_operator_probe.sh [mode] [log_path]
#
#   mode defaults to "evdev", log_path defaults to
#   /tmp/usb_pointer_report_event_unblock_v1.log
#
#   SEXUSB_EVDEV_MOUSE overrides the mouse evdev path
#   (default: /dev/input/event3 — first USB mouse detected)
#
# Returns:
#   0 — build, boot, gate scan all PASS
#   1 — build failed, gate failed, or faults detected
#   2 — fatal error

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MODE="${1:-evdev}"
LOG="${2:-/tmp/usb_pointer_report_event_unblock_v1.log}"
GATE_SCRIPT="./scripts/daily_driver_master_gate.sh"
BUILD_SCRIPT="./scripts/entrypoint_build.sh"
ISO="sexos-v1.0.0.iso"
QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"
PROBE_SECONDS="${USB_POINTER_PROBE_SECONDS:-45}"
QMP_SOCK="/tmp/sexos_usbptr_probe.sock"
QEMU_PID=""

die() { echo "FATAL: $*" >&2; exit 2; }

cleanup() {
    if [ -n "${QEMU_PID:-}" ] && kill -0 "$QEMU_PID" 2>/dev/null; then
        kill "$QEMU_PID" 2>/dev/null || true
        wait "$QEMU_PID" 2>/dev/null || true
    fi
    rm -f "$QMP_SOCK"
}
trap cleanup EXIT INT TERM

echo "============================================"
echo " USB POINTER REAL REPORT OPERATOR PROBE V1"
echo "============================================"
echo ""
echo "  mode:    $MODE"
echo "  log:     $LOG"
echo "  probe:   ${PROBE_SECONDS}s"
echo ""

[ -x "$BUILD_SCRIPT" ] || die "build script not found: $BUILD_SCRIPT"
[ -x "$GATE_SCRIPT" ] || die "gate script not found: $GATE_SCRIPT"

# ---- Build ----
echo "[probe] Building ISO..."
"$BUILD_SCRIPT" 2>&1 | tail -3
[ -f "$ISO" ] || die "ISO not found after build: $ISO"
echo "[probe] Build OK"
echo ""

# ---- Mode-specific QEMU args ----
EXTRA_QEMU_ARGS=()

case "$MODE" in
  evdev)
    EVDEV_MOUSE="${SEXUSB_EVDEV_MOUSE:-/dev/input/event3}"
    [ -r "$EVDEV_MOUSE" ] || die "Cannot read evdev mouse: $EVDEV_MOUSE (try SEXUSB_EVDEV_MOUSE=/dev/input/eventX or chmod)"
    echo "[probe] evdev mouse: $EVDEV_MOUSE"
    echo "[probe] IMPORTANT: Move the mouse and click during the ${PROBE_SECONDS}s probe window!"
    echo "[probe] The host mouse cursor will be shared — QEMU reads the same events."
    echo ""
    EXTRA_QEMU_ARGS+=(-object "input-linux,id=mouse1,evdev=${EVDEV_MOUSE}")
    # Also try to pass keyboard for completeness
    EVDEV_KBD="${SEXUSB_EVDEV_KBD:-}"
    if [ -n "$EVDEV_KBD" ] && [ -r "$EVDEV_KBD" ]; then
        EXTRA_QEMU_ARGS+=(-object "input-linux,id=kbd1,evdev=${EVDEV_KBD}")
        echo "[probe] evdev keyboard: $EVDEV_KBD (type a key during probe)"
    fi
    ;;
  gtk)
    echo "[probe] gtk mode: QEMU window will open. Move mouse INSIDE the QEMU window."
    echo "[probe] IMPORTANT: Click inside the QEMU window first to grab input!"
    echo ""
    EXTRA_QEMU_ARGS+=(-display gtk)
    ;;
  usb-host)
    USB_HOSTBUS="${SEXUSB_USB_HOSTBUS:-3}"
    USB_HOSTADDR="${SEXUSB_USB_HOSTADDR:-2}"
    echo "[probe] usb-host mode: bus=$USB_HOSTBUS addr=$USB_HOSTADDR"
    echo "[probe] Will pass through physical USB device to QEMU."
    echo ""
    EXTRA_QEMU_ARGS+=(-device "usb-host,hostbus=${USB_HOSTBUS},hostaddr=${USB_HOSTADDR}")
    ;;
  *)
    die "Unknown mode: $MODE (use evdev, gtk, or usb-host)"
    ;;
esac

# ---- QEMU Launch ----
echo "[probe] Launching QEMU..."
echo "[probe] Probe window: ${PROBE_SECONDS}s — move/click mouse NOW"

rm -f "$LOG"

# Assemble minimal QEMU args matching daily driver profile
QEMU_ARGS=(
    -M q35
    -m 512M
    -cpu max,+pku
    -cdrom "$ISO"
    -device nec-usb-xhci,id=xhci
    -device usb-tablet,bus=xhci.0
    -serial "file:$LOG"
    -qmp "unix:${QMP_SOCK},server,nowait"
    -display none
    -no-reboot
    -no-shutdown
    "${EXTRA_QEMU_ARGS[@]}"
)

"$QEMU_BIN" "${QEMU_ARGS[@]}" &
QEMU_PID=$!

if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    die "QEMU failed to start"
fi
echo "[probe] QEMU PID: $QEMU_PID"

# ---- Probe window ----
echo "[probe] Waiting ${PROBE_SECONDS}s for boot + USB reports..."
sleep "$PROBE_SECONDS"

# ---- Kill QEMU ----
echo "[probe] Stopping QEMU..."
kill "$QEMU_PID" 2>/dev/null || true
wait "$QEMU_PID" 2>/dev/null || true
QEMU_PID=""
echo "[probe] QEMU stopped"
echo ""

# ---- Check log ----
if [ ! -s "$LOG" ]; then
    die "Log file empty or missing: $LOG"
fi
LOG_LINES=$(wc -l < "$LOG")
echo "[probe] Log captured: ${LOG_LINES} lines"

# ---- Check for USB reports ----
echo ""
echo "============================================"
echo " USB POINTER REPORT SCAN"
echo "============================================"

echo ""
echo "--- USB tablet detection ---"
grep -c "usb\.mouse\.detect.*ok=1" "$LOG" 2>/dev/null || echo "0"
grep "usb\.mouse\.detect" "$LOG" 2>/dev/null | head -3

echo ""
echo "--- Pointer producer begin ---"
grep "usb\.pointer\.producer\.begin" "$LOG" 2>/dev/null || echo "(not found)"

echo ""
echo "--- Pointer producer report (real USB data) ---"
COUNT_REPORT=$(grep -c "usb\.pointer\.producer\.report.*ok=1" "$LOG" 2>/dev/null || echo "0")
echo "Count: $COUNT_REPORT"
grep "usb\.pointer\.producer\.report" "$LOG" 2>/dev/null | head -10 || echo "(not found)"

echo ""
echo "--- Pointer producer to_input ---"
grep -c "usb\.pointer\.producer\.to_input.*ok=1" "$LOG" 2>/dev/null || echo "0"
grep "usb\.pointer\.producer\.to_input" "$LOG" 2>/dev/null | head -3 || echo "(not found)"

echo ""
echo "--- Pointer producer normalized ---"
grep -c "usb\.pointer\.producer\.normalized.*ok=1" "$LOG" 2>/dev/null || echo "0"
grep "usb\.pointer\.producer\.normalized" "$LOG" 2>/dev/null | head -5 || echo "(not found)"

echo ""
echo "--- Pointer producer shell ---"
grep -c "usb\.pointer\.producer\.shell.*ok=1" "$LOG" 2>/dev/null || echo "0"
grep "usb\.pointer\.producer\.shell" "$LOG" 2>/dev/null | head -3 || echo "(not found)"

echo ""
echo "--- Pointer producer click_drag ---"
grep -c "usb\.pointer\.producer\.click_drag.*ok=1" "$LOG" 2>/dev/null || echo "0"
grep "usb\.pointer\.producer\.click_drag" "$LOG" 2>/dev/null | head -3 || echo "(not found)"

echo ""
echo "--- Pointer producer done ---"
grep -c "usb\.pointer\.producer\.done.*ok=1" "$LOG" 2>/dev/null || echo "0"
grep "usb\.pointer\.producer\.done" "$LOG" 2>/dev/null | head -3 || echo "(not found)"

echo ""
echo "--- Interrupt IN timeouts ---"
grep -c "enum\.timeout" "$LOG" 2>/dev/null || echo "0"

echo ""
echo "--- Transfer Events with non-zero data ---"
grep -c "hid\.report\.nonzero" "$LOG" 2>/dev/null || echo "0"
grep "hid\.report\.nonzero" "$LOG" 2>/dev/null | head -5 || echo "(not found)"

echo ""
echo "--- Tablet activity (sexusb) ---"
grep -c "tablet\.active\|tablet\.live\|tablet\.raw" "$LOG" 2>/dev/null || echo "0"
grep "tablet\.active\|tablet\.live\|tablet\.raw" "$LOG" 2>/dev/null | head -5 || echo "(not found)"

# ---- Fault scan ----
echo ""
echo "============================================"
echo " FAULT SCAN"
echo "============================================"
echo ""

fault_patterns=(
    "#PF"
    "#GP"
    "panic"
    "KERNEL PANIC"
    "PAGE FAULT"
    "GENERAL PROTECTION"
    "fault\.kill"
    "null-jump"
    "IPC storm"
    "ring overflow"
    "usb_pointer FAIL"
    "usb_mouse FAIL"
    "normalizer FAIL"
    "pointer FAIL"
    "click FAIL"
    "drag FAIL"
)

FAULTS_FOUND=0
for pat in "${fault_patterns[@]}"; do
    count=$(grep -c "$pat" "$LOG" 2>/dev/null || echo "0")
    if [ "$count" -gt 0 ]; then
        echo "  WARNING: $count x '$pat'"
        FAULTS_FOUND=$((FAULTS_FOUND + count))
    fi
done
if [ "$FAULTS_FOUND" -eq 0 ]; then
    echo "  All fault patterns: 0 (clean)"
else
    echo "  TOTAL FAULTS: $FAULTS_FOUND"
fi

# ---- Gate scan ----
echo ""
echo "============================================"
echo " GATE SCAN"
echo "============================================"
echo ""

GATE_RESULT=0
"$GATE_SCRIPT" "$LOG" 2>&1 | grep -E "usb_pointer|usb_mouse|FINAL|PASS gates|FAIL gates|SKIP gates" || true
GATE_RESULT=${PIPESTATUS[0]}

echo ""
if [ "$FAULTS_FOUND" -eq 0 ] && [ "$GATE_RESULT" -eq 0 ]; then
    echo "[probe] RESULT: PASS (0 faults, gate scan clean)"
    exit 0
elif [ "$FAULTS_FOUND" -gt 0 ]; then
    echo "[probe] RESULT: FAIL (${FAULTS_FOUND} fault(s) detected)"
    exit 1
else
    echo "[probe] RESULT: PARTIAL (gate scan exit=$GATE_RESULT)"
    exit "$GATE_RESULT"
fi
