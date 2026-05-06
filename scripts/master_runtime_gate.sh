#!/usr/bin/env bash
# master_runtime_gate.sh — SexOS Master Runtime Proof Gate V1.
#
# Builds, boots QEMU, captures serial log, and scans for:
#   - All 6 PDs spawn and run (sexdisplay, sexdrive, silk-shell, sexinput, silkbar, linen)
#   - Clock/silkbar liveness marker
#   - No fault.kill, no panic, no #PF/#GP
#   - Scheduler round-robins all PDs (task.running for each)
#
# Usage:
#   ./scripts/master_runtime_gate.sh [--skip-build] [--probe N] [--keep-log]
#
# Options:
#   --skip-build   Use existing ISO (sexos-v1.0.0.iso) instead of rebuilding
#   --probe N      Wait N seconds for runtime probe (default: 20)
#   --keep-log     Keep the serial log after gate run
#
# Returns:
#   0 if ALL gates PASS
#   1 if any gate FAILS
#
# See: docs/handoff/MASTER_RUNTIME_GATE_V1.md

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# ---- config ----
SKIP_BUILD=false
PROBE_SECONDS=20
KEEP_LOG=false
GATE_DIR="${GATE_DIR:-$ROOT_DIR/.gate_master}"
LOG="${LOG_PATH:-$GATE_DIR/serial.log}"
QEMU_PID=""
ISO="${ISO:-sexos-v1.0.0.iso}"
QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"

# ---- gate state ----
BUILD_GATE="FAIL"
SPAWN_GATE="FAIL"
CLOCK_GATE="FAIL"
SCHED_GATE="FAIL"
FAULT_GATE="FAIL"
FINAL_SCORE="RED_MASTER"

# ---- arg parse ----
while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-build) SKIP_BUILD=true; shift ;;
        --probe)      PROBE_SECONDS="$2"; shift 2 ;;
        --keep-log)   KEEP_LOG=true; shift ;;
        *)
            echo "error: unknown option: $1"
            echo "usage: $0 [--skip-build] [--probe N] [--keep-log]"
            exit 1 ;;
    esac
done

