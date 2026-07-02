# INPUT_CURSOR_DRAIN_COHERENCE_V1 — Handoff

Date: 2026-07-02
Baseline: branch `master` @ `167bf934`, dirty tree (this mission's edits uncommitted).
Build: `scripts/entrypoint_build.sh` PASS (via `gate_0_2.sh` BUILD_GATE).

## Result

**Interactive trackpad/mouse control quality went from ~2% to ~80%** (operator
assessment, SDL interactive session). This is the milestone this doc records.
The ~2% era is quantified in `INPUT_FIX_ROOT_CAUSE_V1.md`: pointer delivery
2/28, 1/12, 11/21 moves per run (sexusb single-TRB starvation + serial-spam
stalls). After the committed fixes (multi-TRB in flight `92d8a3d7`, bounded
rel deltas `f18ce45b`/`da5dd87a`, log noise ablation `f07eb9dc`, linen storm
`2d3e6e97`) plus this mission's uncommitted coalescing + fast path, the cursor
tracks hand motion reliably in interactive use.

Measured on the QMP proof lane (run 2, zero faults):

| metric | before (stabilize lane) | after |
|--------|------------------------:|------:|
| shell applies → cursor sends | 1:1 (per-event IPC) | 4:1 coalesced (1024 → 256) |
| send_to_recv (shell → display transport) | **4.00 BAD** | **1.00 OK** |
| display recv → draws → presents | sparse, boot-offset polluted | 256 → 128 → 128, draw=present lockstep |
| cursor repaint cost per move | full `redraw_surface_area()` composite | 8x16 save-under restore + redraw |
| PS/2 keyboard headless injection | dead (IRQ1 never fires) | **works** (`keydown.ok code=28 source=ps2`) |

## What changed (all uncommitted as of this doc)

### 1. silk-shell: coalesced cursor sends (`servers/silk-shell/src/main.rs`)
- Relative pointer applies mark `CURSOR_SEND_PENDING` instead of one
  `OP_SURFACE_UPDATE` IPC per HID event.
- Flush happens when the shell's message backlog drains
  (`pdx_try_listen_raw` returns None → `flush_pending_cursor_send()` before
  blocking) or after `CURSOR_SEND_APPLY_CAP = 4` applies — bounding staleness.
- Latest `POINTER_X/Y` always wins; intermediate positions never sent.
- Tick domain unified: `SHELL_APPLY_TICK` (per real USB apply) is the single
  shell tick; sends stamp the apply tick current at send time, packed into
  unused high 32 bits of arg2 (y stays in low 32 via `as i32` truncation).

### 2. sexdisplay: cursor save-under fast path (`servers/sexdisplay/src/main.rs`)
- `CURSOR_UNDER_SAVED` (8x16) captures framebuffer pixels beneath the cursor
  at draw time; a cursor-only cycle does restore + redraw of that rect instead
  of a full `redraw_surface_area()` composite.
- `draw_cursor_z_top(.., save_under)`: pass `false` from `redraw_top_strip`
  (repaints only y<50 — capturing there would poison the patch with cursor
  pixels); `true` everywhere the fb under the cursor is current.
- Same clip rules as the arrow write loop: never writes y<51 (SilkBar zone),
  bounds-checked against `total_pixels`.
- Draw/present counters advance **only when the drawn seq is new**
  (`LAST_COUNTED_DRAW_SEQ`) — redundant same-position repaints (clock strip,
  chrome) no longer inflate `DISPLAY_DRAW_TICK` with a boot-time offset.
  Display-side coalescing stays visible (N recvs → 1 new-seq draw = N:1).
- Trace summary cadence tightened %32 → %8 (coalescing made %32 stale).

### 3. kernel: IRQ1 spurious-path drain + EOI (`kernel/src/interrupts.rs`)
When the keyboard is uninitialized, the handler previously returned without
reading port 0x60 or sending EOI. **Either alone kills all future IRQ1
delivery**: an unread byte keeps the i8042 output line asserted (no more
edges), and a missing EOI leaves vector 0x21 in-service. Now drains + EOIs.
This closed root cause 3 of `INPUT_FIX_ROOT_CAUSE_V1` — QMP `sendkey` now
reaches the guest via PS/2 headless (`[input.keyboard.keydown.ok] source=ps2`),
removing the "interactive SDL required for keyboard proof" blocker.

### 4. sexinput: keyboard proof markers (`servers/sexinput/src/main.rs`)
Budgeted `[input.keyboard.keydown.ok]` / `[input.keyboard.keyup.ok]` with
`source=usb|ps2` on both send paths.

### 5. Gates hardened
- `input_control_quality_gate.sh`: minimum trace sample
  (`MIN_TRACE_RECV=8`, `MIN_TRACE_PRESENTS=4`, env-overridable) — QMP lane
  stops at first PASS, so ratios were computed on 4 recvs / 1 draw.
- `perf_bisection_gate.sh`: all-`na` display ratios → exit 125 UNTESTABLE
  (bisect must skip, not bless).
- **Faultscan bug fixed in all three gates** (`input_current_tier_gate.sh`,
  `input_control_quality_gate.sh`, `perf_bisection_gate.sh`): they grepped
  `#PF`, but the kernel prints `EXCEPTION: PAGE FAULT` / `KERNEL PAGE FAULT
  HALT` — a real kernel page fault passed faultscan clean this session.
  Pattern is now `#PF|PAGE FAULT`. (`gate_0_2.sh` was already correct.)
- `dev.sh`: `[qemu.input.args/usb/binding]` diagnostics.

## Honest limits / open blockers

### A. Scheduler::tick crash flake — now symbolized
Run 1 of the QMP lane died:
```text
EXCEPTION: PAGE FAULT at 0x68 (RIP: 0xffffffff80220a38, RSP: 0x4444446804e0, ERR: 0x0) pd=8
```
`addr2line -e target/x86_64-sex/release/sex-kernel -f -C 0xffffffff80220a38`
→ `Scheduler::tick`. Disassembly: `rcx = *(next_task+0xb8)` (=
`context.pd_ptr`) held garbage `0x10`; fault reading the pd field at +0x58
(0x10+0x58 = 0x68) inside the `core.set_pd(pd_ptr)` path. A corrupt/stale
task was on the runqueue at timer tick. Same family as the pre-existing
pd=7/pd=8 end-of-run flake (see `PERF_LOG_NOISE_ABLATION_V1.md`) — NOT
introduced by this mission (signature exists in pre-change logs). Rerun was
clean. **Separate kernel mission, STOP FIRST.**

### B. QMP lane drag proof currently FAILS — tablet abs stream anomaly
Chapter 1 gate FAILs on `silk.drag.begin/move/end.ok` only. Root of the
failure in run 2: the tablet abs stream did not match the lane stimulus.
Lane sweeps x=30000→0 at fixed y=14000, then presses at (1200,14000); the
log instead shows **1092 reports** (lane sent ~40 events) wandering around
x≈10900→14000, y≈13600→0 — axis-swapped/garbage-looking coordinates with
small per-report drift. Cursor ended at (646,94), press hit app surface 200,
drag correctly rejected `focused_not_shell_surface`. The earlier run (which
crashed) showed **correct** axis order (`x=1200 y=14000`) — the anomaly is
per-run intermittent. Prime suspect: sexusb multi-TRB in-flight change
(`92d8a3d7`) reading stale/partial report buffers on re-arm. **Next mission:
sexusb interrupt-IN ring/report-buffer audit. STOP FIRST (sexusb domain).**
Note: interactive relative-mode control (the 80% experience) is not visibly
affected; this blocks the *headless abs proof lane*, not daily driving.

### C. Misc
- `gate_0_2.sh` legacy summary still prints `FINAL_SCORE: RED_0_2` from
  stale marker names — known, documented in `QMP_INPUT_PROOF_LANE_STABILIZE_V1.md`.
- QMP unix socket path >108 bytes fails silently into BOOT_GATE FAIL — use
  short `/tmp` dirs for `GATE_DIR`.
- `[input.trace.tick.sample]` cross-domain deltas (`recv_to_draw=-4`) are
  logical-tick artifacts when budgets cap marker emission mid-run; per-domain
  ratios are the trustworthy numbers.

## Proof commands

```text
GATE_DIR=/tmp/qmp_lane_cdc_v2 PROBE_SECONDS=18 POST_STIMULUS_TIMEOUT_SECONDS=24 ./scripts/gate_0_2.sh
scripts/input_current_tier_gate.sh logs/qemu-latest.log      # FAIL (drag markers only, blocker B)
scripts/input_control_quality_gate.sh logs/qemu-latest.log   # CHAPTER1_REGRESSION (same)
scripts/perf_bisection_gate.sh logs/qemu-latest.log          # ratios: 1.00 / 2.00 / 1.00
```

## Next smallest prompt

```text
MISSION: SEXUSB_ABS_REPORT_STREAM_AUDIT_V1 — PROVE TABLET REPORT BUFFER COHERENCE.

Goal:
Find why the usb-tablet abs report stream intermittently carries wrong/wandering
coordinates (1092 reports for ~40 QMP events, axis-swapped-looking values) since
multi-TRB in-flight (92d8a3d7). Suspect stale/partial report buffer reads on re-arm.

Allowed edits:
- servers/sexusb/src/main.rs (trace-only first pass: per-TRB buffer address,
  transfer-event residue, report byte dump budget)
- docs/handoff/SEXUSB_ABS_REPORT_STREAM_AUDIT_V1.md

Do not:
- change kernel
- change sex-pdx ABI
- change shell/display behavior
- "fix" the ring before the trace proves the corruption mechanism

Stop first if the fix requires xHCI ring redesign.
```
