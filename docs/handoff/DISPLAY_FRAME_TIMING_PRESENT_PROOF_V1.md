# DISPLAY_FRAME_TIMING_PRESENT_PROOF_V1

## Status: PASS — All 5 proof markers proven

- date: 2026-05-06
- git commit: (pending)
- gate: SEXOS_DISPLAY_PRESENT_PROOF=1
- result: ALL CHECKS PASSED (GREEN_MASTER, 30s probe)

## Summary

Implemented a display frame timing, present order, and framebuffer safety proof
that runs inline in the sexdisplay compositor. The proof validates that:

1. **Frame ticks** occur at regular intervals (via clock-driven top-strip redraws)
2. **Present events** fire on full render calls with correct FB dimensions
3. **OOB write rejection** catches invalid framebuffer addresses
4. **Bounds clamping** correctly constrains surface rectangles
5. **Sustained render** proves the display subsystem is stable over many frames

No kernel edits, no sex-pdx ABI edits, no framebuffer ownership changes.
Proof runs 100% inline within the existing sexdisplay main loop — no
separate proof module needed.

## Proof Markers

All 5 required markers pass:

| Marker | Status | Sample Output |
|--------|--------|---------------|
| `[display.proof.frame.tick]` | PASS | frame=4 present=2 (fires every 4 frames) |
| `[display.proof.present]` | PASS | frame=1 present=1 fb_w=1280 fb_h=800 |
| `[display.proof.oob_reject]` | PASS | ok=1 null=1 zero_w=1 bounds=1 max_w=1 end_addr=1 |
| `[display.proof.bounds_ok]` | PASS | ok=1 normal=1 neg=1 oversize=1 edge=1 bar=1 |
| `[display.proof.sustained]` | PASS | frames=17 present=2 threshold=12 |

### Additional Diagnostic

| Marker | Purpose |
|--------|---------|
| `[display.proof.oob_reject] start` | OOB test begin |
| `[display.proof.bounds_ok] start` | Bounds test begin |

## Proof Architecture

### Frame Counter (AtomicU64)

Incremented atomically on every:
- `render()` — full framebuffer render (once at boot + on primary FB handoff)
- `redraw_top_strip()` — clock-driven top-strip redraw (every ~1 second)

The `[display.proof.frame.tick]` marker fires every 4th frame to avoid
serial log spam while proving liveness.

### Present Counter (AtomicU64)

Incremented on every `render()` call. The `[display.proof.present]` marker
fires on frame=1 to prove the present path is taken and records FB dimensions.

### OOB Write Rejection (one-shot)

Validates the address guards in `render()`:
- **null test**: address 0 < HIGH_HALF_BASE → render returns immediately
- **zero_w test**: w=0 → render returns (checked_mul fails or zero pixels)
- **bounds test**: MAX_FB_W × MAX_FB_H does not overflow
- **max_w test**: w > MAX_FB_W → render returns
- **end_addr test**: address + byte_count does not overflow u64

All 5 sub-checks must pass (ok=1 on each).

### Bounds Clamp (one-shot)

Validates `clamp_surface()` with 5 edge cases:
- **normal**: surface fully within FB → x=100, y=100, w=400, h=300
- **neg**: negative x/y → clamped to 0 / BAR_H (50)
- **oversize**: w/h exceed FB → clamped to FB dimensions
- **edge**: surface at far right/bottom → clamped within [0, FB_w/h)
- **bar**: y < BAR_H → y clamped to >= 50 (top-strip preserve)

All 5 sub-checks must pass.

### Sustained Render

After SUSTAINED_FRAME_THRESHOLD (12) frames, emits the sustained marker.
On a 30-second probe, this typically fires around frame 17 showing the
display subsystem is stable and producing frames continuously.

## Files Changed

| File | Change |
|------|--------|
| `servers/sexdisplay/src/main.rs` | Added proof gate constant, atomic counters, OOB/bounds/clamp proof functions, frame counter increments in `render()` and `redraw_top_strip()`, sustained emission in main loop |
| `docs/handoff/DISPLAY_FRAME_TIMING_PRESENT_PROOF_V1.md` | This handoff document |

## Files NOT Changed (Per Mission Rules)

| File | Reason |
|------|--------|
| `crates/sex-pdx/src/lib.rs` | STOP FIRST — no new PDX opcodes needed |
| `kernel/src/` | STOP FIRST — no kernel display path changes |
| `apps/sexdrive/src/main.rs` | NOT CHANGED — framebuffer handoff unchanged |
| `servers/silk-shell/` | NOT CHANGED — no renderer policy changes |

## Invariants Preserved

- **sexdisplay sole framebuffer writer**: proof only reads FB pixels; all writes go through existing render paths
- **Framebuffer bounds checks preserved**: OOB proof validates (not weakens) the existing HIGH_HALF_BASE/MAX_FB_W/MAX_FB_H guards
- **Submit/present must not mutate app-owned state**: surface registry is read-only during proof; no surface mutations
- **No renderer policy ownership**: proof runs inline in sexdisplay, no shell changes
- **No alpha/blur/shadow/full-frame effects**: additive markers only
- **No framebuffer ownership change**: FB_PTR/FB_W/FB_H unchanged
- **No shared-memory/backing-buffer redesign**: existing fill-rect model unchanged

## Build/Runtime Result

### Compilation
```
SEXOS_DISPLAY_PRESENT_PROOF=1 cargo check -p sexdisplay --target x86_64-sex.json
```
Result: PASS (no errors; 2 pre-existing warnings about nested unsafe blocks)

### Gate Run
```
SEXOS_DISPLAY_PRESENT_PROOF=1 cargo build -p sexdisplay --release
# repackage ISO with updated binary
./scripts/master_runtime_gate.sh --skip-build --probe 30 --keep-log
```
Result: GREEN_MASTER — all 5 proof markers present and passing

### Probe Window
- Minimum: 20 seconds (to hit sustained threshold of 12 frames at ~1/sec)
- Recommended: 30 seconds (with margin for boot time)

## Gate Run Commands

```bash
# Build sexdisplay with proof
SEXOS_DISPLAY_PRESENT_PROOF=1 cargo clean -p sexdisplay --target x86_64-sex.json
SEXOS_DISPLAY_PRESENT_PROOF=1 cargo build -p sexdisplay --target x86_64-sex.json --release

# Repackage ISO
cp target/x86_64-sex/release/sexdisplay iso_root/servers/sexdisplay
xorriso -as mkisofs ... iso_root -o sexos-v1.0.0.iso

# Run proof gate (30s probe)
./scripts/master_runtime_gate.sh --skip-build --probe 30 --keep-log
```
