# INPUT_PRESENT_TRACE_V1 — Shell→Display→Draw Trace Correlation

Date: 2026-07-02
Result: **PASS**

## What changed

Trace-only correlation added. No kernel, sex-pdx ABI, HID format, PDX ABI,
sexusb, sexinput, drag/focus policy, rendering ownership, or framebuffer
bounds changes.

### Sequence transport (shared seq: YES, no ABI change)

Shell packs a monotonic `u32` trace sequence into the **unused high 32 bits
of `OP_SURFACE_UPDATE` (0xEB) arg1**, cursor surface only, in
`send_cursor_checked()` (`servers/silk-shell/src/main.rs`):

```
arg1 = (seq << 32) | (clamped_x as u32)
```

Safe because sexdisplay extracts x with `msg.arg1 as i32` (low-32
truncation), so every existing consumer sees identical x. Message layout
(3×u64 args), opcode, and semantics for all other senders are unchanged —
untagged senders leave high bits zero, which display reports as
`seq=unknown`. Seq increments only in `apply_rel_pointer()` (real USB
EV_REL path; synthetic boot movement uses the ABS path and is never tagged
`source=usb`).

### Markers

Shell (`servers/silk-shell/src/main.rs`):
- `[input.trace.shell.apply] seq= x= y= dx= dy= source=usb` (budget 32)
- `[input.trace.shell.cursor.send] seq= x= y=` (budget 32)
- `[input.trace.shell.summary] applies= sends= drag_moves= max_jump= budget_hit=`
  (unbounded counters; emitted at applies==4 then every 32 applies)

Display (`servers/sexdisplay/src/main.rs`):
- `[input.trace.display.cursor.recv] seq=N|unknown x= y=` (budget 32, in 0xEB cursor branch)
- `[input.trace.display.cursor.draw] seq=N|unknown x= y=` (budget 32, in `draw_cursor_z_top`)
- `[input.trace.display.present] seq=N|unknown x= y=` (budget 32; cursor pixels are
  written directly to the framebuffer with no flip, so draw completion IS present)
- `[input.trace.display.summary] recv= draws= presents= budget_hit=`
  (emitted at recv==4/%32 and presents==4/%32)

Gate: `scripts/input_control_quality_gate.sh` rewritten — reports chain
counts, shell/display trace totals, `send_to_recv` and `recv_to_draw`
ratios, seq numeric/unknown counts, drag/button/keyboard, fault scan.
Exit 0 = traceable; exit 1 = MEASUREMENT_PARTIAL_STOP_FIRST; exit 2 =
Chapter 1 regression.

## What can now be measured

- **Input-to-shell**: yes (`shell.apply seq=N`).
- **Shell-to-display**: yes (`shell.cursor.send seq=N` → `display.cursor.recv seq=N`,
  exact seq match proven: seq=1..N with matching x/y).
- **Display-to-draw**: yes (`display.cursor.draw seq=N`).
- **Input-to-pixel (ordering/coalescing)**: yes — same seq observed at
  apply→send→recv→draw→present (e.g. seq=2 and seq=18 full-chain in proof log).
- **Coalescing loss**: real data. Heavy stimulus run: sends=257 vs recv=64
  (send_to_recv=4.02) — display drain coalesces under load. Light run:
  send_to_recv=1.03 (near lossless). recv_to_draw ≈ 2–5 (multiple recv per redraw).

## What still CANNOT be measured

- **Wall-clock latency**: no shared monotonic tick/timestamp crosses PDs.
  Seq gives ordering and drop/coalesce ratios, not milliseconds. Adding a
  tick would need either a shared time source or another arg field — arg2
  high 32 bits are also unused (y in low 32) and could carry a low-res tick
  the same trace-only way, but shell has no monotonic clock today.
- Summary totals lag up to 31 applies (cadence emission); final counters are
  lower bounds at log end.
- Early draws/presents before first USB event report `seq=unknown` (honest —
  untagged boot-time sends).

## Proof (2026-07-02)

- Build: `./scripts/entrypoint_build.sh` PASS.
- Runtime: gate_0_2 QEMU lane (q35, nec-usb-xhci, usb-tablet, headless) +
  identical QMP stimulus, but injection waits for `[usb.xhci.enum.done]`
  +3s instead of fixed 4s sleep (fixed sleep flakes: keys land before input
  chain ready → keyboard markers missing; see Recurring Issues).
- `scripts/input_current_tier_gate.sh logs/qemu-latest.log`: **PASS** (all 14 markers).
- `scripts/input_control_quality_gate.sh logs/qemu-latest.log`: **PASS** exit 0.
- Fault scan: pf=0 gp=0 panic=0 fault_kill=0 reboot_loop=0 freeze=0 storm=0.

## Recurring issues (save for future runs)

1. **gate_0_2.sh fixed `sleep 4` pre-injection is flaky**: if boot is slow,
   QMP keys hit PS/2 before sexinput/shell ready → `[ps2.irq1.entry]` fires
   but no `[input.keyboard.keydown.ok]`. Fix: wait for
   `[usb.xhci.enum.done]` in the serial log, then +3s, then inject.
2. **QMP unix socket path limit**: GATE_DIR paths >108 bytes fail
   (`UNIX socket path is too long`). Use short dirs like `/tmp/ipt_vN`.

## Next smallest prompt

MISSION: INPUT_PRESENT_TICK_TRACE_V1 — extend the proven trace channel with a
coarse tick. Shell maintains a local monotonic counter (incremented per HID
drain iteration, no new time source), packs it into the unused high 32 bits
of OP_SURFACE_UPDATE arg2 (y stays in low 32, same `as i32` truncation
safety as seq-in-arg1). Display echoes tick in recv/draw/present markers.
Gate computes seq-matched apply→present tick deltas and reports
distribution. Trace-only; same DO-NOT-TOUCH list as INPUT_PRESENT_TRACE_V1.
STOP FIRST if arg2 high bits turn out to be consumed anywhere.
