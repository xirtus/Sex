#!/usr/bin/env bash
# RSP0_RATCHET_REGRESSION_GATE_V1
# Guards the SCHEDULER_TICK_PD8_PF_FLAKE_V1 phase-3 fix: yield_and_switch must
# set TSS RSP0 = ctx.kstack_top + 168 (saved-frame TOP). ctx.kstack_top alone
# is the saved-frame BASE (rax slot) — using it ratchets the task's kernel
# stack down ~168 bytes per yield-path dispatch until interrupt frames spray
# the adjacent heap Task struct. See docs/handoff/SCHEDULER_TICK_PD8_PF_FLAKE_V1.md.
#
# Usage: scripts/rsp0_regression_gate.sh [serial-log]
#   Static source rows always run. Runtime rows run only if a log is given.
set -u
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT_DIR/kernel/src/scheduler.rs"
LOG="${1:-}"
PASS=1

row() { # name ok
    if [ "$2" = "1" ]; then
        echo "[gate.rsp0.row] $1 PASS"
    else
        echo "[gate.rsp0.row] $1 FAIL"
        PASS=0
    fi
}

echo "== RSP0_RATCHET_REGRESSION_GATE_V1 =="

# ── Static source rows ────────────────────────────────────────────────────
YIELD_BODY="$(awk '/pub unsafe fn yield_and_switch/,/^}/' "$SRC")"

# 1. yield_and_switch exists and has exactly one RSP0 assignment.
RSP0_LINES="$(printf '%s\n' "$YIELD_BODY" | grep -c 'update_tss_rsp0')"
row yield_rsp0_single_site "$([ "$RSP0_LINES" = "1" ] && echo 1 || echo 0)"

# 2. That assignment carries the +168 (frame top), not bare kstack_top.
printf '%s\n' "$YIELD_BODY" | grep -q 'update_tss_rsp0(x86_64::VirtAddr::new(kstack_top + 168))'
row yield_rsp0_plus_168 "$([ $? -eq 0 ] && echo 1 || echo 0)"
printf '%s\n' "$YIELD_BODY" | grep -qE 'update_tss_rsp0\(x86_64::VirtAddr::new\(kstack_top\)\)'
row yield_rsp0_no_bare_base "$([ $? -ne 0 ] && echo 1 || echo 0)"

# 3. The comment must name the trap: next is TaskContext, value is frame base.
printf '%s\n' "$YIELD_BODY" | grep -q 'saved-frame BASE'
row yield_rsp0_comment_names_base "$([ $? -eq 0 ] && echo 1 || echo 0)"

# ── Runtime rows (optional, need serial log) ──────────────────────────────
if [ -n "$LOG" ] && [ -f "$LOG" ]; then
    grep -q '\[scheduler\.pd8\.flake\.fix\.ok\]' "$LOG"
    row runtime_fix_marker "$([ $? -eq 0 ] && echo 1 || echo 0)"
    FAULTS=$(grep -cE 'KERNEL PAGE FAULT|DOUBLE FAULT|panic|fault\.kill' "$LOG")
    row runtime_zero_faults "$([ "$FAULTS" = "0" ] && echo 1 || echo 0)"
    TRIPS=$(grep -cE '\[sched\.steal\.reject|\[sched\.set_pd\.null\]' "$LOG")
    row runtime_zero_tripwires "$([ "$TRIPS" = "0" ] && echo 1 || echo 0)"
elif [ -n "$LOG" ]; then
    echo "[gate.rsp0.row] runtime_log_missing FAIL"
    PASS=0
fi

if [ "$PASS" = "1" ]; then
    echo "[gate.rsp0.result] PASS"
    exit 0
else
    echo "[gate.rsp0.result] FAIL"
    exit 1
fi
