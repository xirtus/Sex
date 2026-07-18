#!/usr/bin/env bash
# PERF_BISECTION_GATE_V1 — measurable slowness gate for git bisect.
#
# Parses a QEMU serial log (does NOT build or run QEMU) and classifies the
# run as GOOD / BAD(slow) / CHAPTER1_REGRESSION / UNTESTABLE.
#
# Usage:
#   scripts/perf_bisection_gate.sh <serial-log>
#   git bisect run scripts/perf_bisection_gate.sh logs/qemu-latest.log
#     (only meaningful if each bisect step rebuilds + reruns QEMU to refresh
#      the log first — wrap in a runner script for full automation)
#
# Exit codes (git-bisect semantics):
#   0   — GOOD (all thresholds met, Chapter 1 intact, no faults)
#   1   — BAD / SLOW (threshold exceeded or fault present)
#   2   — CHAPTER_1_REGRESSION (input chain markers missing)
#   125 — UNTESTABLE (log missing/empty, boot never reached PD spawn, or
#          commit predates trace instrumentation → bisect skip)
#
# BAD thresholds (env-overridable):
#   send_to_recv ratio      > MAX_SEND_TO_RECV      (default 2.0)
#   recv_to_draw ratio      > MAX_RECV_TO_DRAW      (default 2.0)
#   draw_to_present ratio   > MAX_DRAW_TO_PRESENT   (default 2.0)
#   max total_logical delta > MAX_INPUT_TO_PRESENT  (default 4, if measurable)
#   any fault (#PF/#GP/panic/fault.kill/reboot loop/freeze/storm)
set -u

LOG="${1:-logs/qemu-latest.log}"
MAX_SEND_TO_RECV="${MAX_SEND_TO_RECV:-2.0}"
MAX_RECV_TO_DRAW="${MAX_RECV_TO_DRAW:-2.0}"
MAX_DRAW_TO_PRESENT="${MAX_DRAW_TO_PRESENT:-2.0}"
MAX_INPUT_TO_PRESENT="${MAX_INPUT_TO_PRESENT:-4}"

# ── 125: untestable — no log ──
if [ ! -f "$LOG" ] || [ ! -s "$LOG" ]; then
    echo "PERF_BISECTION_GATE_V1: UNTESTABLE (log missing or empty: $LOG)"
    exit 125
fi

count_marker() {
    grep -cF "$1" "$LOG" 2>/dev/null || true
}

# key=value from LAST occurrence of a marker line.
last_field() {
    awk -v marker="$1" -v key="$2" '
        index($0, marker) {
            for (i = 1; i <= NF; i++) {
                split($i, kv, "=")
                if (kv[1] == key) val = kv[2]
            }
        }
        END { print val + 0 }
    ' "$LOG"
}

# ── 1. boot → all PDs spawned ──
spawn_begin=$(count_marker '[bootgraph.pd.spawn.begin]')
spawn_ok=$(count_marker '[bootgraph.pd.spawn.ok]')
timer_init=$(count_marker 'timer.init.done')
first_spawn_line=$(grep -nF '[bootgraph.pd.spawn.begin]' "$LOG" | head -1 | cut -d: -f1)
last_spawn_line=$(grep -nF '[bootgraph.pd.spawn.ok]' "$LOG" | tail -1 | cut -d: -f1)

# ── 125: untestable — boot never reached PD spawn ──
if [ "$spawn_ok" -eq 0 ] && [ "$timer_init" -eq 0 ]; then
    echo "PERF_BISECTION_GATE_V1: UNTESTABLE (no bootgraph.pd.spawn.ok and no timer.init.done — boot did not reach PD spawn)"
    exit 125
fi

# ── 2. scheduler / PD tick counts ──
sched_tick_enter=$(grep -c 'scheduler\.tick\.enter' "$LOG")
sched_yield=$(grep -c 'scheduler\.yield_and_switch\.saved' "$LOG")
sched_pick=$(grep -c 'scheduler\.pick_next' "$LOG")

