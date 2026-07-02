# INPUT_PRESENT_TICK_TRACE_V1 — Logical Tick Deltas for Input→Present

Date: 2026-07-02
Result: **PASS**

## What changed

Trace-only logical ticks added on top of INPUT_PRESENT_TRACE_V1. No kernel,
sex-pdx ABI, HID format, PDX ABI, sexusb, sexinput, drag/focus policy,
framebuffer ownership, or shared-memory changes. Input behavior unchanged.

### Tick transport (shell tick crosses PDs: YES, no ABI change)

Shell packs its local monotonic **send tick into the unused high 32 bits of
`OP_SURFACE_UPDATE` (0xEB) arg2**, cursor surface only, in
`send_cursor_checked()` (`servers/silk-shell/src/main.rs`):

```
arg1 = (seq << 32)             | (clamped_x as u32)   // INPUT_PRESENT_TRACE_V1
arg2 = (shell_send_tick << 32) | (clamped_y as u32)   // this mission
```

### Truncation safety proof (arg2 high 32 bits)

1. Receiver: sexdisplay 0xEB branch extracts y with `msg.arg2 as i32`
   (low-32 truncation) — `let new_y = msg.arg2 as i32;` — so every existing
   consumer sees identical y. `slot.y = new_y` stores the truncated value.
2. Non-cursor 0xEB senders all pass small plain y values (shell window/static
   surface moves, `apps/kaleidoscope` sends `arg2=100`) — high bits already
   zero, behavior unchanged.
3. rg over servers/, apps/, crates/ shows no full-width consumer of 0xEB
   arg2; the only 0xEB arg2 read is the `as i32` at the branch head.
4. Untagged senders leave high bits zero → display reports
   `shell_tick=unknown` (0 is the sentinel, ticks start at 1).

### Tick domains (no shared clock — logical iterations only)

- Shell: `SHELL_APPLY_TICK` (per real USB pointer apply, EV_REL path only),
  `SHELL_SEND_TICK` (per cursor send, any source).
- Display: `DISPLAY_RECV_TICK` / `DISPLAY_DRAW_TICK` / `DISPLAY_PRESENT_TICK`
  (per cursor recv/draw/present event). Draw completion IS present (direct
  framebuffer write, no flip), so draw and present ticks advance together.

### Markers

Shell:
- `[input.trace.shell.apply] seq= tick= x= y= dx= dy= source=usb`
- `[input.trace.shell.cursor.send] seq= tick= x= y=`
- `[input.trace.shell.summary] applies= sends= drag_moves= max_jump= budget_hit=`

Display:
- `[input.trace.display.cursor.recv] seq=N|unknown shell_tick=N|unknown display_tick= x= y=`
- `[input.trace.display.cursor.draw] seq=N|unknown shell_tick=N|unknown display_tick= x= y=`
- `[input.trace.display.cursor.present] seq=N|unknown shell_tick=N|unknown display_tick= x= y=`
  (renamed from `[input.trace.display.present]`; gate updated)
- `[input.trace.display.summary] recv= draws= presents= budget_hit=`

Gate: `scripts/input_control_quality_gate.sh` joins apply/send/recv/draw/
present lines by seq and reports per-seq samples plus avg/max of:
`apply_to_send` (shell domain), `send_to_recv_lag` (shell_tick −
display_recv_tick at recv), `recv_to_draw` and `draw_to_present` (display
domain), `total_logical`. Exit 0 = tick trace measurable + Chapter 1 intact;
exit 1 = MEASUREMENT_PARTIAL_STOP_FIRST (tick failed to cross); exit 2 =
Chapter 1 regression.

## Measured logical deltas (proof run, light stimulus)

Sample lines:

```
[input.trace.tick.sample] seq=6  apply_tick=6  send_tick=7  recv_shell_tick=7  recv_display_tick=7  draw_display_tick=8  present_display_tick=8  apply_to_send=1 send_to_recv_lag=0 recv_to_draw=1   draw_to_present=0 total_logical=2
[input.trace.tick.sample] seq=19 apply_tick=19 send_tick=20 recv_shell_tick=20 recv_display_tick=20 draw_display_tick=10 present_display_tick=10 apply_to_send=1 send_to_recv_lag=0 recv_to_draw=-10 draw_to_present=0 total_logical=-9
[input.trace.tick.sample] seq=31 apply_tick=31 send_tick=32 recv_shell_tick=32 recv_display_tick=32 draw_display_tick=12 present_display_tick=12 apply_to_send=1 send_to_recv_lag=0 recv_to_draw=-20 draw_to_present=0 total_logical=-19
[input.trace.tick.delta] chains=3 apply_to_send_avg=1.00 apply_to_send_max=1 send_to_recv_lag_avg=0.00 send_to_recv_lag_max=0 recv_to_draw_avg=-9.67 recv_to_draw_max=1 draw_to_present_avg=0.00 draw_to_present_max=0 total_logical_avg=-8.67 total_logical_max=2
```