# ---- helpers ----
cleanup() {
    set +e
    if [[ -n "$QEMU_PID" ]] && kill -0 "$QEMU_PID" 2>/dev/null; then
        kill "$QEMU_PID" 2>/dev/null || true
        sleep 1
        kill -9 "$QEMU_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT INT TERM

has_marker() {
    local pattern="$1"
    grep -qE "$pattern" "$LOG" 2>/dev/null && echo 1 || echo 0
}

count_marker() {
    local pattern="$1"
    grep -cE "$pattern" "$LOG" 2>/dev/null || echo 0
}

print_gate() {
    local name="$1" state="$2"
    printf '  %-28s %s\n' "$name" "$state"
}

# ---- 1. BUILD ----
echo ""
echo "============================================"
echo " MASTER RUNTIME GATE V1"
echo "============================================"
echo ""

if [ "$SKIP_BUILD" = true ]; then
    echo "[gate] --skip-build: using existing ISO"
    if [ ! -f "$ISO" ]; then
        echo "[FAIL] ISO not found: $ISO"
        exit 1
    fi
    BUILD_GATE="SKIP"
else
    echo "[gate] BUILD phase..."
    if ./scripts/entrypoint_build.sh >/tmp/master_gate_build.log 2>&1; then
        BUILD_GATE="PASS"
        echo "[gate] BUILD: PASS"
    else
        BUILD_GATE="FAIL"
        echo "[gate] BUILD: FAIL"
        echo "[gate] Build log: /tmp/master_gate_build.log"
    fi
fi

# ---- 2. BOOT + PROBE ----
echo ""
echo "[gate] BOOT+PROBE phase (${PROBE_SECONDS}s)..."
mkdir -p "$GATE_DIR"
rm -f "$LOG" "$GATE_DIR/qmp.sock"

set +e
"$QEMU_BIN" \
    -M q35 \
    -m 512M \
    -cpu max,+pku \
    -cdrom "$ISO" \
    -device nec-usb-xhci,id=xhci \
    -device usb-tablet,bus=xhci.0 \
    -serial "file:$LOG" \
    -display none \
    -no-reboot \
    -no-shutdown &
QEMU_PID=$!
set -e

if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    echo "[FAIL] QEMU failed to start"
    exit 1
fi

echo "[gate] QEMU PID: $QEMU_PID"
echo "[gate] Log: $LOG"
sleep "$PROBE_SECONDS"

# Stop QEMU
if kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    sleep 1
fi

# ---- 3. SCAN LOG ----
echo ""
echo "[gate] SCAN phase..."

if [ ! -f "$LOG" ]; then
    echo "[FAIL] No serial log found at $LOG"
    FINAL_SCORE="RED_MASTER"
fi

LOG_LINES=$(wc -l < "$LOG" 2>/dev/null || echo 0)
echo "[gate] Log lines: $LOG_LINES"
echo ""

# --- SPAWN GATE: all 6 PDs must be spawned ---
echo "--- SPAWN CHECKS ---"

PD_LIST=(
    "sexdisplay"
    "sexdrive"
    "silk-shell"
    "sexinput"
    "silkbar"
    "linen"
)

MISSING_PD=""
for pd in "${PD_LIST[@]}"; do
    if [ "$(has_marker "Spawned PD.*${pd}")" -eq 1 ]; then
        print_gate "  v $pd" "spawned"
    else
        print_gate "  x $pd" "MISSING"
        MISSING_PD="$pd"
    fi
done

if [ -z "$MISSING_PD" ]; then
    SPAWN_GATE="PASS"
else
    SPAWN_GATE="FAIL"
fi
echo ""

# --- CLOCK GATE: silkbar clock must tick ---
echo "--- CLOCK CHECK ---"
CLOCK_COUNT=$(count_marker "silkbar\.clock\.send")
if [ "$CLOCK_COUNT" -ge 2 ]; then
    CLOCK_GATE="PASS"
    echo "  v silkbar.clock.send: ${CLOCK_COUNT} ticks"
else
    CLOCK_GATE="FAIL"
    echo "  x silkbar.clock.send: ${CLOCK_COUNT} ticks (need >=2)"
fi
echo ""

# --- SCHED GATE: all PDs must have task.running evidence ---
echo "--- SCHEDULER CHECK ---"
# PD ID mapping: sexdisplay=1, sexdrive=2, silk-shell=3, sexinput=4,
#                 sexusb=5(bonus), silkbar=6, linen=7
PD_ID_MAP=(
    "1:sexdisplay"
    "2:sexdrive"
    "3:silk-shell"
    "4:sexinput"
    "6:silkbar"
    "7:linen"
)
MISSING_RUNNING=""
for entry in "${PD_ID_MAP[@]}"; do
    pd_id="${entry%%:*}"
    pd_name="${entry##*:}"
    count=$(count_marker "task\.running.*pd_id=${pd_id}")
    if [ "$count" -ge 1 ]; then
        print_gate "  v $pd_name (PD $pd_id)" "running (${count}x)"
    else
        print_gate "  x $pd_name (PD $pd_id)" "NO task.running"
        MISSING_RUNNING="$pd_name"
    fi
done

if [ -z "$MISSING_RUNNING" ]; then
    SCHED_GATE="PASS"
else
    SCHED_GATE="FAIL"
fi
echo ""

# --- FAULT GATE: no panics, faults, kills ---
echo "--- FAULT CHECK ---"
FAULT_PATTERNS=(
    "panic"
    "PAGE FAULT"
    "#PF"
    "GENERAL PROTECTION"
    "#GP"
    "triple fault"
    "Triple fault"
    "fault\.kill"
    "FATAL"
)

FAULT_HIT=""
for pat in "${FAULT_PATTERNS[@]}"; do
    if [ "$(has_marker "$pat")" -eq 1 ]; then
        print_gate "  x pattern: $pat" "FOUND"
        FAULT_HIT="$pat"
    fi
done

if [ -z "$FAULT_HIT" ]; then
    FAULT_GATE="PASS"
    echo "  v No faults/panics detected"
else
    FAULT_GATE="FAIL"
fi
echo ""

# ---- 4. SCORE ----
echo "============================================"
echo " MASTER RUNTIME GATE V1 - RESULTS"
echo "============================================"
echo ""

print_gate "BUILD_GATE" "$BUILD_GATE"
print_gate "SPAWN_GATE" "$SPAWN_GATE"
print_gate "CLOCK_GATE" "$CLOCK_GATE"
print_gate "SCHED_GATE" "$SCHED_GATE"
print_gate "FAULT_GATE" "$FAULT_GATE"

if [ "$BUILD_GATE" = "PASS" ] || [ "$BUILD_GATE" = "SKIP" ]; then
    if [ "$SPAWN_GATE" = "PASS" ] && [ "$CLOCK_GATE" = "PASS" ] \
        && [ "$SCHED_GATE" = "PASS" ] && [ "$FAULT_GATE" = "PASS" ]; then
        FINAL_SCORE="GREEN_MASTER"
    elif [ "$SPAWN_GATE" = "PASS" ] && [ "$CLOCK_GATE" = "PASS" ] \
        && [ "$SCHED_GATE" = "PASS" ]; then
        FINAL_SCORE="YELLOW_MASTER"
    else
        FINAL_SCORE="RED_MASTER"
    fi
else
    FINAL_SCORE="RED_MASTER"
fi

echo ""
print_gate "FINAL_SCORE" "$FINAL_SCORE"
echo ""

# Marker summary
m_spawn_total=$(count_marker "Spawned PD")
m_clock_total=$CLOCK_COUNT
m_task_total=$(count_marker "task\.running")
m_fault_total=$(count_marker "panic|fault\.kill|#PF|#GP|FATAL")

# ---- 5. HANDOFF DOC ----
cat > docs/handoff/MASTER_RUNTIME_GATE_V1.md << MDEOF
# MASTER_RUNTIME_GATE_V1

- date: $(date -Iseconds)
- git commit: $(git rev-parse --short HEAD)
- log_path: $LOG
- probe_seconds: $PROBE_SECONDS
- qemu: $QEMU_BIN -M q35 -m 512M -cpu max,+pku -cdrom $ISO -device nec-usb-xhci,id=xhci -device usb-tablet,bus=xhci.0 -serial file:\$LOG -display none -no-reboot -no-shutdown

## Gate Results

| Gate | Status |
|------|--------|
| BUILD_GATE | $BUILD_GATE |
| SPAWN_GATE | $SPAWN_GATE |
| CLOCK_GATE | $CLOCK_GATE |
| SCHED_GATE | $SCHED_GATE |
| FAULT_GATE | $FAULT_GATE |
| **FINAL_SCORE** | **$FINAL_SCORE** |

## Marker Counts

- Spawned PDs: $m_spawn_total
- Clock ticks (silkbar.clock.send): $m_clock_total
- task.running total: $m_task_total
- Fault/panic hits: $m_fault_total

## PD Spawn Details

$(for pd in "${PD_LIST[@]}"; do
    if [ "$(has_marker "Spawned PD.*${pd}")" -eq 1 ]; then
        echo "- v $pd: spawned"
    else
        echo "- x $pd: MISSING"
    fi
done)

$(for entry in "${PD_ID_MAP[@]}"; do
    pd_id="${entry%%:*}"
    pd_name="${entry##*:}"
    count=$(count_marker "task\.running.*pd_id=${pd_id}")
    echo "- $pd_name (PD $pd_id): task.running ${count}x"
done)

## Clock Liveness

- silkbar.clock.send ticks: $m_clock_total
- Minimum required: 2

## Expected Pass Criteria

1. **BUILD_GATE**: ISO builds without errors (or --skip-build with existing ISO)
2. **SPAWN_GATE**: All 6 PDs have \`v Spawned PD\` markers
3. **CLOCK_GATE**: \`[silkbar.clock.send]\` appears at least twice
4. **SCHED_GATE**: Each PD has at least one \`task.running\` entry
5. **FAULT_GATE**: No \`panic\`, \`fault.kill\`, \`#PF\`, \`#GP\`, \`FATAL\` markers

## Fail Criteria

- Any gate above is FAIL
- QEMU fails to boot within probe window
- ISO missing when --skip-build used

## Run Command

\`\`\`
./scripts/master_runtime_gate.sh
\`\`\`

### Variants

\`\`\`
# Skip rebuild (use existing ISO)
./scripts/master_runtime_gate.sh --skip-build

# Custom probe duration (e.g., 30 seconds)
./scripts/master_runtime_gate.sh --probe 30

# Keep serial log after run
./scripts/master_runtime_gate.sh --keep-log
\`\`\`

## Known Notes

- sexusb (PD 5) is also present and running but excluded from the 6-PD gate requirement.
- purple-scanout module is loaded but is cosmetic/diagnostic only.
- QEMU is run headless (-display none); no window appears unless --display is overridden.
- Serial log is captured to $GATE_DIR/serial.log and preserved only with --keep-log.
MDEOF

HANDOFF_WROTE=1

echo "[gate] Wrote docs/handoff/MASTER_RUNTIME_GATE_V1.md"

# ---- 6. CLEANUP LOG ----
if [ "$KEEP_LOG" = false ]; then
    rm -f "$LOG"
    echo "[gate] Log cleaned (use --keep-log to preserve)"
fi

# ---- 7. EXIT ----
if [ "$FINAL_SCORE" = "GREEN_MASTER" ]; then
    echo "[gate] ALL CHECKS PASSED (GREEN)"
    exit 0
elif [ "$FINAL_SCORE" = "YELLOW_MASTER" ]; then
    echo "[gate] PARTIAL PASS (YELLOW) - review fault details"
    exit 1
else
    echo "[gate] GATE FAILED (RED)"
    exit 1
fi
