# BOOTGRAPH_READINESS_V1

## BootGraph gate command

```bash
scripts/check_bootgraph_log.py /tmp/sexos.log
```

Integrated runtime gate path:

```bash
./scripts/master_runtime_gate.sh --probe 25 --keep-log
```

The runtime gate captures serial output and then runs `scripts/check_bootgraph_log.py <serial_log> --allow-fault` as the host-side BootGraph checker.

## Required pass output

For parser-only run:

- `BOOTGRAPH PASS`

For integrated runtime gate:

- `BOOTGRAPH_GATE: PASS`
- `CAP_GRANT_GATE: PASS`
- `ORDER_GATE: PASS`
- `CLOCK_GATE: PASS`
- `FINAL_SCORE: GREEN_MASTER` for full pass.

## Common failures

- `BOOTGRAPH FAIL: BOOTGRAPH_GATE ... missing ...ready`
  - A required `*.ready` marker is missing in the log, or appears before `*.init.start`.
- `BOOTGRAPH FAIL: CAP_GRANT_GATE ...`
  - Missing `[bootgraph.phase25.begin]` / `[bootgraph.phase25.complete]` or invalid ordering, or missing required A2 grant markers.
- `BOOTGRAPH FAIL: ORDER_GATE ...`
  - Sender `bootgraph.edge.send` appears before receiver `*.ready` or before `phase25.complete`.
- `CLOCK_GATE: PASS WARN: ...`
  - Clock chain is partially degraded (for example: send without recv, recv without redraw, repeated clock drops, or fb_live wait without live render).
- `FAULT_GATE: FAIL ...`
  - Fault patterns (`panic`, `fault.kill`, `#PF`, `#GP`) found in serial log (unless explicitly allowed).

## BootGraph V1 Kernel Markers

The kernel emits marker-only observability logs in `kernel/src/init.rs`:

- `[bootgraph.pd.spawn.begin] pd=<name>`
- `[bootgraph.pd.spawn.ok] pd=<name> id=<id> pkey=<pkey>`
- `[bootgraph.pd.spawn.err] pd=<name> reason=<reason>`
- `[bootgraph.phase25.begin]`
- `[bootgraph.cap.grant] from=kernel to=<pd> slot=<slot> target=<target> ok=1`
- `[bootgraph.cap.grant] from=kernel to=<pd> slot=<slot> target=<target> ok=0 optional=1 reason=<reason>`
- `[bootgraph.phase25.complete]`
- `[bootgraph.boot.handoff] target=<pd> id=<id> entry=<entry_addr>`

Proof command:

```bash
rg "bootgraph.pd.spawn|bootgraph.phase25|bootgraph.cap.grant|bootgraph.boot.handoff|fault.kill|#PF|#GP|panic" /tmp/sexos.log
```

## Clock Canary Markers

Added marker names for clock-freeze layer classification:

- `[silkbar.loop.cadence.start] iter=N`
- `[silkbar.loop.cadence.done] iter=N`
- `[sexdisplay.fb.live.wait] iter=N` (hard-budgeted)

Existing related marker names in this checkout:

- `[silkbar.clock.send]`
- `[silkbar.send_update.drop.clock]`
- `[sexdisplay.clock.recv]`
- `[sexdisplay.clock.redraw]`
- `[sexdisplay.render.live.ok]`

Canonical CLOCK_GATE chain (real PASS):

- `[silkbar.clock.send]` count >= 1
- `[sexdisplay.clock.recv]` count >= 1
- `[sexdisplay.clock.redraw]` count >= 1
- `[sexdisplay.render.live.ok]` count >= 1

Boot canary semantics:

- `[silkbar.clock.boot_canary] send=N threshold=T` proves early-boot accelerated cadence.
- Boot canary marker is advisory and not required forever once steady cadence is active.

Tick-based markers:

- Tick-indexed markers remain optional/advisory.
- Parser no longer emits stale warning solely because tick-indexed markers are absent when canonical chain passes.

## V2 Soft Barrier Marker Contract

V2 rollout edges:

- `silkbar -> sexdisplay` (`slot=5`, `SLOT_DISPLAY`, `op=OP_SILKBAR_UPDATE`)
- `sexinput -> silk-shell` (`slot=6`, `SLOT_SHELL`, `op=OP_HID_EVENT`)
- `sexusb -> sexinput` (`slot=9`, `SLOT_USB_SEXINPUT`, `op=HID_REPORT`)
- `silk-shell -> sexdisplay` (`slot=5`, `SLOT_DISPLAY`, `op=SURFACE_UPDATE`)
- `linen -> sexfiles` (`slot=1`, `SLOT_STORAGE`, `op=STORAGE_OP`)
- `quil -> sexfiles` (`slot=1`, `SLOT_STORAGE`, `op=DISKFS_OP`)