Interpretation (counter-domain deltas, not per-event queue time):
- **apply→send = 1 constant, zero jitter**: send tick leads apply tick by
  exactly one (one untagged boot-time cursor send before the first USB
  apply). Shell adds no logical latency.
- **send_to_recv_lag = 0 at every recv**: display receives sends in
  lockstep — shell_tick equals display_recv_tick at each recv. The PDX
  transport itself adds no backlog under light load (heavy-load coalescing
  4.02 was measured in INPUT_PRESENT_TRACE_V1).
- **recv→draw drift grows −1 → −10 → −20**: recv tick advances ~3× faster
  than draw tick (recv_to_draw ratio 2.91; recv=32 draws=11). By seq 31, 32
  recvs had produced only 12 draws — ~20 cursor updates coalesced into
  skipped redraws. **This is the entire input→pixel latency budget.**
- **draw→present = 0 always**: direct framebuffer write; present is free.

## First quantified bottleneck

Display redraw cadence: ~2.9 cursor recvs per draw (light stimulus), drift
of 20 recvs by seq 31. Everything upstream (shell apply, PDX send, display
recv) is lockstep. The next optimization target is the sexdisplay drain →
`needs_surface_redraw` → redraw scheduling path — drawing the cursor closer
to per-recv (or at least per-drain-batch tail with the LATEST position, which
it may already do — verify freshness before optimizing frequency).

Note: coalescing to the latest position is not automatically bad (fresh
position wins over stale intermediate draws). Before optimizing, prove
whether the drawn position is the freshest received at draw time.

## Files changed

- `servers/silk-shell/src/main.rs` — SHELL_APPLY_TICK/SHELL_SEND_TICK,
  arg2 packing, tick= in apply/send markers.
- `servers/sexdisplay/src/main.rs` — DISPLAY_RECV/DRAW/PRESENT_TICK,
  shell_tick extraction from arg2 high bits, shell_tick/display_tick in
  recv/draw/present markers, present marker renamed to cursor.present.
- `scripts/input_control_quality_gate.sh` — seq-joined tick delta report,
  renamed-marker regexes, tick-crossing exit conditions.
- `docs/handoff/INPUT_PRESENT_TICK_TRACE_V1.md` — this file.

## Proof (2026-07-02)

- Build: `./scripts/entrypoint_build.sh` PASS.
- Runtime: gate_0_2 QEMU lane (q35, nec-usb-xhci, usb-tablet, headless),
  enum.done-synced injection (+3s), keyboard `a` + pointer sweep + drag.
- `scripts/input_current_tier_gate.sh logs/qemu-latest.log`: **PASS** (14/14).
- `scripts/input_control_quality_gate.sh logs/qemu-latest.log`: **PASS** exit 0,
  chains=3, send_to_recv=1.03, recv_to_draw=2.91.
- Fault scan: pf=0 gp=0 panic=0 fault_kill=0 reboot_loop=0 freeze=0 storm=0.

## Recurring issues (save for future runs)

1. **End-of-run crash flake (PRE-EXISTING, not tick-trace)**: intermittently
   after long runs, `fault.kill user_null_jump pd=7 rip=0x0 err=0x14` →
   `EXCEPTION: Failed to forward #PF to sext` → `KERNEL PAGE FAULT HALT
   addr=0x58 rip=0xffffffff802002f8 pd=8`, preceded by a garbage scheduler
   line `scheduler.yield_and_switch.saved pd_id=1174408382`. Identical
   signature exists in the PRE-change INPUT_PRESENT_TRACE_V1 heavy run
   (`/tmp/ipt_v3/sexos-input.log`). Hit 1 of 2 tick-trace proof runs (first
   run failed fault scan; rerun clean). Root cause not yet investigated —
   candidates: pd=7 stack/jump corruption late in run, sext #PF forward
   path. Worth its own mission.
2. gate_0_2-style fixed `sleep 4` pre-injection remains flaky — keep
   enum.done-synced injection (wait for `[usb.xhci.enum.done]` +3s).
3. QMP unix socket path >108 bytes fails — use short dirs (`/tmp/ipt_tk`).

## Next smallest prompt

MISSION: CURSOR_DRAW_FRESHNESS_PROOF_V1 — prove whether coalesced cursor
draws use the freshest received position. Trace-only: at each cursor draw,
display already logs seq/shell_tick (last received) and x/y (drawn). Add a
gate check joining `display.cursor.draw` x/y against the LAST
`display.cursor.recv` x/y before it in the log: count draws where drawn
position != last received position (stale draws). If stale=0, coalescing is
already latest-wins and the optimization target shifts to draw cadence
(more draws per drain), not freshness. No server code changes expected
beyond (if needed) one extra field in the existing draw marker. Same
DO-NOT-TOUCH list as INPUT_PRESENT_TICK_TRACE_V1.
