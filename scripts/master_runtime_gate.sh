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
#   ./scripts/master_runtime_gate.sh [--skip-build] [--probe N] [--keep-log] [--nvme|--no-nvme]
#
# Options:
#   --skip-build   Use existing ISO (sexos-v1.0.0.iso) instead of rebuilding
#   --probe N      Wait N seconds for runtime probe (default: 20)
#   --keep-log     Keep the serial log after gate run
#   --no-nvme      Disable QEMU NVMe test device (default: enabled)
#   --nvme         Enable QEMU NVMe test device
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
PROBE_SECONDS=25
KEEP_LOG=false
ENABLE_NVME="${SEXOS_GATE_NVME:-1}"
PERSIST_REBOOT_PROOF="${SEXOS_PERSISTENCE_REBOOT_PROOF:-0}"
PRESERVE_NVME_IMG="${SEXOS_NVME_PRESERVE_IMG:-0}"
GATE_DIR="${GATE_DIR:-$ROOT_DIR/.gate_master}"
LOG="${LOG_PATH:-$GATE_DIR/serial.log}"
LOG_A="${GATE_DIR}/serial.boot_a.log"
LOG_B="${GATE_DIR}/serial.boot_b.log"
QEMU_PID=""
ISO="${ISO:-sexos-v1.0.0.iso}"
QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"

# ---- gate state ----
BUILD_GATE="FAIL"
SPAWN_GATE="FAIL"
CLOCK_GATE="FAIL"
SCHED_GATE="FAIL"
FAULT_GATE="FAIL"
SEXFILES_GATE="FAIL"
FINAL_SCORE="RED_MASTER"
SEXFILES_BOOT_PROOF="${SEXOS_SEXFILES_BOOT_PROOF:-0}"

# ---- arg parse ----
while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-build) SKIP_BUILD=true; shift ;;
        --probe)      PROBE_SECONDS="$2"; shift 2 ;;
        --keep-log)   KEEP_LOG=true; shift ;;
        --no-nvme)    ENABLE_NVME=0; shift ;;
        --nvme)       ENABLE_NVME=1; shift ;;
        *)
            echo "error: unknown option: $1"
            echo "usage: $0 [--skip-build] [--probe N] [--keep-log] [--nvme|--no-nvme]"
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
    local count
    count="$(grep -cE "$pattern" "$LOG" 2>/dev/null || true)"
    # grep -c returns exit=1 for zero matches but still prints "0";
    # normalize to a single numeric token for arithmetic tests.
    count="${count//$'\n'/}"
    if [[ -z "$count" ]]; then
        echo 0
    else
        echo "$count"
    fi
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
# Pre-create log path so --keep-log always leaves a deterministic file artifact.
: > "$LOG"

NVME_IMG="${GATE_DIR}/nvme.img"
NVME_ARGS=()
NVME_QEMU_DESC="(nvme disabled)"
if [ "$ENABLE_NVME" = "1" ]; then
    # Tiny deterministic probe image in gate dir.
    if [ "$PERSIST_REBOOT_PROOF" = "1" ] || [ "$PRESERVE_NVME_IMG" = "1" ]; then
        if [ ! -f "$NVME_IMG" ]; then
            dd if=/dev/zero of="$NVME_IMG" bs=512 count=2048 2>/dev/null || true
        fi
    else
        dd if=/dev/zero of="$NVME_IMG" bs=512 count=2048 2>/dev/null || true
    fi
    if "$QEMU_BIN" -device help 2>/dev/null | grep -qE 'name[[:space:]]+"nvme"[[:space:]]*,|(^|[[:space:],])nvme([[:space:],]|$)'; then
        NVME_ARGS=(
            -drive "if=none,id=nvm,file=${NVME_IMG},format=raw"
            -device "nvme,serial=sexos01,drive=nvm"
        )
        NVME_QEMU_DESC="-drive if=none,id=nvm,file=$NVME_IMG,format=raw -device nvme,serial=sexos01,drive=nvm"
    else
        echo "[FAIL] QEMU nvme device not supported by this QEMU build."
        echo "[hint] use --no-nvme (or SEXOS_GATE_NVME=0) for absent-path proof."
        exit 1
    fi
else
    echo "[gate] NVMe disabled; absent marker path expected."
fi

set +e
run_boot_once() {
    local boot_log="$1"
    "$QEMU_BIN" \
        -M q35 \
        -m 512M \
        -cpu max,+pku \
        -cdrom "$ISO" \
        -device nec-usb-xhci,id=xhci \
        -device usb-tablet,bus=xhci.0 \
        "${NVME_ARGS[@]}" \
        -serial "file:$boot_log" \
        -display none \
        -no-reboot \
        -no-shutdown &
    QEMU_PID=$!
}

if [ "$PERSIST_REBOOT_PROOF" = "1" ]; then
    : > "$LOG_A"
    : > "$LOG_B"
    run_boot_once "$LOG_A"
else
    run_boot_once "$LOG"
fi
set -e

if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    echo "[FAIL] QEMU failed to start"
    exit 1
fi

echo "[gate] QEMU PID: $QEMU_PID"
if [ "$PERSIST_REBOOT_PROOF" = "1" ]; then
    echo "[gate] Log A: $LOG_A"
else
    echo "[gate] Log: $LOG"
fi
sleep "$PROBE_SECONDS"

# Stop QEMU
if kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    sleep 1
fi

