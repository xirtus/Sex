#!/usr/bin/env bash
# qemuX-autotest.sh — Automated USB-kbd HID injection proof.
#
# Uses xdotool to send X11 key events to the QEMU SDL window.
# SDL translates X11 events → QemuInputEvent → usb-kbd HID report.
# Requires: DISPLAY set, xdotool installed, ISO built.
#
# Verifies: sexusb.kbd.raw b2 != 0 (nonzero USB HID key report)
#
# Usage:
#   ./qemuX-autotest.sh
#   Scan: rg "kbd.raw|usb_kbd|spindle|AccessClose" /tmp/spindle_manual_verify.log

set -e

QEMU_BIN="/home/xirtus_arch/Documents/microkernel/tools/qemu/build/qemu-system-x86_64"
ISO="/home/xirtus_arch/Documents/microkernel/sexos-v1.0.0.iso"
LOG="/tmp/spindle_manual_verify.log"
QMP="/tmp/sexos-qmp-autotest.sock"
PID_FILE="/tmp/qemu-autotest.pid"

# Cleanup prior run
rm -f "$LOG" "$QMP" "$PID_FILE"

if [ ! -f "$QEMU_BIN" ]; then echo "Error: QEMU binary missing"; exit 1; fi
if [ ! -f "$ISO" ]; then echo "Error: ISO missing — run ./scripts/entrypoint_build.sh"; exit 1; fi
if ! command -v xdotool &>/dev/null; then echo "Error: xdotool not found"; exit 1; fi

echo "[autotest] Starting QEMU..."
SDL_VIDEODRIVER=x11 DISPLAY=:1 "$QEMU_BIN" \
    -M q35,i8042=off \
    -m 512M \
    -cpu max,+pku \
    -cdrom "$ISO" \
    -device nec-usb-xhci,id=xhci \
    -device usb-kbd,bus=xhci.0 \
    -device usb-tablet,bus=xhci.0 \
    -serial file:"$LOG" \
    -display sdl \
    -qmp unix:"$QMP",server,nowait \
    -no-reboot \
    2>/dev/null &

QEMU_PID=$!
echo $QEMU_PID > "$PID_FILE"
echo "[autotest] QEMU PID=$QEMU_PID"

cleanup() {
    echo "[autotest] Killing QEMU..."
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
}
trap cleanup EXIT

# Wait for Silk desktop — sexdisplay cursor or clock at ss>=3 means desktop up
echo "[autotest] Waiting for desktop (up to 60s)..."
for i in $(seq 1 120); do
    sleep 0.5
    if grep -q "sexdisplay.cursor.draw\|silkbar.clock.send.*ss=[3-9]" "$LOG" 2>/dev/null; then
        echo "[autotest] Desktop ready (${i} half-seconds elapsed)"
        break
    fi
    if [ $i -eq 120 ]; then
        echo "[autotest] Timeout waiting for desktop"
        echo "--- last 20 log lines ---"
        tail -20 "$LOG" 2>/dev/null
        exit 1
    fi
done

sleep 1

# Find QEMU SDL window (X11 mode via SDL_VIDEODRIVER=x11)
echo "[autotest] Locating QEMU window..."
WIN_ID=$(DISPLAY=:1 xdotool search --name "QEMU" 2>/dev/null | head -1)
if [ -z "$WIN_ID" ]; then
    WIN_ID=$(DISPLAY=:1 xdotool search --name "." 2>/dev/null | while read wid; do
        name=$(DISPLAY=:1 xdotool getwindowname "$wid" 2>/dev/null)
        echo "$wid $name"
    done | grep -i "QEMU" | awk '{print $1}' | head -1)
fi
if [ -z "$WIN_ID" ]; then
    echo "[autotest] FAIL: QEMU window not found — no xdotool target"
    DISPLAY=:1 xdotool search --name "." 2>/dev/null | while read wid; do
        name=$(DISPLAY=:1 xdotool getwindowname "$wid" 2>/dev/null)
        [ -n "$name" ] && echo "$wid: $name"
    done | head -10
    exit 1
fi
echo "[autotest] Window ID=$WIN_ID"

