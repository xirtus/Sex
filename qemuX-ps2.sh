#!/usr/bin/env bash
# qemuX-ps2.sh — Spindle PS/2 dev proof (SPINDLE_STATE_V1)
#
# Uses i8042 PS/2 keyboard (not USB HID) for reliable key injection.
# Path: QMP send-key → i8042 → IRQ1 → kernel INPUT_RING → sexinput
#       → EV_KEY → silk-shell → Spindle commands
#
# Proves:
#   Scroll Lock → [shell.action.spindle] toggle
#   help / about / clear commands
#   F11 → [access.action.close]
#   no panic / #PF / #GP / fault.kill

set -e

QEMU_BIN="/home/xirtus_arch/Documents/microkernel/tools/qemu/build/qemu-system-x86_64"
ISO="/home/xirtus_arch/Documents/microkernel/sexos-v1.0.0.iso"
LOG="/tmp/spindle_ps2_verify.log"
QMP="/tmp/sexos-qmp-ps2.sock"
PID_FILE="/tmp/qemu-ps2.pid"

rm -f "$LOG" "$QMP" "$PID_FILE"

if [ ! -f "$QEMU_BIN" ]; then echo "Error: QEMU binary missing"; exit 1; fi
if [ ! -f "$ISO" ]; then echo "Error: ISO missing — run ./scripts/entrypoint_build.sh"; exit 1; fi

echo "[ps2] Starting QEMU with i8042 PS/2 keyboard..."
"$QEMU_BIN" \
    -M q35 \
    -m 512M \
    -cpu max,+pku \
    -cdrom "$ISO" \
    -device nec-usb-xhci,id=xhci \
    -device usb-tablet,bus=xhci.0 \
    -serial file:"$LOG" \
    -display sdl \
    -qmp unix:"$QMP",server,nowait \
    -no-reboot \
    2>/dev/null &

QEMU_PID=$!
echo $QEMU_PID > "$PID_FILE"
echo "[ps2] QEMU PID=$QEMU_PID"

cleanup() {
    echo "[ps2] Killing QEMU..."
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
}
trap cleanup EXIT

# Wait for QMP socket
echo "[ps2] Waiting for QMP socket..."
for i in $(seq 1 40); do
    sleep 0.5
    [ -S "$QMP" ] && break
    if [ $i -eq 40 ]; then echo "[ps2] Timeout waiting for QMP socket"; exit 1; fi
done
echo "[ps2] QMP socket ready"

# QMP helper: inject key with hold-time (ms) via Python
qmp_key() {
    local qcode="$1" hold_ms="${2:-400}"
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

# Wait for desktop
echo "[ps2] Waiting for desktop (up to 60s)..."
for i in $(seq 1 120); do
    sleep 0.5
    if grep -q "sexdisplay.cursor.draw\|silkbar.clock.send.*ss=[3-9]" "$LOG" 2>/dev/null; then
        echo "[ps2] Desktop ready (${i} half-seconds elapsed)"
        break
    fi
    if [ $i -eq 120 ]; then
        echo "[ps2] Timeout waiting for desktop"
        tail -20 "$LOG" 2>/dev/null
        exit 1
    fi
done
sleep 1

echo "[ps2] === KEY INJECTION SEQUENCE ==="

# Scroll Lock → ToggleSpindle (PS/2 scancode 0x46)
echo "[ps2] Scroll Lock → spindle.toggle"
qmp_key scroll_lock 800
sleep 1.2

# help Enter
echo "[ps2] help Enter"
for k in h e l p; do qmp_key "$k" 300; sleep 0.15; done
qmp_key ret 300
sleep 0.8

# about Enter
echo "[ps2] about Enter"
for k in a b o u t; do qmp_key "$k" 300; sleep 0.15; done
qmp_key ret 300
sleep 0.8

# clear Enter
echo "[ps2] clear Enter"
for k in c l e a r; do qmp_key "$k" 300; sleep 0.15; done
qmp_key ret 300
sleep 0.8

# F11 → AccessClose (PS/2 0x57)
echo "[ps2] F11 → AccessClose"
qmp_key f11 800
sleep 1.2

echo "[ps2] Injection complete."
sleep 1

echo ""
echo "=== SCAN RESULTS ==="
echo "--- PS/2 scancode path ---"
grep "ps2.scancode\|sexinput.ps2" "$LOG" 2>/dev/null | head -20 || echo "(none)"
echo "--- spindle/action markers ---"
grep "spindle\|shell.action\|access.action" "$LOG" 2>/dev/null | head -30 || echo "(none)"
echo "--- panic/fault ---"
grep "panic\|#PF\|#GP\|fault.kill\|EXCEPTION\|trap" "$LOG" 2>/dev/null | head -10 || echo "(none)"
echo "==="

echo ""
echo "=== PASS/FAIL ==="

SPINDLE=$(grep "shell.action.spindle" "$LOG" 2>/dev/null | head -1)
HELP=$(grep "spindle.command.help" "$LOG" 2>/dev/null | head -1)
ABOUT=$(grep "spindle.command.about" "$LOG" 2>/dev/null | head -1)
CLEAR=$(grep "spindle.command.clear" "$LOG" 2>/dev/null | head -1)
CLOSE=$(grep "access.action.close" "$LOG" 2>/dev/null | head -1)
PANIC=$(grep "panic\|fault.kill\|#PF\|#GP\|EXCEPTION" "$LOG" 2>/dev/null | head -1)
PS2=$(grep "sexinput.ps2.scancode" "$LOG" 2>/dev/null | head -1)

[ -n "$PS2"     ] && echo "PASS: PS/2 scancodes received:     $PS2"     || echo "FAIL: no PS/2 scancodes in sexinput"
[ -n "$SPINDLE" ] && echo "PASS: spindle toggle:               $SPINDLE" || echo "FAIL: no shell.action.spindle"
[ -n "$HELP"    ] && echo "PASS: help command:                 $HELP"    || echo "FAIL: no spindle.command.help"
[ -n "$ABOUT"   ] && echo "PASS: about command:                $ABOUT"   || echo "FAIL: no spindle.command.about"
[ -n "$CLEAR"   ] && echo "PASS: clear command:                $CLEAR"   || echo "FAIL: no spindle.command.clear"
[ -n "$CLOSE"   ] && echo "PASS: AccessClose:                  $CLOSE"   || echo "FAIL: no access.action.close"
[ -z "$PANIC"   ] && echo "PASS: no panic/fault"                         || echo "FAIL: panic/fault: $PANIC"
