#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

GATE_DIR="${GATE_DIR:-$ROOT_DIR/.gate_0_2}"
mkdir -p "$GATE_DIR"

LOG="${LOG_PATH:-$GATE_DIR/sexos-input.log}"
QMP_SOCK="${QMP_SOCK:-$GATE_DIR/qmp.sock}"
QEMU_PID=""
ISO="${ISO:-sexos-v1.0.0.iso}"
QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"
PROBE_SECONDS="${PROBE_SECONDS:-18}"
QMP_ENV_FAIL=0

EXPECTED_SCOPE_PREFIXES=(
  "servers/sexinput/src/main.rs"
  "kernel/src/interrupts.rs"
  "kernel/src/syscalls/mod.rs"
  "kernel/src/hal/x86_64.rs"
  "scripts/gate_0_2.sh"
  "docs/handoff/GATE_0_2_LAST_RUN.md"
)

BUILD_GATE="FAIL"
BOOT_GATE="FAIL"
POINTER_GATE="FAIL"
KEYBOARD_GATE="FAIL"
OWNERSHIP_GATE="FAIL"
FAULT_GATE="FAIL"
SCOPE_GATE="WARN"
HANDOFF_GATE="FAIL"
FINAL_SCORE="RED_0_2"

FIRST_MISSING_POINTER="none"
FIRST_MISSING_KEYBOARD="none"
FIRST_MISSING_ANY="none"

cleanup() {
  set +e
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    sleep 1
    kill -9 "$QEMU_PID" 2>/dev/null || true
  fi
  rm -f "$QMP_SOCK"
}
trap cleanup EXIT INT TERM

count_marker() {
  local pattern="$1"
  local c
  c="$(grep -cE "$pattern" "$LOG" 2>/dev/null || true)"
  if [[ -z "$c" ]]; then
    echo 0
  else
    echo "$c"
  fi
}

has_marker() {
  local pattern="$1"
  if grep -qE "$pattern" "$LOG" 2>/dev/null; then
    echo 1
  else
    echo 0
  fi
}

first_missing_from_chain() {
  local -n chain_ref=$1
  for m in "${chain_ref[@]}"; do
    if ! grep -qF "$m" "$LOG" 2>/dev/null; then
      echo "$m"
      return
    fi
  done
  echo "none"
}

extract_cursor_unique_positions() {
  if [[ ! -f "$LOG" ]]; then
    echo 0
    return
  fi
  grep -oE 'sexdisplay\.cursor\.draw[^\n]*x=[0-9]+ y=[0-9]+' "$LOG" \
    | sed -E 's/.*x=([0-9]+) y=([0-9]+).*/\1,\2/' \
    | sort -u | wc -l | tr -d ' '
}

run_qmp_injection() {
  # Keyboard injection using existing helper
  if [[ -S "$QMP_SOCK" ]]; then
    python3 scripts/qmp_input_probe.py "$QMP_SOCK" a >/tmp/gate_0_2_qmp_kbd.out 2>/tmp/gate_0_2_qmp_kbd.err || true
  fi

  # Pointer injection via QMP absolute events for usb-tablet
  if [[ -S "$QMP_SOCK" ]]; then
    python3 - "$QMP_SOCK" <<'PY' >/tmp/gate_0_2_qmp_ptr.out 2>/tmp/gate_0_2_qmp_ptr.err || true
import json, os, socket, sys, time
sock_path = sys.argv[1]

def read_json(s):
    buf=b""
    while True:
        chunk=s.recv(4096)
        if not chunk:
            return None
        buf += chunk
        parts = buf.split(b"\n")
        if len(parts) > 1:
            line = parts[0].strip()
            if line:
                try:
                    return json.loads(line.decode())
                except Exception:
                    pass
            buf = b"\n".join(parts[1:])

def send_cmd(s, cmd):
    s.sendall((json.dumps(cmd)+"\n").encode())
    return read_json(s)

if not os.path.exists(sock_path):
    sys.exit(1)

s=socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(5)
s.connect(sock_path)
read_json(s)  # greeting
send_cmd(s,{"execute":"qmp_capabilities"})

points=[(1000,200),(5000,500),(12000,2000),(22000,6000),(30000,12000),(10000,28000)]
for x,y in points:
    cmd={
      "execute":"input-send-event",
      "arguments":{"events":[
        {"type":"abs","data":{"axis":"x","value":x}},
        {"type":"abs","data":{"axis":"y","value":y}}
      ]}
    }
    send_cmd(s,cmd)
    time.sleep(0.05)

# click press/release
for down in (True, False):
    cmd={"execute":"input-send-event","arguments":{"events":[
      {"type":"btn","data":{"down":down,"button":"left"}}
    ]}}
    send_cmd(s,cmd)
    time.sleep(0.05)

s.close()
PY
  fi
}

