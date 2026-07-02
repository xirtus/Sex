# SILKBAR_SINGLE_CLOCK_OWNER_FIX_V1

## Root Cause

Visible clock had competing producers inside `sexdisplay`:

1. local fallback clock advancement (raw-tick + synthetic loop paths) mutating `bar.clock_*`
2. real SilkBar `SetClock` updates mutating the same fields

With bounded drain batching, multiple queued `SetClock` updates could be applied in one cycle and only one strip redraw emitted, making visible time jump (for example `2 -> 18 -> 34 -> 50`).

## Policy Enforced

- SilkBar is single clock owner for visible time.
- `sexdisplay` no longer advances `hh:mm:ss` from fallback tick sources.
- `sexdisplay` redraws top strip only on model-visible update.

## Code Changes

- `servers/sexdisplay/src/main.rs`
  - Removed fallback clock producer paths:
    - raw tick increment path (`bar.clock_ss += 1` path)
    - synthetic fallback loop path
    - stale-silkbar fallback rearm state
    - fallback tick markers (`sexdisplay.clock.source.fallback.*`)
  - Kept `DEFAULT_SILK_BAR` static clock until first SilkBar `SetClock`.
  - Preserved `DRAIN_MAX = 16`.
  - Added per-drain SetClock clamp:
    - process at most one changed `SetClock` per drain cycle
    - skip additional `SetClock` messages in same drain cycle
  - Kept batched redraw (post-drain), no per-message redraw loop.

## ABI/Scope Safety

- No ABI/model layout changes.
- No OP format changes.
- No kernel/scheduler/APIC edits.
- No USB/HID edits.
- Framebuffer bounds checks untouched.

## Proof Markers

- Expect absent:
  - `[sexdisplay.clock.source.fallback.tick]`
  - `[sexdisplay.clock.source.fallback.rearm]`
- Expect present:
  - `[sexdisplay.clock.redraw] ... source=silkbar`
  - `[sexusb.ready]`
- Fault scan must remain clean:
  - `KERNEL PANIC`
  - `EXCEPTION PAGE FAULT`
  - `KERNEL PAGE FAULT HALT`
  - `GP FAULT`
  - `PKU SECURITY`
  - `fault.kill pd=6`
