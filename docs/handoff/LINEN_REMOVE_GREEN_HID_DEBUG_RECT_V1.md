# LINEN_REMOVE_GREEN_HID_DEBUG_RECT_V1

Date: 2026-05-22
Scope: servers/linen/src/main.rs only
Status: DONE

---

## Root Cause

During boot, `maybe_run_linen_keyboard_nav_proof()` injects synthetic HID keydown events
(J/down 0x24 and K/up 0x25) to exercise the navigation state machine. Each of these
calls `handle_hid_event()`, which contained an unconditional debug side effect: a
`LINEN_COLOR_TOGGLE` that alternated between neon green (`0x0000FF00`) and red
(`0x00FF6464`), drawing an 80×60 fill rectangle at (20,20) via `pdx_call(SLOT_DISPLAY,
0xEF, …)`. This produced a visible neon green rectangle on first keydown without any
user input, which is unwanted for a production boot path.

## Removed Code

In `fn handle_hid_event()` (was lines 742–756), removed:

```rust
static mut LINEN_COLOR_TOGGLE: bool = false;
if value == 1 {
    LINEN_COLOR_TOGGLE = !LINEN_COLOR_TOGGLE;
    let color = if LINEN_COLOR_TOGGLE { 0x0000FF00 } else { 0x00FF6464 };
    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_LINEN,
        (20u64 << 32) | 20u64,
        (color << 32) | (60u64 << 16) | 80u64);

    static mut LINEN_VISUAL_BUDGET: u32 = 16;
    let vb = &mut LINEN_VISUAL_BUDGET;
    if *vb > 0 {
        *vb -= 1;
        serial_println!("[linen.focus.visual_update] color={:#x}", color);
    }
}
```

## Added Marker

At boot init (after `[linen.ready]`), added a one-time marker:

```
[linen.hid.debug_rect.disabled] ok=1 reason=remove_neon_green_red_debug_rect_v1
```

## What Is Preserved

- All `handle_hid_event` call sites (keyboard input dispatch, nav proof injection) — unchanged
- Navigation state machine (`linen_nav_move`, `linen_nav_select_current`, etc.) — unchanged
- Init-time coral placeholder fill rect (line 570) — kept
- `SURFACE_ID_LINEN` and `SLOT_DISPLAY` usage — kept (init path still uses them)
- SexFiles100 proof markers — unaffected

## Proof Commands

```bash
# 1) Syntax/symbol verification
rg "0x0000FF00|LINEN_COLOR_TOGGLE|linen\.hid\.debug" servers/linen/src/main.rs
# Expected: only [linen.hid.debug_rect.disabled] marker at line 575

# 2) Binary marker check (rebuild first)
strings iso_root/servers/linen | grep linen.sexfiles100
# Expected: linen.sexfiles100.audit.begin present

# 3) Boot QEMU — confirm no neon green 80×60 rectangle drawn at boot

# 4) Daily gate
# Expected: FINAL PASS, faults_zero PASS
```

## Diff Summary

```
574a575
>     serial_println!("[linen.hid.debug_rect.disabled] ok=1 reason=...");
739,754d739
<     // removed: LINEN_COLOR_TOGGLE static
<     // removed: color toggling + pdx_call 0xEF debug fill rect
<     // removed: LINEN_VISUAL_BUDGET debug print
```

## Files Changed

- `servers/linen/src/main.rs` — remove debug rect, add disabled marker
- `servers/linen/src/main.rs.bak` — backup created
- `docs/handoff/LINEN_REMOVE_GREEN_HID_DEBUG_RECT_V1.md` — this file

## Note

First-paint delay is a separate concern. This change only removes the HID-event-driven
debug rectangle. The init-time coral placeholder fill rect is untouched.

## Commit Command

```
git add servers/linen/src/main.rs docs/handoff/LINEN_REMOVE_GREEN_HID_DEBUG_RECT_V1.md
git commit -m "linen: remove green HID debug rect, gate via [linen.hid.debug_rect.disabled] marker"
```
