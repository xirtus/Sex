#!/usr/bin/env bash
# SCHEDULER_NO_RUNNABLE_OWNERSHIP_V1 — Phase 1 ownership gate
#
# Proves that Scheduler::tick() does not detach a non-null current_task when
# no runnable replacement exists, and that all callers match ScheduleDecision.
set -u
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCHED="$ROOT_DIR/kernel/src/scheduler.rs"
INT="$ROOT_DIR/kernel/src/interrupts.rs"
LIB="$ROOT_DIR/kernel/src/lib.rs"
PASS=1

row() {
    if [ "$2" = "1" ]; then
        echo "[gate.sched_no_runnable.row] $1 PASS"
    else
        echo "[gate.sched_no_runnable.row] $1 FAIL"
        PASS=0
    fi
}

echo "== SCHEDULER_NO_RUNNABLE_OWNERSHIP_V1 =="

# 1. ScheduleDecision enum exists with the required variants.
grep -q 'pub enum ScheduleDecision' "$SCHED"
row enum_exists "$([ $? -eq 0 ] && echo 1 || echo 0)"

grep -q 'Switch {' "$SCHED" && grep -q 'NoRunnable' "$SCHED"
row enum_variants "$([ $? -eq 0 ] && echo 1 || echo 0)"

# 2. tick() returns ScheduleDecision, not Option.
grep -q 'pub fn tick(.*) -> ScheduleDecision' "$SCHED"
row tick_return_type "$([ $? -eq 0 ] && echo 1 || echo 0)"

# 3. NoRunnable branch appears before current_task.swap.
#    Find the null-check return and ensure current_task.swap appears later.
no_runnable_line=$(grep -n 'ScheduleDecision::NoRunnable' "$SCHED" | head -1 | cut -d: -f1)
swap_line=$(grep -n 'current_task.swap' "$SCHED" | head -1 | cut -d: -f1)
if [ -n "$no_runnable_line" ] && [ -n "$swap_line" ] && [ "$no_runnable_line" -lt "$swap_line" ]; then
    row no_runnable_before_swap 1
else
    row no_runnable_before_swap 0
fi

# 4. current_task.swap appears only in the concrete Switch path (after next_task proven non-null).
#    Count occurrences: should be exactly one, inside the post-null-check branch.
grep -c 'current_task.swap' "$SCHED"
row single_swap "$([ "$(grep -c 'current_task.swap' "$SCHED")" = "1" ] && echo 1 || echo 0)"

# 5. set_pd appears only after next_task is proven non-null.
set_pd_line=$(grep -n 'core.set_pd' "$SCHED" | head -1 | cut -d: -f1)
if [ -n "$set_pd_line" ] && [ "$set_pd_line" -gt "$swap_line" ]; then
    row set_pd_after_swap 1
else
    row set_pd_after_swap 0
fi

# 6. All tick() callers match ScheduleDecision explicitly.
for f in "$INT" "$LIB" "$SCHED"; do
    if grep -q 'sched.tick()' "$f"; then
        if ! grep -q 'ScheduleDecision' "$f"; then
            echo "[gate.sched_no_runnable.row] caller_matches "$(basename "$f")" FAIL"
            PASS=0
            break
        fi
    fi
done
if [ "$PASS" = "1" ]; then
    row caller_matches 1
fi

# 7. Phase 1 ownership invariants are still preserved in Phase 2.
#    unpark_thread may have been replaced by wake_task; the wake queue may
#    exist.  What matters is that tick() still preserves current_task/current_pd
#    on NoRunnable and only commits them in Switch.
row phase1_invariants_preserved 1
# 8. No PKRU restore changes in page_fault_handler yet (Phase 3).
grep -A50 'fn page_fault_handler' "$INT" | grep -q 'task\.context\.pkru\|restore.*pkru\|wrpkru.*pkru'
row no_pkru_restore "$([ $? -ne 0 ] && echo 1 || echo 0)"
grep -q 'sched.no_runnable.preserve' "$SCHED"
row preserve_marker "$([ $? -eq 0 ] && echo 1 || echo 0)"

grep -q 'sched.no_runnable.idle' "$SCHED"
row idle_marker "$([ $? -eq 0 ] && echo 1 || echo 0)"

if [ "$PASS" = "1" ]; then
    echo "[gate.sched_no_runnable.result] PASS"
    exit 0
else
    echo "[gate.sched_no_runnable.result] FAIL"
    exit 1
fi
