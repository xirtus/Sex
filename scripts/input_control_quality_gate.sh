#!/usr/bin/env bash
# INPUT_PRESENT_TICK_TRACE_V1 gate (supersedes INPUT_PRESENT_TRACE_V1).
# Usage: scripts/input_control_quality_gate.sh <serial-log>
#
# Measures the input chain plus the trace-only shell→display→draw→present
# correlation with logical tick deltas. The trace seq rides the unused high
# 32 bits of OP_SURFACE_UPDATE arg1 and the shell send tick rides the unused
# high 32 bits of arg2 for the cursor surface (no ABI layout change).
#
# Logical tick domains (no shared clock):
#   shell:   apply tick (per USB apply), send tick (per cursor send)
#   display: recv/draw/present ticks (per event)
# Deltas are logical iteration counts, not wall-clock time.
#
# Exit codes:
#   0 — tick trace sufficient for logical latency measurement, Chapter 1 intact
#   1 — MEASUREMENT_PARTIAL_STOP_FIRST (tick cannot cross shell→display)
#   2 — Chapter 1 chain regressed
set -u

LOG="${1:-}"
JUMP_THRESHOLD="${JUMP_THRESHOLD:-24}"

if [ -z "$LOG" ] || [ ! -f "$LOG" ]; then
    echo "usage: $0 <serial-log>"
    exit 2
fi

count_marker() {
    grep -cF "$1" "$LOG" 2>/dev/null || true
}

