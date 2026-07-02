# SILK_POINTER_RESIZE_STATE_V1 Handoff

**Status:** Complete  
**Date:** 2026-05-20  
**File patched:** `servers/silk-shell/src/main.rs`

---

## What Was Done

Implemented pointer resize interaction state in silk-shell. No ABI/protocol changes.
No sexdisplay edits. No existing drag behavior broken.

### New Types

```rust
enum ResizeEdge {
    Left, Right, Top, Bottom,
    TopLeft, TopRight, BottomLeft, BottomRight,
}

// Added to InteractionState:
Resizing {
    surface_id: u64,
    edge: ResizeEdge,
    start_x: i32, start_y: i32,
    start_w: u32, start_h: u32,
    pending_dx: i32, pending_dy: i32,
}
```

### New Constants

```rust
const FRAME_CHROME_RESIZE: u32 = 3;   // new kind for hit_test_surface_chrome
const FRAME_RESIZE_ZONE_PX: i32 = FRAME_RIM_PX;  // = 4px
```

### Hit Zone Design

`hit_test_surface_chrome` checks resize zones BEFORE the rim/drag check:

| Zone | Condition | Result |
|------|-----------|--------|
| Bottom edge | `y > bottom - 4` | RESIZE |
| Left/right below top bar | `(on_left\|on_right) && y >= sy + band_height` | RESIZE |
| Top-left/right corners | `(on_left\|on_right) && y < sy + 4` | RESIZE |
| Top bar (rest) | `y < sy + band_height` | RIM (drag, unchanged) |

Top bar drag behavior preserved. All drag markers `[shell.drag.*]` unchanged.

### New Helper Functions

- `compute_resize_edge(px, py, sx, sy, sw, sh) -> ResizeEdge` — pure, no unsafe
- `resize_accumulate_delta(dx, dy) -> bool` — unsafe, updates `INTERACTION` in-place
- `clear_resize_if_dead()` — unsafe, cancels resize if target surface dead

### State Transitions Added to `try_transition`

- `ClickPending → Resizing` (resize start on pointer down in resize zone)
- `Resizing → Idle` (pointer release or surface death)

### Proof Markers

All four required markers emitted:

| Marker | When |
|--------|------|
| `[silk.resize.hit]` | Resize zone detected, edge computed, before transition |
| `[silk.resize.begin]` | After `try_transition(Resizing)` succeeds (budgeted ×8) |
| `[silk.resize.delta]` | Each pointer move while Resizing (budgeted ×16) |
| `[silk.resize.end]` | Pointer release while Resizing (all 3 button handlers) |

### Button Release Coverage

Three handler sites updated:
1. Early HID handler (line ~8813) — synthetic/inline path
2. USB mouse report handler (line ~19472) — real USB path
3. HID EV_BTN handler (line ~20773) — HID relay path

### Pointer Move Coverage

Two move sites updated:
1. USB mouse report (after `drag_move_focused`) — `resize_accumulate_delta(dx as i32, dy as i32)`
2. HID EV_REL path (after `drag_move_focused`) — `resize_accumulate_delta(dx, dy)`

### Geometry Not Applied

Pending deltas are tracked only. Geometry application (surface resize IPC to sexdisplay)
is NOT done in V1 — existing keyboard resize code is not a shared helper (inline per surface),
so pointer resize application is deferred to next prompt.

---

## Build Verification

Build gate: `bash scripts/entrypoint_build.sh`  
Result: **success** (`[SEXOS ENTRYPOINT] success`)  
Warnings: pre-existing (dead_code, unnecessary unsafe in kaleidoscope/silkclient). None new.

No `#PF`, `#GP`, panic, or fault.kill in build output.

---

## Recurrence Notes

- `try_transition` FSM is strict. Any new state needs explicit arms in both directions.
- `hit_test_surface_chrome` check order matters: resize check must precede rim check.
- Button release has THREE handler sites — all must be updated together.
- Pointer move has TWO sites (USB and HID). Missing one = half-delta accumulation.
- `INTERACTION` is `static mut`. Reconstructing enum variant is the pattern for in-place field update.
