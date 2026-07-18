#!/usr/bin/env bash
# BELL_ATTENTION_FIREWALL_V1 gate.
# Lane 1: notify+list round trip. Drives the real, input-reachable
#         apps/spindle "notify" command via QMP keyboard (ToggleSpindle =
#         Scroll Lock, same hotkey used by SPINDLE_LIVE_TERMINAL_GHOST_HISTORY_V1),
#         confirms sexbell accepts it, and that SilkBar's own periodic
#         OP_BELL_LIST poll (already live, ~2s cadence) observes it.
#         (Uses "notify" not "bell-test" — QEMU's "minus" qcode does not
#         reliably reach Spindle's line buffer, dropping the hyphen and
#         producing "belltest" instead of "bell-test"; "notify" has no
#         hyphen and exercises the identical OP_BELL_NOTIFY path.)
# Lane 2: spam budget enforcement. Fires "notify" faster than the
#         SPAM_MAX_PER_WINDOW=8 limit, confirms sexbell rejects the excess
#         with reason=spam_budget_exceeded instead of silently queueing it.
# Lane 3: whole-boot fault freedom (no KERNEL PAGE FAULT / DOUBLE FAULT).
#
# NOT covered (needs a driving command that doesn't exist in any live PD
# today — out of scope for a gate-script-only change, see
# docs/handoff/BELL_ATTENTION_FIREWALL_V1.md): muted-sender rejection,
# non-allowlisted OP_BELL_LIST/SUBSCRIBE caller denial, queue-full drop
# (BELL_QUEUE_CAPACITY=16, unreachable before the spam budget trips first
# with spindle's only sender), FullHidden privacy redaction (spindle's
# notify/bell-test hardcode privacy=0/Public, arg0=0x100). Fuller
# negative-path coverage needs new commands added to spindle/silk-shell.
#
# KNOWN FLAKY: Lane 2 (spam budget) sends ~70 rapid QMP keystrokes and, in
# 13/13 test attempts during development, triggered a pre-existing kernel
# scheduler fault (KERNEL PAGE FAULT HALT pd=8 rip=0xffffffff802005bc, or a
# GP FAULT/KERNEL PANIC variant at pd=6) before or during the burst —
# same signature already documented in
# docs/handoff/SCHEDULER_TICK_PD8_PF_FLAKE_V1.md from unrelated earlier
# work. Crash probability scales with sustained keyboard-input volume
# (Lane 1's single lighter command passes cleanly roughly half the time;
# Lane 2's longer burst essentially never completed cleanly). This is
# kernel-side, off-limits here, and NOT caused by anything in
# servers/sexbell or crates/sex-pdx — Lane 1 alone already proves the
# Bell-side F.1/F.2 changes didn't break the live notify/list path. Budget
# many retries for Lane 2, or treat its FAIL as inconclusive rather than a
# Bell regression until the scheduler flake itself is fixed.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

GATE_DIR="${GATE_DIR:-/tmp/sexos_bell_gate}"
mkdir -p "$GATE_DIR" "$ROOT_DIR/logs"
ISO="${ISO:-sexos-v1.0.0.iso}"
QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"
READY_TIMEOUT="${READY_TIMEOUT:-45}"
SETTLE_SECONDS="${SETTLE_SECONDS:-4}"
SKIP_BUILD="${SKIP_BUILD:-0}"

export SEXOS_PROOFS_DISABLED="${SEXOS_PROOFS_DISABLED:-1}"
export SEXOS_BELL_DELIVERY_PROOF=1

QEMU_PID=""
cleanup() {
  set +e
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null
    sleep 1
    kill -9 "$QEMU_PID" 2>/dev/null
  fi
}
trap cleanup EXIT INT TERM

has() { grep -qE "$1" "$2" 2>/dev/null; }
count_of() { grep -cE "$1" "$2" 2>/dev/null || true; }

wait_marker() { # pattern log timeout
  local deadline=$((SECONDS + $3))
  while (( SECONDS < deadline )); do
    has "$1" "$2" && return 0
    sleep 1
  done
  return 1
}

boot_lane() { # name log qmp_sock
  local name="$1" log="$2" qmp="$3"
  rm -f "$log" "$qmp"
  echo "[bell_gate] lane=$name boot"
  set +e
  "$QEMU_BIN" -M q35 -m 512M -cdrom "$ISO" \
    -serial "file:$log" \
    -qmp "unix:$qmp,server=on,wait=off" \
    -display none -no-reboot -no-shutdown &
  QEMU_PID=$!
  set -e
}

stop_lane() {
  set +e
  if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null
    sleep 1
    kill -9 "$QEMU_PID" 2>/dev/null
  fi
  QEMU_PID=""
  set -e
}

qmp_sendkey() { # sock qcode
  python3 - "$1" "$2" <<'PY'
import json, socket, sys, time, os
sock_path, key = sys.argv[1], sys.argv[2]
deadline = time.time() + 10
while not os.path.exists(sock_path):
    if time.time() > deadline:
        print(f"qmp socket never appeared: {sock_path}", file=sys.stderr)
        sys.exit(1)
    time.sleep(0.2)
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(5)
s.connect(sock_path)
def rd():
    buf = b""
    while b"\n" not in buf:
        buf += s.recv(4096)
    return buf
rd()
def cmd(c):
    s.sendall((json.dumps(c) + "\n").encode())
    rd()
cmd({"execute": "qmp_capabilities"})
cmd({"execute": "send-key", "arguments": {"keys": [{"type": "qcode", "data": key}]}})
time.sleep(0.15)
PY
}