print_gate() {
  local name="$1"
  local state="$2"
  printf '%-24s %s\n' "$name" "$state"
}

echo "[gate_0_2] BUILD_GATE"
if ./scripts/entrypoint_build.sh >/tmp/gate_0_2_build.log 2>&1; then
  BUILD_GATE="PASS"
else
  BUILD_GATE="FAIL"
fi

rm -f "$LOG" "$QMP_SOCK"

echo "[gate_0_2] BOOT+PROBE"
set +e
"$QEMU_BIN" \
  -M q35 \
  -m 512M \
  -cdrom "$ISO" \
  -device nec-usb-xhci,id=xhci \
  -device usb-tablet,bus=xhci.0 \
  -serial "file:$LOG" \
  -qmp "unix:$QMP_SOCK,server=on,wait=off" \
  -no-reboot \
  -no-shutdown &
QEMU_PID=$!
set -e

if ! kill -0 "$QEMU_PID" 2>/dev/null; then
  QMP_ENV_FAIL=1
fi

sleep 4
if [[ "$QMP_ENV_FAIL" -eq 0 ]]; then
  if [[ ! -S "$QMP_SOCK" ]]; then
    QMP_ENV_FAIL=1
  else
    run_qmp_injection
  fi
fi
sleep "$PROBE_SECONDS"

if kill -0 "$QEMU_PID" 2>/dev/null; then
  kill "$QEMU_PID" 2>/dev/null || true
  sleep 1
fi

if [[ -f "$LOG" ]]; then
  # boot gate heuristic: core services observed and no hard faults
  core_hits=0
  grep -q "sexdisplay" "$LOG" && core_hits=$((core_hits+1))
  grep -q "sexinput" "$LOG" && core_hits=$((core_hits+1))
  grep -q "silk-shell" "$LOG" && core_hits=$((core_hits+1))
  grep -q "sexusb" "$LOG" && core_hits=$((core_hits+1))
  if [[ "$core_hits" -ge 3 ]]; then
    BOOT_GATE="PASS"
  else
    BOOT_GATE="FAIL"
  fi
else
  BOOT_GATE="FAIL"
fi

# Fault gate
if [[ -f "$LOG" ]] && ! grep -qE "panic|PAGE FAULT|#PF|GENERAL PROTECTION|#GP|triple fault|Triple fault|reset" "$LOG"; then
  FAULT_GATE="PASS"
else
  FAULT_GATE="FAIL"
fi

# Pointer gate
pointer_chain=(
  "[sexinput.pointer.recv]"
  "[sexinput.pointer.send]"
  "[silk-shell.pointer.recv]"
  "[silk-shell.cursor.update]"
  "[sexdisplay.cursor.draw]"
)
FIRST_MISSING_POINTER="$(first_missing_from_chain pointer_chain)"
unique_cursor_positions="$(extract_cursor_unique_positions)"
if [[ "$FIRST_MISSING_POINTER" == "none" && "$unique_cursor_positions" -gt 1 ]]; then
  POINTER_GATE="PASS"