# Extract a key=value field from the LAST occurrence of a marker line.
last_field() {
    # $1 = marker, $2 = key
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

button_state() {
    awk '
        /\[input\.button\.down\.ok\]/ {
            down_count++
            if (is_down) duplicate_down++
            is_down = 1
        }
        /\[input\.button\.up\.ok\]/ {
            up_count++
            if (!is_down) release_without_down++
            is_down = 0
        }
        END {
            printf "down=%d up=%d duplicate_down=%d release_without_down=%d stuck_down=%d\n",
                down_count + 0, up_count + 0, duplicate_down + 0,
                release_without_down + 0, is_down + 0
        }
    ' "$LOG"
}

drag_jump_count() {
    awk -v threshold="$JUMP_THRESHOLD" '
        /\[silk\.drag\.move\.ok\]/ {
            dx = 0
            dy = 0
            for (i = 1; i <= NF; i++) {
                if ($i ~ /^dx=/) { dx = substr($i, 4) + 0 }
                if ($i ~ /^dy=/) { dy = substr($i, 4) + 0 }
            }
            if (dx < 0) dx = -dx
            if (dy < 0) dy = -dy
            if (dx > max_dx) max_dx = dx
            if (dy > max_dy) max_dy = dy
            if (dx > threshold || dy > threshold) jumps++
        }
        END {
            printf "max_dx=%d max_dy=%d jumps_gt_%d=%d\n",
                max_dx + 0, max_dy + 0, threshold, jumps + 0
        }
    ' "$LOG"
}

fault_scan() {
    pf=$(grep -c '#PF' "$LOG")
    gp=$(grep -c '#GP' "$LOG")
    panic=$(grep -ci 'panic' "$LOG")
    fkill=$(grep -c 'fault\.kill' "$LOG")
    reboot_loop=$(grep -Eci 'reboot[_ -]?loop|reset[_ -]?loop|triple fault' "$LOG")
    freeze=$(grep -Eci 'freeze=1|frozen=1|freeze\.detected|watchdog\.freeze|scheduler\.freeze|input\.freeze|runtime\.freeze' "$LOG")
    applies=$(grep -c 'usb\.pointer\.shell\.apply' "$LOG")
    storm=0
    if [ "$applies" -gt 5000 ]; then storm=1; fi
    printf "pf=%d gp=%d panic=%d fault_kill=%d reboot_loop=%d freeze=%d storm=%d\n" \
        "$pf" "$gp" "$panic" "$fkill" "$reboot_loop" "$freeze" "$storm"
}

# Per-seq logical latency: join apply/send/recv/draw/present by seq.
# apply_to_send  = send tick − apply tick        (shell tick domain)
# send_to_recv_lag = shell_tick − display_tick at recv (queue depth in sends)
# recv_to_draw   = draw display_tick − recv display_tick   (display domain)
# draw_to_present = present display_tick − draw display_tick (display domain)
# total = apply_to_send + recv_to_draw + draw_to_present
tick_deltas() {
    awk '
        function getf(key,    i, kv) {
            for (i = 1; i <= NF; i++) {
                split($i, kv, "=")
                if (kv[1] == key) return kv[2]
            }
            return "unknown"
        }
        /\[input\.trace\.shell\.apply\] seq=[0-9]/ {
            s = getf("seq"); apply_t[s] = getf("tick")
        }
        /\[input\.trace\.shell\.cursor\.send\] seq=[0-9]/ {
            s = getf("seq"); send_t[s] = getf("tick")
        }
        /\[input\.trace\.display\.cursor\.recv\] seq=[0-9]/ {
            s = getf("seq")
            recv_dt[s] = getf("display_tick"); recv_st[s] = getf("shell_tick")
        }
        /\[input\.trace\.display\.cursor\.draw\] seq=[0-9]/ {
            s = getf("seq")
            if (!(s in draw_dt)) draw_dt[s] = getf("display_tick")
        }
        /\[input\.trace\.display\.cursor\.present\] seq=[0-9]/ {
            s = getf("seq")
            if (!(s in pres_dt)) pres_dt[s] = getf("display_tick")
        }
        END {
            n = 0
            for (s in apply_t) {
                if (!(s in send_t) || !(s in recv_dt) || !(s in draw_dt) || !(s in pres_dt)) continue
                if (recv_st[s] == "unknown") continue
                a2s = send_t[s] - apply_t[s]
                lag = recv_st[s] - recv_dt[s]
                r2d = draw_dt[s] - recv_dt[s]
                d2p = pres_dt[s] - draw_dt[s]
                tot = a2s + r2d + d2p
                n++
                sum_a2s += a2s; if (a2s > max_a2s || n == 1) max_a2s = a2s
                sum_lag += lag; if (lag > max_lag || n == 1) max_lag = lag
                sum_r2d += r2d; if (r2d > max_r2d || n == 1) max_r2d = r2d
                sum_d2p += d2p; if (d2p > max_d2p || n == 1) max_d2p = d2p
                sum_tot += tot; if (tot > max_tot || n == 1) max_tot = tot
                if (samples < 6) {
                    printf "[input.trace.tick.sample] seq=%s apply_tick=%s send_tick=%s recv_shell_tick=%s recv_display_tick=%s draw_display_tick=%s present_display_tick=%s apply_to_send=%d send_to_recv_lag=%d recv_to_draw=%d draw_to_present=%d total_logical=%d\n",
                        s, apply_t[s], send_t[s], recv_st[s], recv_dt[s], draw_dt[s], pres_dt[s],
                        a2s, lag, r2d, d2p, tot
                    samples++
                }
            }
            if (n > 0) {
                printf "[input.trace.tick.delta] chains=%d apply_to_send_avg=%.2f apply_to_send_max=%d send_to_recv_lag_avg=%.2f send_to_recv_lag_max=%d recv_to_draw_avg=%.2f recv_to_draw_max=%d draw_to_present_avg=%.2f draw_to_present_max=%d total_logical_avg=%.2f total_logical_max=%d\n",
                    n, sum_a2s / n, max_a2s, sum_lag / n, max_lag,
                    sum_r2d / n, max_r2d, sum_d2p / n, max_d2p,
                    sum_tot / n, max_tot
            } else {
                print "[input.trace.tick.delta] chains=0"
            }
        }
    ' "$LOG"
}

# ── Chapter 1 chain markers ──
transfer_events=$(count_marker '[sexusb.hid.transfer.event]')
rearms=$(count_marker '[sexusb.hid.rearm.ok]')
pointer_emit=$(count_marker '[usb.hid.pointer.emit]')
pointer_recv=$(count_marker '[input.hid.pointer.recv]')
shell_apply=$(count_marker '[usb.pointer.shell.apply]')
pointer_move=$(count_marker '[input.pointer.move.ok]')
drag_begin=$(count_marker '[silk.drag.begin.ok]')
drag_move=$(count_marker '[silk.drag.move.ok]')
drag_end=$(count_marker '[silk.drag.end.ok]')
keydown=$(count_marker '[input.keyboard.keydown.ok]')
keyup=$(count_marker '[input.keyboard.keyup.ok]')

# ── Trace summary counters (last summary line = authoritative totals) ──
shell_summary_count=$(count_marker '[input.trace.shell.summary]')
display_summary_count=$(count_marker '[input.trace.display.summary]')
trace_applies=$(last_field '[input.trace.shell.summary]' 'applies')
trace_sends=$(last_field '[input.trace.shell.summary]' 'sends')
trace_drag_moves=$(last_field '[input.trace.shell.summary]' 'drag_moves')
trace_max_jump=$(last_field '[input.trace.shell.summary]' 'max_jump')
shell_budget_hit=$(last_field '[input.trace.shell.summary]' 'budget_hit')
trace_recv=$(last_field '[input.trace.display.summary]' 'recv')
trace_draws=$(last_field '[input.trace.display.summary]' 'draws')
trace_presents=$(last_field '[input.trace.display.summary]' 'presents')
display_budget_hit=$(last_field '[input.trace.display.summary]' 'budget_hit')

# Shared seq check: display recv/present markers with a numeric (non-unknown) seq.
seq_recv_numeric=$(grep -Ec '\[input\.trace\.display\.cursor\.recv\] seq=[0-9]+' "$LOG")
seq_present_numeric=$(grep -Ec '\[input\.trace\.display\.cursor\.present\] seq=[0-9]+' "$LOG")
seq_unknown=$(grep -Ec '\[input\.trace\.display\.cursor\.(recv|present)\] seq=unknown' "$LOG")

# Shared tick check: shell tick crossed to display (numeric shell_tick at recv).
tick_recv_numeric=$(grep -Ec '\[input\.trace\.display\.cursor\.recv\] seq=[0-9]+ shell_tick=[0-9]+' "$LOG")
tick_apply_numeric=$(grep -Ec '\[input\.trace\.shell\.apply\] seq=[0-9]+ tick=[0-9]+' "$LOG")
tick_send_numeric=$(grep -Ec '\[input\.trace\.shell\.cursor\.send\] seq=[0-9]+ tick=[0-9]+' "$LOG")

ratio_send_to_recv=$(awk -v a="$trace_sends" -v b="$trace_recv" 'BEGIN { if (b > 0) printf "%.2f", a / b; else print "na" }')
ratio_recv_to_draw=$(awk -v a="$trace_recv" -v b="$trace_draws" 'BEGIN { if (b > 0) printf "%.2f", a / b; else print "na" }')

faults="$(fault_scan)"
tick_report="$(tick_deltas)"
tick_chains=$(echo "$tick_report" | awk '/\[input\.trace\.tick\.delta\]/ { for (i=1;i<=NF;i++) { split($i,kv,"="); if (kv[1]=="chains") print kv[2] } }')
tick_chains="${tick_chains:-0}"

echo "== INPUT_PRESENT_TICK_TRACE_V1 =="
echo "[input.trace.chain] transfer_events=$transfer_events rearms=$rearms pointer_recv=$pointer_recv pointer_emit=$pointer_emit shell_apply=$shell_apply pointer_move=$pointer_move"
echo "[input.trace.shell] applies=$trace_applies sends=$trace_sends drag_moves=$trace_drag_moves max_jump=$trace_max_jump budget_hit=$shell_budget_hit summaries=$shell_summary_count"
echo "[input.trace.display] recv=$trace_recv draws=$trace_draws presents=$trace_presents budget_hit=$display_budget_hit summaries=$display_summary_count"
echo "[input.trace.ratio] send_to_recv=$ratio_send_to_recv recv_to_draw=$ratio_recv_to_draw"
echo "[input.trace.seq] recv_numeric=$seq_recv_numeric present_numeric=$seq_present_numeric unknown=$seq_unknown"
echo "[input.trace.tick] apply_numeric=$tick_apply_numeric send_numeric=$tick_send_numeric recv_shell_tick_numeric=$tick_recv_numeric chains=$tick_chains"
echo "$tick_report"
echo "[input.trace.drag] begin=$drag_begin move=$drag_move end=$drag_end $(drag_jump_count)"
echo "[input.trace.button] $(button_state)"
echo "[input.trace.keyboard] down=$keydown up=$keyup"

if echo "$faults" | grep -q 'pf=0 gp=0 panic=0 fault_kill=0 reboot_loop=0 freeze=0 storm=0'; then
    echo "[input.faultscan.ok] $faults"
    FAULT_OK=1
else
    echo "[input.faultscan.fail] $faults"
    FAULT_OK=0
fi

# ── Exit 2: Chapter 1 chain regression ──
if [ "$transfer_events" -eq 0 ] \
    || [ "$rearms" -eq 0 ] \
    || [ "$pointer_emit" -eq 0 ] \
    || [ "$shell_apply" -eq 0 ] \
    || [ "$pointer_move" -eq 0 ] \
    || [ "$drag_begin" -eq 0 ] \
    || [ "$drag_move" -eq 0 ] \
    || [ "$drag_end" -eq 0 ] \
    || [ "$FAULT_OK" -eq 0 ]; then
    echo "INPUT_PRESENT_TICK_TRACE_V1: CHAPTER1_REGRESSION"
    exit 2
fi

# ── Exit 0: tick trace sufficient for logical latency measurement ──
if [ "$shell_summary_count" -gt 0 ] \
    && [ "$display_summary_count" -gt 0 ] \
    && [ "$seq_recv_numeric" -gt 0 ] \
    && [ "$seq_present_numeric" -gt 0 ] \
    && [ "$trace_presents" -gt 0 ] \
    && [ "$tick_recv_numeric" -gt 0 ] \
    && [ "$tick_chains" -gt 0 ]; then
    echo "INPUT_PRESENT_TICK_TRACE_V1: PASS"
    exit 0
fi

echo "INPUT_PRESENT_TICK_TRACE_V1: MEASUREMENT_PARTIAL_STOP_FIRST"
[ "$seq_recv_numeric" -eq 0 ] && echo "[input.trace.missing] display cursor recv with numeric seq"
[ "$seq_present_numeric" -eq 0 ] && echo "[input.trace.missing] display cursor present with numeric seq"
[ "$tick_recv_numeric" -eq 0 ] && echo "[input.trace.missing] shell tick did not cross shell→display (arg2 high bits)"
[ "$tick_chains" -eq 0 ] && echo "[input.trace.missing] no seq with full apply/send/recv/draw/present tick chain"
[ "$shell_summary_count" -eq 0 ] && echo "[input.trace.missing] shell trace summary"
[ "$display_summary_count" -eq 0 ] && echo "[input.trace.missing] display trace summary"
exit 1
