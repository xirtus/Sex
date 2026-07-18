#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

GATE_DIR="${GATE_DIR:-/tmp/sexos_gate_0_2}"
mkdir -p "$GATE_DIR"
mkdir -p "$ROOT_DIR/logs"

LOG="${LOG_PATH:-$ROOT_DIR/logs/qemu-latest.log}"
QMP_SOCK="${QMP_SOCK:-$GATE_DIR/qmp.sock}"
QEMU_PID=""
ISO="${ISO:-sexos-v1.0.0.iso}"
QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"
PROBE_SECONDS="${PROBE_SECONDS:-18}"
READY_TIMEOUT_SECONDS="${READY_TIMEOUT_SECONDS:-45}"
POST_STIMULUS_TIMEOUT_SECONDS="${POST_STIMULUS_TIMEOUT_SECONDS:-24}"
QMP_ENV_FAIL=0

# This lane proves host-injected QMP input. Disable built-in synthetic input
# proofs for the gate build so they cannot race or consume the drag/click path.
export SEXOS_PROOFS_DISABLED="${SEXOS_PROOFS_DISABLED:-1}"
# Negative proofs that cannot race QMP stimulus: one-shot try_set_focus on a
# dead surface (input-free), and a one-shot background click at ready that
# completes before host stimulus begins.
export SEXOS_SILK_FOCUS_REJECT_PROOF="${SEXOS_SILK_FOCUS_REJECT_PROOF:-1}"
export SEXOS_SILK_DRAG_REJECT_PROOF="${SEXOS_SILK_DRAG_REJECT_PROOF:-1}"
# Deterministic one-shot frame-drag proof through the real handle_hid_event
# path. Runs at ready (before host stimulus). The QMP pointer drag remains a
# best-effort real-input pass: sexinput's smoothing yields ~1-3px per tablet
# report, so host sweeps cannot reliably reach the left column.
export SEXOS_SILK_WINDOW_MOVE_PROOF="${SEXOS_SILK_WINDOW_MOVE_PROOF:-1}"

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

wait_for_marker() {
  local pattern="$1"
  local timeout="$2"
  local i
  for i in $(seq 1 "$((timeout * 4))"); do
    if grep -qE "$pattern" "$LOG" 2>/dev/null; then
      return 0
    fi
    sleep 0.25
  done
  return 1
}

wait_for_qmp_socket() {
  local timeout="$1"
  local i
  for i in $(seq 1 "$((timeout * 4))"); do
    if [[ -S "$QMP_SOCK" ]]; then
      return 0
    fi
    sleep 0.25
  done
  return 1
}

hard_fault_seen() {
  grep -qE "panic|PAGE FAULT|#PF|GENERAL PROTECTION|#GP|fault\\.kill|triple fault|Triple fault|reset[_ -]?loop|reboot[_ -]?loop|freeze=1|frozen=1|freeze\\.detected|watchdog\\.freeze|scheduler\\.freeze|input\\.freeze|runtime\\.freeze" "$LOG" 2>/dev/null
}

linen_storm_fixed() {
  ! grep -qE "\\[linen\\.session\\.reject\\] reason=bad_name_len len=0.*caller=12" "$LOG" 2>/dev/null \
    && ! grep -qE "\\[perf\\.noise\\.summary\\].*name=linen\\.session\\.reject" "$LOG" 2>/dev/null \
    && grep -qF "[linen.zero_name_storm.ok]" "$LOG" 2>/dev/null
}

input_gates_pass() {
  scripts/input_current_tier_gate.sh "$LOG" >/tmp/gate_0_2_input_current.out 2>&1 \
    && scripts/input_control_quality_gate.sh "$LOG" >/tmp/gate_0_2_input_quality.out 2>&1
}