if [ "$PERSIST_REBOOT_PROOF" = "1" ]; then
    set +e
    run_boot_once "$LOG_B"
    set -e
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
        echo "[FAIL] QEMU failed to start (boot B)"
        exit 1
    fi
    echo "[gate] QEMU PID (boot B): $QEMU_PID"
    echo "[gate] Log B: $LOG_B"
    sleep "$PROBE_SECONDS"
    if kill -0 "$QEMU_PID" 2>/dev/null; then
        kill "$QEMU_PID" 2>/dev/null || true
        sleep 1
    fi
    LOG="$LOG_B"
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

# --- SEXFILES GATE: sexfiles must boot and emit ready marker ---
echo "--- SEXFILES CHECK ---"
SEXFILES_READY=$(has_marker '\[sexfiles\.ready\]')
SEXFILES_KSPAWN=$(has_marker '\[kernel\.spawn\.sexfiles\]')
SEXFILES_TASK_COUNT=$(count_marker 'task\.running.*pd_id=11')

if [ "$SEXFILES_READY" -eq 1 ]; then
    print_gate "  v sexfiles.ready" "FOUND"
else
    print_gate "  x sexfiles.ready" "MISSING"
fi

if [ "$SEXFILES_KSPAWN" -eq 1 ]; then
    print_gate "  v kernel.spawn.sexfiles" "FOUND"
else
    print_gate "  x kernel.spawn.sexfiles" "MISSING"
fi

if [ "$SEXFILES_TASK_COUNT" -ge 1 ]; then
    print_gate "  v sexfiles (PD 11)" "running (${SEXFILES_TASK_COUNT}x)"
else
    print_gate "  x sexfiles (PD 11)" "NO task.running"
fi

if [ "$SEXFILES_READY" -eq 1 ] && [ "$SEXFILES_TASK_COUNT" -ge 1 ]; then
    SEXFILES_GATE="PASS"
elif [ "$SEXFILES_BOOT_PROOF" != "1" ]; then
    SEXFILES_GATE="SKIP"
    echo "  (i) SEXOS_SEXFILES_BOOT_PROOF not set — sexfiles gate skipped"
else
    SEXFILES_GATE="FAIL"
fi
echo ""
echo "============================================"
echo " MASTER RUNTIME GATE V1 - RESULTS"
echo "============================================"
echo ""

print_gate "BUILD_GATE" "$BUILD_GATE"
print_gate "SPAWN_GATE" "$SPAWN_GATE"
print_gate "CLOCK_GATE" "$CLOCK_GATE"
print_gate "SCHED_GATE" "$SCHED_GATE"
print_gate "FAULT_GATE" "$FAULT_GATE"
print_gate "SEXFILES_GATE" "$SEXFILES_GATE"

if [ "$BUILD_GATE" = "PASS" ] || [ "$BUILD_GATE" = "SKIP" ]; then
    if [ "$SPAWN_GATE" = "PASS" ] && [ "$CLOCK_GATE" = "PASS" ] \
        && [ "$SCHED_GATE" = "PASS" ] && [ "$FAULT_GATE" = "PASS" ]; then
        if [ "$SEXFILES_GATE" = "PASS" ] || [ "$SEXFILES_GATE" = "SKIP" ]; then
            FINAL_SCORE="GREEN_MASTER"
        else
            FINAL_SCORE="YELLOW_MASTER"
        fi
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

# Emit typed block proof markers when requested so callers can pipe/grep stdout.
if [ "${SEXOS_SEXFILES_REAL_BLOCK_PROOF:-0}" = "1" ]; then
    echo "--- TYPED BLOCK PROOF MARKERS ---"
    grep -E 'sexfiles\.(realread|diskfs\.typed|block\.proof)\.|sexdrive\.block\.read\.api\.|sexdrive\.block\.typed|sexblock\.abi' "$LOG" || true
    echo ""
fi

if [ "$PERSIST_REBOOT_PROOF" = "1" ]; then
    echo "--- PERSISTENCE BOOT A MARKERS ---"
    grep -E 'sexfiles\.persistence\.boot_a|sexfiles\.persistence\.boot_b\.read_before_write\.mismatch|sexdrive\.nvme\.write\.' "$LOG_A" || true
    echo ""
    echo "--- PERSISTENCE BOOT B MARKERS ---"
    grep -E 'sexfiles\.persistence\.boot_b|sexdrive\.block\.read\.api|sexfiles\.diskfs\.typed\.read\.call' "$LOG_B" || true
    echo ""
fi

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
- nvme_enabled: $ENABLE_NVME
- nvme_img: $NVME_IMG
- qemu: $QEMU_BIN -M q35 -m 512M -cpu max,+pku -cdrom $ISO -device nec-usb-xhci,id=xhci -device usb-tablet,bus=xhci.0 $NVME_QEMU_DESC -serial file:\$LOG -display none -no-reboot -no-shutdown

## Gate Results

| Gate | Status |
|------|--------|
| BUILD_GATE | $BUILD_GATE |
| SPAWN_GATE | $SPAWN_GATE |
| CLOCK_GATE | $CLOCK_GATE |
| SCHED_GATE | $SCHED_GATE |
| FAULT_GATE | $FAULT_GATE |
| SEXFILES_GATE | $SEXFILES_GATE |
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

- sexfiles: SEXFILES_GATE=$SEXFILES_GATE ready=$SEXFILES_READY kspawn=$SEXFILES_KSPAWN

$(for entry in "${PD_ID_MAP[@]}"; do
    pd_id="${entry%%:*}"
    pd_name="${entry##*:}"
    count=$(count_marker "task\.running.*pd_id=${pd_id}")
    echo "- $pd_name (PD $pd_id): task.running ${count}x"
done)

- sexfiles (PD 11): task.running ${SEXFILES_TASK_COUNT}x

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