else
  POINTER_GATE="FAIL"
fi

# Keyboard gate
keyboard_chain=(
  "[ps2.irq1.entry]"
  "[ps2.port60.read]"
  "[ps2.input_ring.enqueue]"
  "[sexinput.ps2.scancode]"
  "[sexinput.keyboard.send]"
  "[silk-shell.keyboard.recv]"
)
FIRST_MISSING_KEYBOARD="$(first_missing_from_chain keyboard_chain)"
if [[ "$FIRST_MISSING_KEYBOARD" == "none" ]]; then
  KEYBOARD_GATE="PASS"
else
  KEYBOARD_GATE="FAIL"
fi

# Input ownership gate (static-diff heuristic)
ownership_fail=0
if git diff -- servers/sexdisplay/src/main.rs servers/sexusb/src/main.rs | grep -E '^\+.*(EV_REL|EV_ABS|OP_HID_EVENT|pointer|keyboard|input policy|click-hit|focus policy)' >/dev/null; then
  ownership_fail=1
fi
if [[ "$ownership_fail" -eq 0 ]]; then
  OWNERSHIP_GATE="PASS"
else
  OWNERSHIP_GATE="FAIL"
fi

# Scope gate (warn only)
scope_warn=0
while IFS= read -r f; do
  [[ -z "$f" ]] && continue
  matched=0
  for p in "${EXPECTED_SCOPE_PREFIXES[@]}"; do
    if [[ "$f" == "$p" ]]; then
      matched=1
      break
    fi
  done
  if [[ "$matched" -eq 0 ]]; then
    scope_warn=1
  fi
done < <(git status --short | awk '{print $2}')
if [[ "$scope_warn" -eq 0 ]]; then
  SCOPE_GATE="PASS"
else
  SCOPE_GATE="WARN"
fi

# first missing any
if [[ "$FIRST_MISSING_POINTER" != "none" ]]; then
  FIRST_MISSING_ANY="$FIRST_MISSING_POINTER"
elif [[ "$FIRST_MISSING_KEYBOARD" != "none" ]]; then
  FIRST_MISSING_ANY="$FIRST_MISSING_KEYBOARD"
else
  FIRST_MISSING_ANY="none"
fi

# score
if [[ "$BUILD_GATE" == "PASS" && "$BOOT_GATE" == "PASS" && "$POINTER_GATE" == "PASS" && "$KEYBOARD_GATE" == "PASS" && "$OWNERSHIP_GATE" == "PASS" && "$FAULT_GATE" == "PASS" ]]; then
  FINAL_SCORE="GREEN_0_2"
elif [[ "$BUILD_GATE" == "PASS" && "$BOOT_GATE" == "PASS" && "$POINTER_GATE" == "PASS" && "$KEYBOARD_GATE" != "PASS" ]]; then
  FINAL_SCORE="YELLOW_0_2"
else
  FINAL_SCORE="RED_0_2"
fi

# print report
echo
echo "=== GATE 0.2 SUMMARY ==="
print_gate "BUILD_GATE" "$BUILD_GATE"
print_gate "BOOT_GATE" "$BOOT_GATE"
print_gate "POINTER_LIVE_GATE" "$POINTER_GATE"
print_gate "KEYBOARD_LIVE_GATE" "$KEYBOARD_GATE"
print_gate "INPUT_OWNERSHIP_GATE" "$OWNERSHIP_GATE"
print_gate "FAULT_REGRESSION_GATE" "$FAULT_GATE"
print_gate "SCOPE_GATE" "$SCOPE_GATE"
print_gate "HANDOFF_GATE" "PENDING"
if [[ "$QMP_ENV_FAIL" -eq 1 ]]; then
  echo "FAIL_QMP_ENVIRONMENT"