Soft-barrier defer marker:

- `[bootgraph.edge.defer from=silkbar to=sexdisplay slot=5 reason=missing_cap]`
- `[bootgraph.edge.defer from=sexinput to=silk-shell slot=6 reason=missing_cap]`
- `[bootgraph.edge.defer from=sexusb to=sexinput slot=9 reason=missing_cap]`
- `[bootgraph.edge.defer from=silk-shell to=sexdisplay slot=5 reason=missing_cap]`
- `[bootgraph.edge.defer from=linen to=sexfiles slot=1 reason=missing_cap]`
- `[bootgraph.edge.defer from=quil to=sexfiles slot=1 reason=missing_cap]`

Canonical first-send examples:

- `[bootgraph.edge.send from=silkbar to=sexdisplay slot=5 op=OP_SILKBAR_UPDATE first=1]`
- `[bootgraph.edge.send from=sexinput to=silk-shell slot=6 op=OP_HID_EVENT first=1]`
- `[bootgraph.edge.send from=sexusb to=sexinput slot=9 op=HID_REPORT first=1]`
- `[bootgraph.edge.send from=silk-shell to=sexdisplay slot=5 op=SURFACE_UPDATE first=1]`
- `[bootgraph.edge.send from=linen to=sexfiles slot=1 op=STORAGE_OP first=1]`
- `[bootgraph.edge.send from=quil to=sexfiles slot=1 op=DISKFS_OP first=1]`

Rules:

- No separate probe call is allowed; first `pdx_call_checked` remains the real send attempt.
- Canonical first-send marker grammar is fixed:
  `[bootgraph.edge.send from=<sender> to=<target> slot=<slot_num> op=<op_name> first=1]`
  (use numeric `slot`, not symbolic slot names).
- Rollout note: for `silk-shell -> sexdisplay`, only the first boot-critical display send is checked in V1.
  Broad conversion of all shell display calls is intentionally deferred.
- Rollout note: for `linen -> sexfiles`, central helper `pdx_storage_sync` is adapted to checked send.
  Broad per-call-site storage conversion is intentionally deferred.
- Rollout note: for `quil -> sexfiles`, central storage path (`pdx_storage_call -> pdx_call_and_reply(SLOT_STORAGE, ...)`) is adapted for edge markers.
  Broad per-call-site storage conversion is intentionally deferred.
- Emit at most one defer marker per boot per edge/slot.
- Defer before `phase25.complete` is informational/pass.
- Defer after `phase25.complete` is warning.
- Defer followed by normal `bootgraph.edge.send` is pass recovery.

### Storage Edge Boot-Probe Status (V2)

Storage edges are V2-installed, but boot proof is deferred to storage workload phase:

- `linen -> sexfiles`: `INSTALLED / UNEXERCISED / DEFERRED_TO_STORAGE_WORKLOAD`
- `quil -> sexfiles`: `INSTALLED / UNEXERCISED / DEFERRED_TO_STORAGE_WORKLOAD`

Reason:
- Current storage helpers can block waiting for reply-path messages.
- BootGraph must not introduce blocking-risk proof triggers in the 25s boot probe window.

Workload-phase proof markers (not required during 25s boot probe):

- `[bootgraph.edge.send from=linen to=sexfiles slot=1 op=STORAGE_OP first=1]`
- `[bootgraph.edge.send from=quil to=sexfiles slot=1 op=DISKFS_OP first=1]`

Guardrails:
- Do not add BootGraph-only storage send paths.
- Do not add timeout behavior to storage helpers as part of BootGraph rollout.
- Do not force blocking storage proof during boot probes.

Note: earlier "missing handoff path" note is superseded; `AGENT_HANDOFF_GP_CLOCK.md` may live under `docs/legacy/` in this checkout.

## SILKBAR_CLOCK_LIVENESS_RESTORED_V1

Runtime proof:
SilkBar clock advances past the old iter=2 / ss=2 freeze boundary.

Observed pass:
- SilkBar emits ss=1..6
- sexdisplay applies ss=1..6
- loop advances old=0→1 through old=5→6
- iter=2 reaches cadence.done and iter_advance old=2 new=3