# ── 3. sexusb transfer / rearm ──
transfer_events=$(count_marker '[sexusb.hid.transfer.event]')
rearms=$(count_marker '[sexusb.hid.rearm.ok]')

# ── 4/5. shell + display trace summaries (fallback: raw marker counts) ──
shell_summary=$(count_marker '[input.trace.shell.summary]')
display_summary=$(count_marker '[input.trace.display.summary]')
if [ "$shell_summary" -gt 0 ]; then
    applies=$(last_field '[input.trace.shell.summary]' 'applies')
    sends=$(last_field '[input.trace.shell.summary]' 'sends')
    shell_budget_hit=$(last_field '[input.trace.shell.summary]' 'budget_hit')
else
    applies=$(count_marker '[input.trace.shell.apply]')
    sends=$(count_marker '[input.trace.shell.cursor.send]')
    shell_budget_hit=na
fi
if [ "$display_summary" -gt 0 ]; then
    recv=$(last_field '[input.trace.display.summary]' 'recv')
    draws=$(last_field '[input.trace.display.summary]' 'draws')
    presents=$(last_field '[input.trace.display.summary]' 'presents')
    display_budget_hit=$(last_field '[input.trace.display.summary]' 'budget_hit')
else
    recv=$(count_marker '[input.trace.display.cursor.recv]')
    draws=$(count_marker '[input.trace.display.cursor.draw]')
    presents=$(count_marker '[input.trace.display.cursor.present]')
    display_budget_hit=na
fi

# ── 125: untestable — commit predates trace instrumentation ──
if [ "$sends" -eq 0 ] && [ "$recv" -eq 0 ] && [ "$draws" -eq 0 ]; then
    echo "PERF_BISECTION_GATE_V1: UNTESTABLE (no input.trace shell/display markers — commit predates INPUT_PRESENT_TICK_TRACE_V1 instrumentation)"
    exit 125
fi

# ── 6/7/8. ratios ──
ratio_send_to_recv=$(awk -v a="$sends" -v b="$recv" 'BEGIN { if (b > 0) printf "%.2f", a / b; else print "na" }')
ratio_recv_to_draw=$(awk -v a="$recv" -v b="$draws" 'BEGIN { if (b > 0) printf "%.2f", a / b; else print "na" }')
ratio_draw_to_present=$(awk -v a="$draws" -v b="$presents" 'BEGIN { if (b > 0) printf "%.2f", a / b; else print "na" }')