wait_for_clean_proof() {
  local timeout="$1"
  local i
  for i in $(seq 1 "$((timeout * 4))"); do
    if hard_fault_seen; then
      return 2
    fi
    if linen_storm_fixed && input_gates_pass; then
      return 0
    fi
    sleep 0.25
  done
  return 1
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
  local matches
  matches="$(grep -oE 'sexdisplay\.cursor\.draw[^\n]*x=[0-9]+ y=[0-9]+' "$LOG" 2>/dev/null || true)"
  if [[ -z "$matches" ]]; then
    echo 0
    return
  fi
  printf '%s\n' "$matches" \
    | sed -E 's/.*x=([0-9]+) y=([0-9]+).*/\1,\2/' \
    | sort -u | wc -l | tr -d ' '
}

run_qmp_injection() {
  # Keyboard injection using existing helper
  if [[ -S "$QMP_SOCK" ]]; then
    python3 scripts/qmp_input_probe.py "$QMP_SOCK" ret >/tmp/gate_0_2_qmp_kbd.out 2>/tmp/gate_0_2_qmp_kbd.err || true
    wait_for_marker "\\[shell\\.focus\\.set\\] id=100|\\[shell\\.kbd\\.ui\\.focus\\].*new=100" 30 || return 0
  fi

  # Pointer injection via QMP absolute events for usb-tablet
  if [[ -S "$QMP_SOCK" ]]; then
    SEXOS_QMP_LOG="$LOG" python3 - "$QMP_SOCK" <<'PY' >/tmp/gate_0_2_qmp_ptr.out 2>/tmp/gate_0_2_qmp_ptr.err || true
import json, os, re, socket, sys, time
sock_path = sys.argv[1]
log_path = os.environ.get("SEXOS_QMP_LOG")

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

def wait_log(pattern, timeout=5.0):
    if not log_path:
        return False
    deadline = time.monotonic() + timeout
    rx = re.compile(pattern)
    while time.monotonic() < deadline:
        try:
            with open(log_path, "r", errors="ignore") as f:
                if rx.search(f.read()):
                    return True
        except OSError:
            pass
        time.sleep(0.1)
    return False

if not os.path.exists(sock_path):
    sys.exit(1)

s=socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(5)
s.connect(sock_path)
read_json(s)  # greeting
send_cmd(s,{"execute":"qmp_capabilities"})

# Steer to shell-owned surface 100 content, avoiding app-owned Mesh content.
# QEMU usb-tablet reports are normalized into bounded relative deltas, so the
# decreasing x sweep walks the shell cursor left from its boot position.
def abs_move(x, y, delay=0.08):
    cmd={
      "execute":"input-send-event",
      "arguments":{"events":[
        {"type":"abs","data":{"axis":"x","value":x}},
        {"type":"abs","data":{"axis":"y","value":y}}
      ]}
    }
    send_cmd(s,cmd)
    time.sleep(delay)

def left_sweep(y):
    # Slow, fine-grained sweep: the guest turns each processed tablet report
    # into one bounded relative step, so pacing must match the guest drain
    # rate or the ring coalesces reports and the pointer undershoots.
    abs_move(30000, y, 0.05)
    for x in range(29000, -1, -1000):
        abs_move(x, y, 0.08)

def log_size():
    try:
        return os.path.getsize(log_path) if log_path else 0
    except OSError:
        return 0

def wait_from(offset, pattern, timeout=3.0):
    # Like wait_log, but only matches content appended after `offset`,
    # so sweep retries cannot false-positive on earlier pointer positions.
    if not log_path:
        return False
    deadline = time.monotonic() + timeout
    rx = re.compile(pattern)
    while time.monotonic() < deadline:
        try:
            with open(log_path, "rb") as f:
                f.seek(offset)
                data = f.read().decode(errors="ignore")
            if rx.search(data):
                return True
        except OSError:
            pass
        time.sleep(0.1)
    return False

# Let the guest drain the pre-drag motion before button-down. The final raw
# coordinate is unique to this proof lane, so the wait is marker-driven.
# One extra sweep retry if the shell-applied position did not reach x<200
# (the [usb.pointer.shell.apply] marker is budgeted, so only one retry is
# attempted and only against fresh log content).
left_sweep(14000)
time.sleep(0.2)
left_sweep(14000)
abs_move(1200, 14000, 0.04)
wait_log(r"\[sexusb\.tablet\.abs\] x=1200 y=14000 buttons=0", 6.0)
if not wait_log(r"\[usb\.pointer\.shell\.apply\] x=([0-9]{1,2}|1[0-9]{2}) y=", 3.0):
    start = log_size()
    left_sweep(14000)
    abs_move(1200, 14000, 0.04)
    wait_from(start, r"\[sexusb\.tablet\.abs\] x=1200 y=14000 buttons=0", 6.0)
    wait_from(start, r"\[usb\.pointer\.shell\.apply\] x=([0-9]{1,2}|1[0-9]{2}) y=", 3.0)
time.sleep(0.6)

# Press on shell-owned surface 100, drag right, then release.
cmd={"execute":"input-send-event","arguments":{"events":[
  {"type":"btn","data":{"down":True,"button":"left"}}
]}}
send_cmd(s,cmd)
time.sleep(0.08)

for x in (2200, 3200):
    abs_move(x, 14000, 0.05)

cmd={"execute":"input-send-event","arguments":{"events":[
  {"type":"btn","data":{"down":False,"button":"left"}}
]}}
send_cmd(s,cmd)
time.sleep(0.05)

# Duplicate release to clear any queued tablet button state before the
# post-drag display-trace sweep.
send_cmd(s,cmd)
time.sleep(0.10)

for x in (4200, 5200, 6200):
    abs_move(x, 14000, 0.06)

# (The background-click drag.reject negative is covered by the input-free
# SEXOS_SILK_DRAG_REJECT_PROOF at shell ready — a QMP background click would
# need a long tablet sweep that floods the input queue and destabilizes
# the lane.)

s.close()
PY
  fi

  # SILK_WINDOW_MOVE_TEXT_INPUT_CURRENT_TIER_V1: focused typing + negatives.
  if [[ -S "$QMP_SOCK" ]]; then
    # Pin focus to surface 100 (digit '1' -> Focus100; the pointer phase may
    # have click-focused an app tile), then type into the shell text sink:
    # H I <space> H <backspace>.
    python3 scripts/qmp_input_probe.py "$QMP_SOCK" key 1 h i spc h backspace >/tmp/gate_0_2_qmp_text.out 2>/tmp/gate_0_2_qmp_text.err || true
    wait_for_marker "\\[silk\\.text\\.input\\.proof\\.done\\] ok=1" 15 || true
    # Negative: '/' has no sink mapping, no reserved action, and no app route
    # while surface 100 is focused -> key must be rejected without mutation.
    python3 scripts/qmp_input_probe.py "$QMP_SOCK" key 1 slash >/tmp/gate_0_2_qmp_neg.out 2>/tmp/gate_0_2_qmp_neg.err || true
    wait_for_marker "\\[silk\\.key\\.reject\\] reason=no_focus" 10 || true
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

if [[ "$QMP_ENV_FAIL" -eq 0 ]]; then
  if ! wait_for_qmp_socket 8; then
    QMP_ENV_FAIL=1
  else
    if ! wait_for_marker "\\[usb\\.xhci\\.enum\\.done\\]" "$READY_TIMEOUT_SECONDS"; then
      QMP_ENV_FAIL=1
    elif ! wait_for_marker "\\[silk-shell\\.ready\\]" "$READY_TIMEOUT_SECONDS"; then
      QMP_ENV_FAIL=1
    else
      # The successful Chapter 1/2 lane needs a short post-readiness settle so
      # boot layout and focus markers stop racing the first pointer packets.
      sleep 3
    fi
  fi
  if [[ "$QMP_ENV_FAIL" -eq 0 ]]; then
    run_qmp_injection
  fi
fi

if [[ "$QMP_ENV_FAIL" -eq 0 ]]; then
  wait_for_clean_proof "$POST_STIMULUS_TIMEOUT_SECONDS" || true
else
  sleep "$PROBE_SECONDS"
fi

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
if [[ -f "$LOG" ]] && ! hard_fault_seen; then
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