Root lessons:
1. Do not trust budgeted markers for failing-iteration proof.
2. Place proof markers immediately after the state mutation they claim to prove.
3. Do not switch to long steady thresholds before boot liveness is proven.
4. Synthetic yield-based clock is a liveness canary, not a real wall-clock.

Remaining work:
Replace yield-count clock with a real monotonic timer source once timer calibration is ready.

## SILKBAR_SYNTHETIC_THRESHOLD_2_V1

Status:
QEMU synthetic fallback clock is tuned to threshold=2.

Runtime proof:
- `[silkbar.clock.synthetic.visible] threshold=2`
- `[silkbar.clock.source] kind=synthetic raw_ticks=0 threshold=2`
- SilkBar emits ss=1..13+
- sexdisplay applies ss=1..13+
- no `fault.kill`, `#PF`, `#GP`, or `panic` observed in proof grep

Notes:
- This is still synthetic QEMU/degraded liveness time, not wall-clock time.
- It is slightly fast but usable.
- Real tick path remains preferred when `sex_pdx::get_ticks()` advances.
- Default clock seed remains model/demo state and should be handled separately by real clock/RTC work.

## TIMER_BOOT_PHASE_GUARD_V1

Root cause:
Real LAPIC timer delivery can fire before the scheduler reaches `BootPhase::SchedulerRunning`.
Before this guard, the timer ISR could call `sched.tick()` during early boot phase 0 and hit
`SCHEDULER_RUNNING_VIOLATION`.

Fix:
Early timer IRQs now increment `TICKS`, optionally emit `[timer.tick.defer]`, send EOI, and
return until scheduler phase is running. Scheduler invariants remain enforced after that phase.

Runtime proof:
- `scheduler.tick.enter core=0 phase=4 rq_depth=12` repeats safely
- boot continues to userland
- SilkBar clock continues with synthetic fallback in QEMU
- no `SCHEDULER_RUNNING_VIOLATION`, kernel panic, `#PF`, or `#GP` observed

## SILKBAR_KVM_STALE_THRESHOLD_4_V1

Status:
KVM stale-real-tick fallback is tuned separately from TCG synthetic fallback.

Runtime behavior:
- TCG path: `raw_ticks=0`, synthetic threshold=2.
- KVM stale path: `raw_ticks=1` then stale, fallback threshold=4.
- KVM clock is still slightly fast but usable.
- TCG clock remains usable.
- No panic / `#PF` / `#GP` observed in proof runs.

Invariant:
Do not use one synthetic threshold for all degraded clock modes.
`raw_ticks==0` TCG fallback and `raw_ticks!=0 but stale` KVM fallback have different scheduler cadence behavior.

Remaining:
True wall-clock requires calibrated tick-rate export and RTC seed. That is separate STOP FIRST work.

## SILKSHELL_LINEN_SYNC_PRESERVE_INPUT_V1

Root cause:
`linen_sync_reply()` consumed and acked OP_HID_EVENT messages while waiting for Linen replies.
Pointer events arriving during Linen paint were lost before SilkShell cursor dispatch.

Fix:
`linen_sync_reply()` now handles OP_HID_EVENT inline while continuing to wait for Linen replies.
The pre-Linen non-blocking input drain remains in place.

Runtime proof:
- sexinput emits pointer movement
- `[silk-shell.linen_sync.input_hid]` receives class=2 movement
- sexdisplay receives cursor surface updates
- `sexdisplay.cursor.draw` moves away from center
- no `fault.kill`, `#PF`, `#GP`, or panic observed

Remaining:
Pointer movement quality is not yet smooth/perfect. Route is alive; tuning should be separate.

## CURSOR_ROUTE_ALIVE_V1

Route map (end-to-end, all hops proven live at runtime):

```
sexusb → sexinput → silk-shell → sexdisplay
```

### Root cause

`silk-shell` `linen_sync_reply()` consumed and acked non-reply `OP_HID_EVENT` messages
while waiting for Linen replies.  Pointer events arriving during Linen paint were lost
before Shell HID dispatch — cursor appeared frozen during boot composition.

### Fix

`linen_sync_reply()` now recognises `OP_HID_EVENT` and handles cursor input inline
(`apply_rel_pointer`) while continuing to wait for Linen replies.
A pre-Linen non-blocking input drain is also in place.

### Runtime proof markers