# ── 9. max logical input→present delta (seq-joined tick chains) ──
tick_stats=$(awk '
    function getf(key,    i, kv) {
        for (i = 1; i <= NF; i++) {
            split($i, kv, "=")
            if (kv[1] == key) return kv[2]
        }
        return "unknown"
    }
    /\[input\.trace\.shell\.apply\] seq=[0-9]/   { s = getf("seq"); apply_t[s] = getf("tick") }
    /\[input\.trace\.shell\.cursor\.send\] seq=[0-9]/ { s = getf("seq"); send_t[s] = getf("tick") }
    /\[input\.trace\.display\.cursor\.recv\] seq=[0-9]/ {
        s = getf("seq"); recv_dt[s] = getf("display_tick"); recv_st[s] = getf("shell_tick")
    }
    /\[input\.trace\.display\.cursor\.draw\] seq=[0-9]/ {
        s = getf("seq"); if (!(s in draw_dt)) draw_dt[s] = getf("display_tick")
    }
    /\[input\.trace\.display\.cursor\.present\] seq=[0-9]/ {
        s = getf("seq"); if (!(s in pres_dt)) pres_dt[s] = getf("display_tick")
    }
    END {
        n = 0
        for (s in apply_t) {
            if (!(s in send_t) || !(s in recv_dt) || !(s in draw_dt) || !(s in pres_dt)) continue
            if (recv_st[s] == "unknown") continue
            tot = (send_t[s] - apply_t[s]) + (draw_dt[s] - recv_dt[s]) + (pres_dt[s] - draw_dt[s])
            n++
            if (n == 1 || tot > max_tot) max_tot = tot
        }
        if (n > 0) printf "chains=%d max_total_logical=%d\n", n, max_tot
        else print "chains=0 max_total_logical=na"
    }
' "$LOG")
tick_chains=$(echo "$tick_stats" | sed -n 's/.*chains=\([0-9]*\).*/\1/p')
max_total_logical=$(echo "$tick_stats" | sed -n 's/.*max_total_logical=\(-\{0,1\}[0-9a-z]*\).*/\1/p')

# ── 10. serial log volume / marker budget ──
# PERF_LOG_NOISE_ABLATION_V1: noisy families print first 4 event lines then
# power-of-two [perf.noise.summary] lines. True event count = last summary
# count if present, else raw line count (pre-ablation logs).
noise_true_count() {
    # $1 = summary name, $2 = raw line count
    awk -v name="$1" -v raw="$2" '
        /\[perf\.noise\.summary\]/ {
            for (i = 1; i <= NF; i++) {
                split($i, kv, "=")
                if (kv[1] == "name" && kv[2] == name) matched = 1
                if (kv[1] == "count") c = kv[2]
            }
            if (matched) { last = c; matched = 0 }
        }
        END { print (last + 0 > raw + 0) ? last : raw }
    ' "$LOG"
}
total_lines=$(wc -l < "$LOG")
noise_bootframe=$(noise_true_count 'boot_frame.alloc' "$(count_marker '[kernel.mem.boot_frame.alloc]')")
noise_linen=$(noise_true_count 'linen.session.reject' "$(count_marker '[linen.session.reject]')")
noise_yield=$(noise_true_count 'scheduler.yield' "$sched_yield")

# ── 11. fault scan ──
# Kernel prints "EXCEPTION: PAGE FAULT" / "KERNEL PAGE FAULT HALT", not "#PF"
# (2026-07-02: a real kernel PF passed faultscan because of this).
pf=$(grep -Ec '#PF|PAGE FAULT' "$LOG")
gp=$(grep -c '#GP' "$LOG")
panic=$(grep -ci 'panic' "$LOG")
fkill=$(grep -c 'fault\.kill' "$LOG")
reboot_loop=$(grep -Eci 'reboot[_ -]?loop|reset[_ -]?loop|triple fault' "$LOG")
freeze=$(grep -Eci 'freeze=1|frozen=1|freeze\.detected|watchdog\.freeze|scheduler\.freeze|input\.freeze|runtime\.freeze' "$LOG")
shell_apply_raw=$(grep -c 'usb\.pointer\.shell\.apply' "$LOG")
storm=0
[ "$shell_apply_raw" -gt 5000 ] && storm=1
faults="pf=$pf gp=$gp panic=$panic fault_kill=$fkill reboot_loop=$reboot_loop freeze=$freeze storm=$storm"

# ── Chapter 1 chain markers (same set as input_control_quality_gate.sh) ──
pointer_emit=$(count_marker '[usb.hid.pointer.emit]')
pointer_move=$(count_marker '[input.pointer.move.ok]')
drag_begin=$(count_marker '[silk.drag.begin.ok]')
drag_move=$(count_marker '[silk.drag.move.ok]')
drag_end=$(count_marker '[silk.drag.end.ok]')

# ── Report ──
echo "== PERF_BISECTION_GATE_V1 =="
echo "[perf.gate.boot] pd_spawn_begin=$spawn_begin pd_spawn_ok=$spawn_ok spawn_line_span=${first_spawn_line:-na}..${last_spawn_line:-na} boot_to_all_pds_spawn=unavailable(no wall-clock marker)"
echo "[perf.gate.sched] tick_enter=$sched_tick_enter pick_next=$sched_pick yield_and_switch=$sched_yield"
echo "[perf.gate.usb] transfer_events=$transfer_events rearms=$rearms"
echo "[perf.gate.shell] applies=$applies sends=$sends budget_hit=$shell_budget_hit summaries=$shell_summary"
echo "[perf.gate.display] recv=$recv draws=$draws presents=$presents budget_hit=$display_budget_hit summaries=$display_summary"
echo "[perf.gate.ratio] send_to_recv=$ratio_send_to_recv recv_to_draw=$ratio_recv_to_draw draw_to_present=$ratio_draw_to_present"
echo "[perf.gate.latency] $tick_stats"
echo "[perf.gate.logvolume] total_lines=$total_lines boot_frame_alloc=$noise_bootframe linen_session_reject=$noise_linen scheduler_yield=$noise_yield"
echo "[perf.gate.faults] $faults"
echo "[perf.gate.thresholds] max_send_to_recv=$MAX_SEND_TO_RECV max_recv_to_draw=$MAX_RECV_TO_DRAW max_draw_to_present=$MAX_DRAW_TO_PRESENT max_input_to_present=$MAX_INPUT_TO_PRESENT"

# ── Exit 2: Chapter 1 regression ──
missing=""
[ "$transfer_events" -eq 0 ] && missing="$missing sexusb.hid.transfer.event"
[ "$rearms" -eq 0 ] && missing="$missing sexusb.hid.rearm.ok"
[ "$pointer_emit" -eq 0 ] && missing="$missing usb.hid.pointer.emit"
[ "$shell_apply_raw" -eq 0 ] && missing="$missing usb.pointer.shell.apply"
[ "$pointer_move" -eq 0 ] && missing="$missing input.pointer.move.ok"
[ "$drag_begin" -eq 0 ] && missing="$missing silk.drag.begin.ok"
[ "$drag_move" -eq 0 ] && missing="$missing silk.drag.move.ok"
[ "$drag_end" -eq 0 ] && missing="$missing silk.drag.end.ok"
if [ -n "$missing" ]; then
    echo "PERF_BISECTION_GATE_V1: CHAPTER_1_REGRESSION (missing:$missing)"
    exit 2
fi

# ── Exit 1: BAD / slow ──
bad=""
if [ "$faults" != "pf=0 gp=0 panic=0 fault_kill=0 reboot_loop=0 freeze=0 storm=0" ]; then
    bad="$bad fault($faults)"
fi
ratio_bad() {
    # $1 = ratio value or "na", $2 = threshold; na is not BAD (unmeasurable)
    awk -v r="$1" -v t="$2" 'BEGIN { if (r == "na") exit 1; exit !(r + 0 > t + 0) }'
}
ratio_bad "$ratio_send_to_recv" "$MAX_SEND_TO_RECV" && bad="$bad send_to_recv($ratio_send_to_recv>$MAX_SEND_TO_RECV)"
ratio_bad "$ratio_recv_to_draw" "$MAX_RECV_TO_DRAW" && bad="$bad recv_to_draw($ratio_recv_to_draw>$MAX_RECV_TO_DRAW)"
ratio_bad "$ratio_draw_to_present" "$MAX_DRAW_TO_PRESENT" && bad="$bad draw_to_present($ratio_draw_to_present>$MAX_DRAW_TO_PRESENT)"
if [ "${tick_chains:-0}" -gt 0 ] && [ "$max_total_logical" != "na" ]; then
    if [ "$max_total_logical" -gt "$MAX_INPUT_TO_PRESENT" ]; then
        bad="$bad input_to_present($max_total_logical>$MAX_INPUT_TO_PRESENT)"
    fi
fi

if [ -n "$bad" ]; then
    echo "PERF_BISECTION_GATE_V1: BAD ($bad)"
    exit 1
fi

# ── 125: unmeasurable ratios must not report GOOD ──
# INPUT_CURSOR_DRAIN_COHERENCE_V1: a run where the shell sent cursor updates
# but the display recv/draw/present sample is empty (all ratios na) proves
# nothing — bisect must skip it, not bless it.
if [ "$ratio_send_to_recv" = "na" ] && [ "$ratio_recv_to_draw" = "na" ]; then
    echo "PERF_BISECTION_GATE_V1: UNTESTABLE (display trace sample empty — ratios unmeasurable)"
    exit 125
fi

echo "PERF_BISECTION_GATE_V1: GOOD"
exit 0