# QMP send-key: inject key directly into QEMU (bypass SDL/X11 keysym mapping).
# Uses hold-time (ms) so the USB poll sees a held key.
qmp_send_key() {
    local qcode="$1" hold_ms="${2:-1000}"
    python3 - "$QMP" "$qcode" "$hold_ms" <<'PYEOF'
import socket, sys
path, qcode, hold_ms = sys.argv[1], sys.argv[2], int(sys.argv[3])
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect(path)
s.recv(4096)
s.sendall(b'{"execute":"qmp_capabilities"}\n')
s.recv(4096)
cmd = ('{"execute":"send-key","arguments":{"keys":[{"type":"qcode","data":"' +
       qcode + '"}],"hold-time":' + str(hold_ms) + '}}\n').encode()
s.sendall(cmd)
s.recv(4096)
s.close()
PYEOF
}

# Focus window and send alphanumeric keys via X11.
# Hold each key for 1s so the USB poll catches the held-key state.
DISPLAY=:1 xdotool windowfocus --sync "$WIN_ID"
sleep 0.5

send_key() {
    local k="$1" hold="${2:-1.0}"
    DISPLAY=:1 xdotool keydown --window "$WIN_ID" "$k"
    sleep "$hold"
    DISPLAY=:1 xdotool keyup --window "$WIN_ID" "$k"
    sleep 0.3
}

# Verify nonzero report path with 'h' (HID 0x0B)
echo "[autotest] Injecting: h (hold 1s)"
send_key h 1.0

# Scroll Lock via QMP (xdotool keysym unreliable for special keys).
# HID 0x47 → sexinput hid_to_ps2 → PS/2 0x46 → silk-shell ToggleSpindle.
echo "[autotest] Injecting: scroll_lock via QMP (hold 1s)"
qmp_send_key scroll_lock 1000
sleep 1.5

# Re-focus window after QMP injection before xdotool keys.
DISPLAY=:1 xdotool windowfocus --sync "$WIN_ID"
sleep 0.3

# help Enter
echo "[autotest] Injecting: help Enter"
for k in h e l p; do send_key "$k" 0.8; done
send_key Return 0.8
sleep 1.0

# F11 → AccessClose (HID 0x44 → PS/2 0x57) via QMP.
echo "[autotest] Injecting: f11 via QMP (hold 1s)"
qmp_send_key f11 1000
sleep 1.5

echo "[autotest] Input sequence complete. Shutting down..."
sleep 1

echo ""
echo "=== SCAN RESULTS ==="
echo "--- kbd raw reports ---"
grep "kbd.raw\|kbd.recv\|usb_kbd" "$LOG" 2>/dev/null | head -40 || echo "(none)"
echo "--- spindle/action markers ---"
grep "spindle\|shell.action\|access.action\|shell.spindle" "$LOG" 2>/dev/null | head -20 || echo "(none)"
echo "--- panic/fault ---"
grep "panic\|#PF\|#GP\|fault.kill\|EXCEPTION\|trap" "$LOG" 2>/dev/null | head -10 || echo "(none)"
echo "==="

# Check: nonzero USB kbd report
NONZERO=$(grep "kbd.raw" "$LOG" 2>/dev/null | grep -v "b2=0x0 " | grep -v "b2=0x0$" | head -3)
if [ -n "$NONZERO" ]; then
    echo "[autotest] PASS: nonzero USB kbd report found:"
    echo "$NONZERO"
else
    echo "[autotest] FAIL: all kbd.raw reports show b2=0x0"
fi

# Check: Scroll Lock reached sexinput (HID 0x47)
SCROLL=$(grep "hid=0x47\|kbd.raw.*b2=0x47" "$LOG" 2>/dev/null | head -3)
if [ -n "$SCROLL" ]; then
    echo "[autotest] PASS: Scroll Lock HID 0x47 reached:"
    echo "$SCROLL"
else
    echo "[autotest] FAIL: Scroll Lock (b2=0x47) not seen in USB reports"
fi

# Check: EV_KEY forwarded to shell
EVKEY=$(grep "usb_kbd.evkey" "$LOG" 2>/dev/null | head -5)
if [ -n "$EVKEY" ]; then
    echo "[autotest] PASS: EV_KEY forwarded:"
    echo "$EVKEY"
else
    echo "[autotest] FAIL: no usb_kbd.evkey events"
fi