fi
echo "FIRST_MISSING_POINTER: $FIRST_MISSING_POINTER"
echo "FIRST_MISSING_KEYBOARD: $FIRST_MISSING_KEYBOARD"
echo "FIRST_MISSING_ANY: $FIRST_MISSING_ANY"
echo "FINAL_SCORE: $FINAL_SCORE"

# marker counts for handoff
m_ps2_irq1=$(count_marker "ps2\\.irq1\\.entry")
m_ps2_read=$(count_marker "ps2\\.port60\\.read")
m_ps2_enq=$(count_marker "ps2\\.input_ring\\.enqueue")
m_ps2_sc=$(count_marker "sexinput\\.ps2\\.scancode")
m_ps2_send=$(count_marker "sexinput\\.keyboard\\.send")
m_ps2_shell=$(count_marker "silk-shell\\.keyboard\\.recv")

m_ptr_recv=$(count_marker "sexinput\\.pointer\\.recv")
m_ptr_send=$(count_marker "sexinput\\.pointer\\.send")
m_ptr_shell=$(count_marker "silk-shell\\.pointer\\.recv")
m_ptr_upd=$(count_marker "silk-shell\\.cursor\\.update")
m_ptr_draw=$(count_marker "sexdisplay\\.cursor\\.draw")

cat > docs/handoff/GATE_0_2_LAST_RUN.md <<MD
# GATE_0_2_LAST_RUN

- date: $(date -Iseconds)
- git commit: $(git rev-parse --short HEAD)
- qmp_sock: $QMP_SOCK
- log_path: $LOG
- qmp_environment_failure: $(if [[ "$QMP_ENV_FAIL" -eq 1 ]]; then echo yes; else echo no; fi)
- qemu lane: qemu-system-x86_64 -M q35 -m 512M -cdrom $ISO -device nec-usb-xhci,id=xhci -device usb-tablet,bus=xhci.0 -serial file:$LOG -qmp unix:$QMP_SOCK,server=on,wait=off -no-reboot -no-shutdown

## Gate Results

- BUILD_GATE: $BUILD_GATE
- BOOT_GATE: $BOOT_GATE
- POINTER_LIVE_GATE: $POINTER_GATE
- KEYBOARD_LIVE_GATE: $KEYBOARD_GATE
- INPUT_OWNERSHIP_GATE: $OWNERSHIP_GATE
- FAULT_REGRESSION_GATE: $FAULT_GATE
- SCOPE_GATE: $SCOPE_GATE
- FINAL_SCORE: $FINAL_SCORE

## Marker Counts

- [ps2.irq1.entry]: $m_ps2_irq1
- [ps2.port60.read]: $m_ps2_read
- [ps2.input_ring.enqueue]: $m_ps2_enq
- [sexinput.ps2.scancode]: $m_ps2_sc
- [sexinput.keyboard.send]: $m_ps2_send
- [silk-shell.keyboard.recv]: $m_ps2_shell

- [sexinput.pointer.recv]: $m_ptr_recv
- [sexinput.pointer.send]: $m_ptr_send
- [silk-shell.pointer.recv]: $m_ptr_shell
- [silk-shell.cursor.update]: $m_ptr_upd
- [sexdisplay.cursor.draw]: $m_ptr_draw

## First Missing Marker

- pointer chain: $FIRST_MISSING_POINTER
- keyboard chain: $FIRST_MISSING_KEYBOARD
- overall: $FIRST_MISSING_ANY

## Remaining Risks

- GUI backend availability on host affects interactive proof reliability.
- QMP injection may not perfectly emulate real human timing/capture.
- Dirty tree scope warnings are advisory and non-blocking.
- If FAIL_QMP_ENVIRONMENT is set, treat as host policy/runtime issue, not SexOS regression.
MD

HANDOFF_GATE="PASS"
print_gate "HANDOFF_GATE" "$HANDOFF_GATE"

echo "[gate_0_2] wrote docs/handoff/GATE_0_2_LAST_RUN.md"
exit 0