# Types an ASCII string (lowercase letters, digits, '-') into the focused
# window, then presses Enter. One QMP round trip per keystroke, matching
# the pattern already proven live in SPINDLE_LIVE_TERMINAL_GHOST_HISTORY_V1.
qmp_type_line() { # sock ascii_string
  local sock="$1" text="$2"
  local ch qcode
  for (( i=0; i<${#text}; i++ )); do
    ch="${text:$i:1}"
    case "$ch" in
      -) qcode="minus" ;;
      [a-z0-9]) qcode="$ch" ;;
      *) qcode="spc" ;;
    esac
    qmp_sendkey "$sock" "$qcode"
  done
  qmp_sendkey "$sock" "ret"
}

FAULT_RE='KERNEL PAGE FAULT|DOUBLE FAULT|KERNEL PANIC|GP FAULT'

declare -A ROW

# ── BUILD ────────────────────────────────────────────────────────────────
if [[ "$SKIP_BUILD" == "1" ]]; then
  ROW[build]="SKIP"
elif ./scripts/entrypoint_build.sh >"$GATE_DIR/build.log" 2>&1; then
  ROW[build]="PASS"
else
  ROW[build]="FAIL"
fi

# ── LANE 1: notify + list round trip ─────────────────────────────────────
L1="$GATE_DIR/lane_notify.log"
if [[ "${ROW[build]}" != "FAIL" ]]; then
  boot_lane notify "$L1" "$GATE_DIR/qmp1.sock"
  if wait_marker '\[sexbell\.ready\]' "$L1" "$READY_TIMEOUT"; then
    sleep "$SETTLE_SECONDS"
    # Focus Spindle (ToggleSpindle = Scroll Lock), run bell-test.
    qmp_sendkey "$GATE_DIR/qmp1.sock" "scroll_lock" || true
    sleep 1
    qmp_type_line "$GATE_DIR/qmp1.sock" "notify" || true
    # Give SilkBar's ~2s poll cadence time to observe the new event.
    sleep 5
  fi
  stop_lane

  if has '\[spindle\.bell\.send\] command=notify .* status=0' "$L1" \
     && has '\[bell\.notify\.ok\]' "$L1" \
     && has '\[bell\.queue\.push\]' "$L1"; then
    ROW[bell_notify_accept]="PASS"
  else
    ROW[bell_notify_accept]="FAIL"
  fi

  if has '\[silkbar\.bell\.poll\.reply\] total=[1-9]' "$L1" \
     || has '\[bell\.list\.item\]' "$L1"; then
    ROW[bell_list_visible]="PASS"
  else
    ROW[bell_list_visible]="FAIL"
  fi
else
  ROW[bell_notify_accept]="SKIP"
  ROW[bell_list_visible]="SKIP"
fi

# ── LANE 2: spam budget enforcement ──────────────────────────────────────
L2="$GATE_DIR/lane_spam.log"
if [[ "${ROW[build]}" != "FAIL" ]]; then
  boot_lane spam "$L2" "$GATE_DIR/qmp2.sock"
  if wait_marker '\[sexbell\.ready\]' "$L2" "$READY_TIMEOUT"; then
    sleep "$SETTLE_SECONDS"
    qmp_sendkey "$GATE_DIR/qmp2.sock" "scroll_lock" || true
    sleep 1
    # SPAM_MAX_PER_WINDOW=8 per window (SPAM_WINDOW_TICKS=62 ticks — in
    # practice this window held across ~9 wall-clock seconds of typing in
    # testing, so send well past 8 to guarantee the reject actually fires
    # before the window rolls over.
    for i in $(seq 1 10); do
      qmp_type_line "$GATE_DIR/qmp2.sock" "notify" || true
    done
    sleep 3
  fi
  stop_lane

  if has '\[bell\.notify\.reject\] caller_pd=[0-9]+ reason=spam_budget_exceeded' "$L2"; then
    ROW[bell_spam_budget]="PASS"
  else
    ROW[bell_spam_budget]="FAIL"
  fi
else
  ROW[bell_spam_budget]="SKIP"
fi

# ── LANE 3: whole-boot fault freedom (both lanes above) ──────────────────
if [[ "${ROW[build]}" != "FAIL" ]]; then
  if ! has "$FAULT_RE" "$L1" && ! has "$FAULT_RE" "$L2"; then
    ROW[bell_fault_free]="PASS"
  else
    ROW[bell_fault_free]="FAIL"
  fi
else
  ROW[bell_fault_free]="SKIP"
fi

# ── SUMMARY ──────────────────────────────────────────────────────────────
echo ""
echo "=== BELL_GATE SUMMARY ==="
EXIT=0
for r in build bell_notify_accept bell_list_visible bell_spam_budget bell_fault_free; do
  printf '%-26s %s\n' "$r" "${ROW[$r]}"
  [[ "${ROW[$r]}" == "FAIL" ]] && EXIT=1
done
echo "lane logs: $GATE_DIR/lane_*.log"
exit "$EXIT"
