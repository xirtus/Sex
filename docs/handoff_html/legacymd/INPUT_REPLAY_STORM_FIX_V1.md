# INPUT_REPLAY_STORM_FIX_V1

**Date:** 2026-05-03
**Status:** MERGED

## Symptom

Around 4 seconds after boot, the UI enters an infinite drag replay storm:
- `[shell.drag.start] id=100 x=200 y=200` repeats every ~160ms
- `[shell.drag.move] dx=6 dy=4` follows each start
- `[shell.drag.end]` closes each cycle
- Cycle repeats endlessly (~6×/second) — **50 drag cycles in 8 seconds**
- Clock appears to freeze (log spam + snapshot storms starve visual updates)

## Root Cause

`servers/sexinput/src/main.rs` line 198:
```rust
drag_proof_stage = (drag_proof_stage + 1) % 3;
```

The synthetic drag proof state machine wraps 0→1→2→0→1→2→... **forever**.
There is no terminal `DONE` state. Additionally, the trigger condition `tick % 120 == 0`
fires every 120 iterations because `tick` wraps and keeps incrementing.

The silkbar-click and synth-click proofs are one-shot (guarded by exact tick values
like `tick == 3`), so they do not repeat. Only the drag proof loops infinitely.

## Invariant Violated

**One-shot proof invariant:** Every synthetic proof sequence must have a terminal
`DONE`/`DISABLED` state after its final event. State machines must not wrap forever.

## Fix

**File:** `servers/sexinput/src/main.rs` (+8 lines)

1. Added `static mut SYNTHETIC_DRAG_PROOF_DONE: bool = false;` — one-shot gate flag.
2. Gate condition changed from:
   ```rust
   if !USB_PROOF_DISABLE_SYNTH_DRAG && tick % 120 == 0 {
   ```
   to:
   ```rust
   if !USB_PROOF_DISABLE_SYNTH_DRAG && !unsafe { SYNTHETIC_DRAG_PROOF_DONE } && tick % 120 == 0 {
   ```
3. Stage 2 (the `_ =>` arm, button up) now sets:
   ```rust
   unsafe { SYNTHETIC_DRAG_PROOF_DONE = true; }
   ```
   and emits a new terminal marker `[sexinput.drag_proof.done]`.

No other files changed. Zero shell/kernel/display/pdx edits.

## Verification

```bash
# Build
./scripts/entrypoint_build.sh

# Run 15 seconds
SEXUSB_XHCI_TRACE=0 timeout 15 ./dev.sh run-nographic \
  2>/tmp/input-replay-fix.trace | tee /tmp/input-replay-fix.log

# Verify drag is one-shot
grep -c "shell.drag.start" /tmp/input-replay-fix.log   # Should be 1 (was 50)
grep -c "shell.drag.end" /tmp/input-replay-fix.log     # Should be 1 (was 50)
grep -c "shell.drag.move" /tmp/input-replay-fix.log    # Should be 1 (was 47)

# Verify DONE marker
grep -c "sexinput.drag_proof.done" /tmp/input-replay-fix.log  # Should be 1

# Verify all proof markers preserved
grep -c "silk.contract.validate.ok" /tmp/input-replay-fix.log  # ≥1
grep -c "silk.render_proof.top_strip.ok" /tmp/input-replay-fix.log  # ≥1
grep -c "shell.silkbar.click" /tmp/input-replay-fix.log  # ≥4

# Verify no regressions
grep -cE "fault|panic|GP|PF" /tmp/input-replay-fix.log  # 0 (real faults)
grep -c "pdx.opcode.unknown" /tmp/input-replay-fix.log  # 0
```

## Verified Results (2026-05-03)

```
sexinput.synthetic:          11  (unchanged)
sexinput.drag_proof.done:     1  (NEW — terminal marker)
shell.drag.start:             1  (was 50 — FIXED)
shell.drag.end:               1  (was 50 — FIXED)
shell.drag.move:              1  (was 47 — FIXED)
silk.contract.validate.ok:    2  (unchanged)
silk.render_proof.top_strip.ok: 1  (unchanged)
shell.silkbar.click:          7  (unchanged)
shell.interaction.transition: 16  (unchanged — budget cap hit by expected proof)
fault/panic/GP/PF:            2  (false positives: "PRIMARY_GPU_LEASE")
pdx.opcode.unknown:           0  (unchanged)
```

## Changed Invariants

1. The synthetic drag proof is now **strictly one-shot**. After stage 2 (button up),
   `SYNTHETIC_DRAG_PROOF_DONE` is set to `true` and the block never executes again.
2. All other synthetic proofs (silkbar-click, synth-click) remain one-shot via
   their existing tick-gated stage machines and are unaffected.
3. Normal USB pointer events are not affected — the gate only guards the synthetic
   proof path.

## STOP FIRST Conditions

1. Removing the one-shot gate without adding equivalent terminal state
2. Adding other wrap-forever state machines in synthetic proof paths
3. Broad input architecture redesign
4. Kernel/sex-pdx/sexdisplay/silk-shell edits related to input