| Hop | Marker |
|-----|--------|
| sexusb → sexinput | `[sexinput.pointer.raw] a0=... a1=... a2=... caller=N` |
| sexinput → silk-shell | `[sexinput.pointer.send] class=2 ...`  /  `[sexinput.hid.emit.rel] ...` |
| silk-shell receives during Linen sync | `[silk-shell.linen_sync.input_hid] class=2 ...` |
| silk-shell updates cursor | `pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_CURSOR, x, y)` |
| sexdisplay applies | `[sexdisplay.cursor.surface.update] n=0 x=...` |
| sexdisplay draws | `[sexdisplay.cursor.draw] n=0 x=...` |

### Invariant

**Blocking reply helpers MUST be input-aware.**
No wait-for-reply loop may ack/drop `OP_HID_EVENT`.

Any future helper that loops on `pdx_try_listen_raw` waiting for a specific reply
opcode must handle `OP_HID_EVENT` inline and continue waiting — never ack/drop it.

### Remaining work

Pointer movement quality (smoothing, gain reduction, clamping) is separate
and tracked under `POINTER_QUALITY_V1`.  Route liveness is the prerequisite.

## CURSOR_ROUTE_BOOTGRAPH_GATE_V1

**Date:** 2026-05-08
**Status:** MERGED

### What it proves

The full input chain is alive end-to-end:

```
sexusb → sexinput → silk-shell → sexdisplay (cursor draw)
    ✅         ✅          ✅              ✅
  [send]    [send]    [recv/hid]    [draw x≠640,y≠360]
```

### Command

```bash
python3 scripts/check_cursor_route_log.py /tmp/sexos.log
```

### Pass output

```
[cursor.route.PASS] sexinput->silk-shell->sexdisplay moved cursor
```

### Fail output examples

```
[cursor.route.FAIL] missing=shell_hid,display_draw
[cursor.route.FAIL] missing=cursor_moved_from_center
[cursor.route.FAIL] fatal_fault_or_panic detected
```

### Required markers

| Key | Marker pattern | Proves |
|-----|---------------|--------|
| `sexinput_send` | `[sexinput.pointer.send] class=2` | sexinput emitted EV_REL |
| `shell_hid` | `[silk-shell.{linen_sync.input_hid,pointer.recv,hid.raw}] class=2` | Shell processed HID |
| `display_update` | `[sexdisplay.cursor.surface.update] n=0 x=... y=...` | Display received update |
| `display_draw` | `[sexdisplay.cursor.draw] n=0 x=... y=...` | Display rendered cursor |
| `cursor_moved_from_center` | Any draw where (x,y) ≠ (640,360) | Cursor actually moved |
| `fatal_fault_or_panic` | `fault.kill`, `#PF`, `#GP`, `panic` | Must be absent |

### Invariant

BootGraph gates turn runtime proof chains into repeatable checks.
Every new input/hid/cursor edge should add its required markers here.

## POINTER_QUALITY_V2_GATE_SYNTHETIC_ABS_V1

Status:
Cursor route remains alive and pointer movement is smoother.

Runtime proof:
- Relative movement is filtered:
  `[silk-shell.pointer.filter] raw_dx=-18 raw_dy=2 dx=-4 dy=1`
- Synthetic/proof absolute jump is gated:
  `[silk-shell.pointer.synthetic_abs.skip] x=940 y=560 reason=real_input_seen`
- Display cursor moves gradually:
  `[sexdisplay.cursor.draw] n=0 x=... y=...`
- Automatic gate passes:
  `[cursor.route.PASS] sexinput->silk-shell->sexdisplay moved cursor`

Invariant:
Real user pointer input outranks boot-time synthetic pointer proofs. Synthetic/proof ABS
events must not yank the cursor after real relative movement begins.

Remaining:
Fine-tune pointer feel later if needed, but do not change USB decode, sexinput wire format,
or sexdisplay render path for gain/clamp work unless STOP FIRST proves route regression.

## CLICK_FOCUS_ROUTE_V1

Status:
Click/focus route is fully instrumented. No patch needed from audit.

Route:
sexusb button report -> sexinput EV_BTN -> silk-shell click handler -> hit-test/focus/drag.

Proof markers:
- `[sexinput.pointer.button.down]`
- `[sexinput.pointer.button.up]`
- `[sexinput.pointer.send] class=4`
- `[silk-shell.pointer.recv] class=EV_BTN`
- `[silk-shell.click.down]`
- `[shell.click_focus.hit]` or `[shell.click_focus.miss]`
- `[shell.click_focus.send.ok]`
- `[silk-shell.click.up]`
- optional drag markers:
  `[shell.interact.drag.begin]`
  `[shell.interact.drag.move]`
  `[shell.interact.drag.end]`

Invariant:
Button/focus route is separate from pointer smoothing. Do not modify USB decode, sexinput wire
format, or sexdisplay render path for focus issues unless STOP FIRST proves the route regressed.

## POINTER_QUALITY_V5_COALESCE_ACCEL

Status:
Pointer movement is usable with shell-side piecewise scaling.

Runtime proof:
- `[silk-shell.pointer.filter] ... mode=piecewise`
- `[silk-shell.pointer.synthetic_abs.skip] ... reason=real_input_seen`
- `[silk-shell.click.down]`
- `[shell.click_focus.hit]`
- `[silkbar.clock.tick] ... ss=12`
- `[cursor.route.PASS] sexinput->silk-shell->sexdisplay moved cursor`

Formula:
- preserve ±1 for tiny nonzero movement
- medium movement scaled moderately
- large HID bursts capped at ±16 px
- cursor state clamped before display update

Invariant:
Pointer quality is SilkShell policy. Do not change USB decode, sexinput wire format,
sexdisplay framebuffer path, kernel routing, or PDX ABI for pointer feel unless STOP FIRST
proves route regression.

## POINTER_QUALITY_V8_TRACKER_LITE_CAP8

Status:
Pointer movement uses tracker-lite accumulation with max step capped near 8px.
It is not perfect, but it is usable enough to preserve as the current working baseline.

Runtime proof:
- `[silk-shell.pointer.accum]`
- `[silk-shell.pointer.flush] ... mode=tracker_lite`
- `[silk-shell.pointer.synthetic_abs.skip] x=940 y=560 reason=real_input_seen`
- `[silk-shell.click.down]`
- `[shell.click_focus.hit]`
- `[silkbar.clock.tick] ... ss=12`
- `[cursor.route.PASS] sexinput->silk-shell->sexdisplay moved cursor`

Remaining:
Trackpad/mouse feel still needs deeper acceleration/velocity tuning later. Do not regress the
working route while tuning.

Invariant:
Pointer feel is SilkShell policy. Do not change USB decode, sexinput wire format,
sexdisplay framebuffer path, kernel routing, or PDX ABI for pointer acceleration unless
STOP FIRST proves route regression.

## SPINDLE_CAP_GRANT_V1

Status:
SilkShell -> Spindle PDX route is proven.

Root cause:
SilkShell had a logical Spindle route marker, but no kernel capability at `SLOT_SPINDLE=14`.
Synthetic keyboard sends failed with `ERR_CAP_INVALID = -4`.

Fix:
Kernel init now grants SilkShell `SLOT_SPINDLE -> Domain(spindle_id)`.

Runtime proof:
- `[kernel.cap.spindle.route] shell->spindle slot=14`
- `[bootgraph.cap.grant from=kernel to=3 slot=SLOT_SPINDLE target=12 ok=1 optional=1]`
- `[shell.synthetic_key.send] ... status=0`
- `[spindle.pdx.raw] type=0x202 ... caller=3`
- `[spindle.input.recv]`
- `[spindle.line.append]`
- `[spindle.line.backspace]`
- `[spindle.line.enter]`

Known follow-up:
Spindle receives keys, but its scancode-to-char mapping is wrong for several set-1 scancodes:
- `0x30` produced `~`, expected `b`
- `0x2e` produced `|`, expected `c`
- `0x20` produced `c`, expected `d`

Invariant:
A UI route marker is not a capability proof. Every cross-PD route must have a matching
kernel capability grant and a runtime send-status proof.

## SPINDLE_KEYBOARD_ROUTE_AND_KEYMAP_V1

Status:
Synthetic keyboard route to Spindle is proven.

Runtime proof:
- `[kernel.cap.spindle.route] shell->spindle slot=14`
- `[shell.synthetic_key.send] ... status=0`
- `[spindle.pdx.raw] type=0x202`
- `[spindle.line.append] ch=a len=1`
- `[spindle.line.append] ch=b len=2`
- `[spindle.line.append] ch=c len=3`
- `[spindle.line.backspace] len=2`
- `[spindle.line.append] ch=d len=3`
- `[spindle.line.enter] len=3 mode=insert text="abd"`

Fixes:
- SilkShell now has a kernel capability grant to call Spindle at `SLOT_SPINDLE=14`.
- Spindle now uses explicit PS/2 set-1 scancode mapping instead of broken range math.

Invariant:
A UI route marker is not a capability proof. Every cross-PD route must have a matching
kernel capability grant and runtime send-status proof.
