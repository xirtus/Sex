# FRAME_TOP_BAR_RENDER_PLAN_V1

## Status

Design (2026-05-04). Sexdisplay rendering and protocol extension for Silk Frame Top Bar. Docs-only — no code changed.

---

## Verdict: TOP_BAR_RENDER_SAFE_USERLAND ✅

| Requirement | Feasible? | How |
|-------------|-----------|-----|
| Sexdisplay renders 16px top bar | ✅ | Extend composite_pixel() Pass 2 with top bar zone |
| Sexdisplay knows chrome mode per surface | ✅ | Extend 0xFD arg2 high bits to carry `chrome_flags` |
| Lights render at 8×8 with 4px gaps | ✅ | New top bar constants in sexdisplay, similar to rim lights |
| Tab strip inside top bar | ✅ | Tab strip uses top bar height and 40px exclusion when mode=1 |
| Minimal 4px rim preserved | ✅ | When `chrome_flags & 1 == 0`, existing 4px rendering path runs |
| No new kernel ABI | ✅ | Userland opcode extension only, no syscall changes |
| No new opcode constant | ✅ | Pack chrome_flags into 0xFD arg2 high byte (bit 8) |
| Backward compatible | ✅ | Existing callers have high bits=0, chrome_flags decodes to 0 (minimal mode) |

---

## Protocol Route: Extend 0xFD arg2

**Chosen: Option A1 — pack `chrome_flags` in `arg2` bit 8.**

No new opcode. No new sex-pdx constant. No dispatch entry. Minimal diff.

### Current 0xFD layout

```
arg0 = surface_id (u64)
arg1 = tab_count (u64, clamped to u8 · max 8)
arg2 = active_tab (u64, clamped to u8 · 0..tab_count-1)
```

### Proposed 0xFD layout

```
arg0 = surface_id (u64)
arg1 = tab_count (u64, clamped to u8 · max 8)
arg2 = active_tab as u64 | (chrome_flags as u64) << 8
```

**Bits in arg2:**
```
bit 0..7 : active_tab (0..7)
bit 8    : chrome_flags & 1  → top_bar_enabled (0=minimal, 1=default/top bar)
bit 9..63: reserved (zero for now)
```

### Decode in sexdisplay 0xFD handler

```rust
let tab_count = (msg.arg1 as u8).min(8);
let active_tab = if tab_count > 0 {
    (msg.arg2 as u8).min(tab_count.saturating_sub(1))
} else { 0 };
let chrome_flags = (msg.arg2 >> 8) as u8;
let top_bar_enabled = (chrome_flags & 1) != 0;
```

### Backward compatibility

Existing callers pass `arg2 = active_tab` (a u8, 0..7). High bits are zero.
`chrome_flags` decodes to 0, `top_bar_enabled` = false → minimal 4px rim rendering.
No old code breaks.

### Shell send point: `send_frame_tab_info()`

Extend the existing function — no new shell entry points.

```rust
unsafe fn send_frame_tab_info(frame_id: u32) {
    let surface_id = match active_surface_for_frame(frame_id) {
        Some(sid) => sid,
        None => return,
    };
    let tab_count = frame_tab_count(frame_id);
    let active_tab = frame_active_tab_index(frame_id);
    let chrome_flags: u64 = if frame_has_top_bar(frame_id) { 1 } else { 0 };
    pdx_call(SLOT_DISPLAY, OP_SURFACE_TAB_INFO, surface_id,
        tab_count as u64,
        active_tab as u64 | (chrome_flags << 8));
}
```

The marker `[shell.frame.tab.info.send]` already logs frame, surface, tabs, active. Extend it to also log `chrome=N`.

---

## Surface Fields

### Add `chrome_flags: u8` to sexdisplay Surface struct

```rust
struct Surface {
    // ... existing fields (surface_id, owner_pd, x, y, w, h, color, active) ...
    tab_count: u8,
    active_tab: u8,
    // NEW:
    chrome_flags: u8,   // bit 0: top_bar_enabled
    // ... fill rect fields ...
}
```

### SURFACE_EMPTY initializer

```rust
const SURFACE_EMPTY: Surface = Surface {
    surface_id: 0, owner_pd: 0, x: 0, y: 0, w: 0, h: 0, color: 0, active: false,
    tab_count: 0, active_tab: 0,
    chrome_flags: 0,       // ← NEW
    fill_sx: 0, fill_sy: 0, fill_sw: 0, fill_sh: 0, fill_color: 0, fill_active: false,
};
```

Both create-site initializers (0xE4 legacy at line ~804, 0xEC create at line ~855) need `chrome_flags: 0` added.

### Deduced property

```rust
fn surface_has_top_bar(surf: &Surface) -> bool {
    (surf.chrome_flags & 1) != 0
}
```

---

## Rendering Constants (sexdisplay)

```rust
// ── Top Bar Constants ──
/// Height of the top bar chrome band (matches shell FRAME_TOP_BAR_HEIGHT_PX).
const FRAME_TOP_BAR_HEIGHT_PX: usize = 16;
/// Width and height of each frame light in default mode.
const FRAME_TOP_BAR_LIGHT_SIZE_PX: usize = 8;
/// Gap between adjacent frame lights in default mode.
const FRAME_TOP_BAR_LIGHT_GAP_PX: usize = 4;
/// X-width of the Frame Lights exclusion zone in default mode (matches shell).
const FRAME_TOP_BAR_LIGHT_EXCLUSION_PX: usize = 40;
/// Top bar background color (same as rim color for visual continuity).
const FRAME_TOP_BAR_COLOR: u32 = FRAME_RIM_COLOR;  // 0x00C0F0FF

/// Light vertical range within top bar (y=4..12, 8px tall, centered).
const FRAME_TOP_BAR_LIGHT_TOP: usize = 4;
const FRAME_TOP_BAR_LIGHT_BOTTOM: usize = 12;
```

### Existing constants preserved (unchanged)

| Constant | Value | Role |
|----------|-------|------|
| FRAME_RIM_PX | 4 | Left/right/bottom rim width; top rim height in minimal mode |
| FRAME_RIM_COLOR | 0x00C0F0FF | Neon rim color (also used for top bar background) |
| FRAME_LIGHT_SIZE_PX | 4 | Light size in minimal mode |
| FRAME_LIGHT_GAP_PX | 2 | Light gap in minimal mode |
| TAB_STRIP_LIGHT_EXCLUSION_PX | 20 | Light exclusion zone in minimal mode |
| TAB_ACTIVE_COLOR | 0x00A8E0FF | Active tab color |
| TAB_INACTIVE_COLOR | 0x006080B0 | Inactive tab color |
| FRAME_LIGHT_CLOSE_COLOR | 0x00FF4444 | Close light color |
| FRAME_LIGHT_MINIMIZE_COLOR | 0x00FFCC44 | Minimize light color |
| FRAME_LIGHT_ZOOM_COLOR | 0x0044FF44 | Zoom light color |

---

## Rendering Geometry

### Default mode top bar (16px, `chrome_flags & 1 == 1`)

```
sy ─┌──────────────────────────────────────────────────┐
    │ y=0..16: TOP BAR ZONE                            │
    │  ╔══╗╔══╗╔══╗  [tab 0] [tab 1]                  │
    │  ║CL║║MI║║ZO║  light colors                     │
    │  ╚══╝╚══╝╚══╝                                   │
    │  x=4  x=16 x=28  x=40..rim_right                 │
    ├──────────────────────────────────────────────────┤ sy+16
    │ y=16..sh-4: SURFACE CONTENT AREA                 │
    │  (focused surface color + optional fill rect)    │
    │                                                  │
    ├──────────────────────────────────────────────────┤ sy+sh-4
    │ y=sh-4..sh: BOTTOM RIM (4px)                     │
    └──────────────────────────────────────────────────┘ sy+sh
    ↑          ↑                       ↑
    sx         sx+4                   sx+sw-4..sx+sw
    left rim   content start           right rim
```

| Element | Y-range | X-range |
|---------|---------|---------|
| Top bar background | ly=0..16 | lx=0..sw |
| CLOSE light | ly=4..12 | lx=4..12 |
| MINIMIZE light | ly=4..12 | lx=16..24 |
| ZOOM light | ly=4..12 | lx=28..36 |
| Tab strip | ly=0..16 | lx=40..rim_right |
| Left rim | ly=16..sh-4 | lx=0..4 |
| Right rim | ly=16..sh-4 | lx=sw-4..sw |
| Bottom rim | ly=sh-4..sh | lx=0..sw |
| Content area | ly=16..sh-4 | lx=4..sw-4 |

### Minimal mode (4px rim, `chrome_flags & 1 == 0`)

Unchanged from current behavior.

| Element | Y-range | X-range |
|---------|---------|---------|
| Rim (all edges) | 4px | full width |
| Lights | ly=0..4 | lx=2..18 |
| Tab strip | ly=0..4 | lx=20..rim_right |
| Content area | ly=4..sh-4 | lx=4..sw-4 |

---

## Render Priority (composite_pixel Pass 2)

### Default mode rendering tree

```
Pixel (lx, ly) within focused surface:

1. If ly < FRAME_TOP_BAR_HEIGHT_PX (16) && chrome_flags & 1:
   → TOP BAR ZONE:
     a. If ly >= FRAME_TOP_BAR_LIGHT_TOP (4) && ly < FRAME_TOP_BAR_LIGHT_BOTTOM (12):
        → Lights band (8px tall, vertically centered):
          - CLOSE:   lx >= 4 && lx < 12          → FRAME_LIGHT_CLOSE_COLOR
          - MINIMIZE: lx >= 16 && lx < 24        → FRAME_LIGHT_MINIMIZE_COLOR
          - ZOOM:     lx >= 28 && lx < 36        → FRAME_LIGHT_ZOOM_COLOR
     b. If lx >= FRAME_TOP_BAR_LIGHT_EXCLUSION_PX (40) && lx < rim_right:
        → Tab strip (full 16px height):
          - Compute tab index → TAB_ACTIVE_COLOR or TAB_INACTIVE_COLOR
     c. Else:
        → FRAME_TOP_BAR_COLOR (rim color, neon cyan)

2. Else if ly < FRAME_RIM_PX (4) || lx < FRAME_RIM_PX
          || lx >= sw - FRAME_RIM_PX || ly >= sh - FRAME_RIM_PX:
   → EDGE RIM:
     a. If ly < FRAME_RIM_PX && chrome_flags & 1 == 0:
        → Minimal mode top rim (existing 4px lights → tab strip → rim color)
     b. Else:
        → FRAME_RIM_COLOR

3. Else:
   → CONTENT AREA:
     a. fill_rect_color() if fill_active
     b. FOCUS_SURFACE_COLOR
```

### Priority within top bar zone: lights > tab strip > background

```
Top bar pixel priority:
  1. CLOSE light      (y=4..12, x=4..12)
  2. MINIMIZE light   (y=4..12, x=16..24)
  3. ZOOM light       (y=4..12, x=28..36)
  4. Tab strip        (y=0..16, x=40..rim_right)
  5. Background       (everything else in top bar)
```

This matches the shell-side click priority in `click_hit_test_and_focus()`:
lights > tab strip > rim drag (background).

---

## 0xFD Handler Changes

```rust
0xFD => {
    let surface_id = msg.arg0;
    if surface_id == 0 { continue; }
    let tab_count = (msg.arg1 as u8).min(8);
    let active_tab = if tab_count > 0 {
        (msg.arg2 as u8).min(tab_count.saturating_sub(1))
    } else { 0 };
    let chrome_flags = (msg.arg2 >> 8) as u8;    // NEW: extract from arg2 high bits
    unsafe {
        let mut updated = false;
        for slot in SURFACES.iter_mut() {
            if slot.active && slot.surface_id == surface_id {
                slot.tab_count = tab_count;
                slot.active_tab = active_tab;
                slot.chrome_flags = chrome_flags;  // NEW: store chrome mode
                updated = true;
                break;
            }
        }
        if updated {
            // EXTENDED marker: add chrome_flags
            static mut SURFACE_TAB_INFO_BUDGET: u32 = 8;
            let b = &mut SURFACE_TAB_INFO_BUDGET;
            if *b > 0 {
                *b -= 1;
                serial_println!("[sexdisplay.surface.tab.info] surface={} tabs={} active={} chrome={:#x}",
                    surface_id, tab_count, active_tab, chrome_flags);
            }
            if fb_live {
                redraw_surface_area(FB_PTR as *mut u32, FB_W as usize, FB_H as usize);
            }
        }
    }
}
```

---

## composite_pixel() Refactoring Plan

The current Pass 2 rendering block (lines 114-181) needs restructuring:

### Current structure (simplified)

```
Pass 2:
  for surf where surface_id == focused_id:
    if point in surface bounds:
      if in rim band (ly < 4 || lx < 4 || lx >= sw-4 || ly >= sh-4):
        if ly < 4:                    // top rim only
          if light check: → light color
          else if tab strip: → tab color
          else: → rim color
        else: → rim color              // non-top edges
      else: → content color
```

### Proposed structure (simplified)

```
Pass 2:
  for surf where surface_id == focused_id:
    if point in surface bounds:
      let top_bar = (surf.chrome_flags & 1) != 0;
      if top_bar && ly < 16:
        // TOP BAR ZONE
        if ly >= 4 && ly < 12:
          if light_x(lx): → light color    // 8px lights
        if lx >= 40 && lx < rim_right:
          if tab_at(lx): → tab color       // full 16px height
        → FRAME_TOP_BAR_COLOR
      else if ly < 4 || lx < 4 || lx >= sw-4 || ly >= sh-4:
        // EDGE RIM
        if !top_bar && ly < 4:
          // Minimal mode top rim (existing 4px code)
          if light_x(lx): → light color
          else if tab_at(lx): → tab color
          else: → rim color
        else:
          → rim color
      else:
        → content color (with fill rect)
```

**Key refactoring notes:**
- The top bar zone check comes FIRST (before the rim band check)
- The light_x() checks use different constants per mode (8px/4px for top bar/minimal)
- The tab_at() logic uses different exclusion zone per mode (40px/20px)
- The rim band check on non-top edges (left/right/bottom) is identical in both modes
- Content area unchanged

---

## Compatibility with Minimal Rim Mode

When `chrome_flags & 1 == 0` (minimal mode), the new `if top_bar && ly < 16` branch is skipped entirely. The existing `else if` rim band check runs unchanged:

```
Pass 2 (minimal mode, chrome_flags bit 0 = 0):
  for surf where surface_id == focused_id:
    if ly < 4 || lx < 4 || lx >= sw-4 || ly >= sh-4:
      if ly < 4:
        → existing 4px lights/tab strip/rim code
      else:
        → rim color
    else:
      → content color
```

Zero behavioral change. All existing markers, click targets, and rendering paths are preserved.

---

## Diagnostic Markers

### Extended marker

| Marker | Budget | Changes |
|--------|--------|---------|
| `[sexdisplay.surface.tab.info]` | 8 | Append `chrome=N` field to existing log line |

No new markers needed — the existing 0xFD marker is extended with the chrome field.

### Shell marker

| Marker | Budget | Changes |
|--------|--------|---------|
| `[shell.frame.tab.info.send]` | 8 | Append `chrome=N` field to existing log line |

The existing `send_frame_tab_info()` marker is sufficient. Extended to log `chrome=1` or `chrome=0`.

---

## Implementation Files

### Modified: `servers/sexdisplay/src/main.rs`

| Change | Lines |
|--------|-------|
| Add `chrome_flags: u8` to Surface struct | After `active_tab` |
| Add `chrome_flags: 0` to SURFACE_EMPTY | In constant initializer |
| Add `chrome_flags: 0` to 0xE4 create site | ~line 804 |
| Add `chrome_flags: 0` to 0xEC create site | ~line 860 |
| Add top bar constants: `FRAME_TOP_BAR_HEIGHT_PX`, `FRAME_TOP_BAR_LIGHT_SIZE_PX`, `FRAME_TOP_BAR_LIGHT_GAP_PX`, `FRAME_TOP_BAR_LIGHT_EXCLUSION_PX`, `FRAME_TOP_BAR_COLOR`, `FRAME_TOP_BAR_LIGHT_TOP`, `FRAME_TOP_BAR_LIGHT_BOTTOM` | After tab strip constants |
| Update 0xFD handler to extract and store `chrome_flags` from arg2 bit 8 | ~line 1014-1023 |
| Refactor `composite_pixel()` Pass 2 to add top bar zone when `chrome_flags & 1` | ~line 128-178 |

### Modified: `servers/silk-shell/src/main.rs`

| Change | Lines |
|--------|-------|
| Extend `send_frame_tab_info()` to pack `chrome_flags` in arg2 bit 8 | ~line 1123 |
| Extend marker `[shell.frame.tab.info.send]` to log `chrome=N` | ~line 1127 |

### NOT Modified

- `crates/sex-pdx/src/lib.rs` — no new opcode constant needed
- `kernel/` — no ABI changes
- `servers/silkbar/` — no forwarding changes
- `crates/silkbar-model/` — no model changes
- `servers/sexusb/` — no synthetic proof changes
- `servers/sexinput/` — untouched

---

## Forbidden in FRAME_TOP_BAR_RENDER_V1

- Text rendering
- Settings app
- Dynamic allocation
- Kernel edits
- New opcode constants
- Sex-pdx changes
- Broad refactor of sexdisplay
- Per-pixel logging
- Frame buffer layout changes
- SilkBar model changes

---

## STOP Conditions

1. **0xFD arg2 high bits clobbered by existing callers** — Check: all `pdx_call(SLOT_DISPLAY, 0xFD, ...)` callers pass `active_tab` as arg2 (a u8). The high bits are zero. If any caller passes a wider value, the top bit would be clobbered. Mitigation: audit all 0xFD callers in the codebase. Current known callers: `send_frame_tab_info()` in silk-shell (only caller).

2. **composite_pixel() refactoring introduces drift** — The top bar zone and minimal rim zone share tab strip computation logic. Extract tab index computation into a helper function to prevent code drift between the two paths.

3. **Top bar overlaps with fill rect** — The top bar zone is rendered BEFORE the fill rect check. In Pass 2, the `else` branch (content area) handles `fill_rect_color()`. Since the top bar zone pixels never reach the `else` branch, fill rects that extend into the top 16px are clipped to the content area. This preserves existing behavior (fill rects in the rim zone are already clipped).

4. **Light geometry mismatch between shell and display** — If shell's `frame_light_at()` and sexdisplay's light rendering use different constants for the same mode, clicks won't match visuals. Solution: define matching constants in both files. The handoff doc FRAME_TOP_BAR_MODEL_V1.md documents the shared geometry.

5. **Tab strip tab block width divergence** — Shell's `frame_tab_at()` and sexdisplay's tab strip rendering must compute the same `slot_w` for the same tab_count. Mitigation: use the same formula: `available = rim_right - exclusion; slot_w = available / tab_count`. Both use integer division, so they produce the same result.

---

## Next Phase

### FRAME_TOP_BAR_RENDER_V1

```
MISSION: FRAME_TOP_BAR_RENDER_V1.

Implement top bar rendering in sexdisplay composite_pixel() Pass 2.
Pack chrome_flags into existing 0xFD arg2 high bits.
Shell-only change: extend send_frame_tab_info() arg2 encoding.

Design complete in FRAME_TOP_BAR_RENDER_PLAN_V1.md.

Changes:

1. servers/sexdisplay/src/main.rs:
   a. Add chrome_flags: u8 to Surface struct
   b. Add chrome_flags: 0 to SURFACE_EMPTY and both create-site initializers
   c. Add top bar constants (HEIGHT, LIGHT_SIZE, LIGHT_GAP, EXCLUSION, COLOR, LIGHT_TOP/BOTTOM)
   d. Update 0xFD handler: extract chrome_flags from arg2>>8, store in slot.chrome_flags
   e. Extend [sexdisplay.surface.tab.info] marker to log chrome=N
   f. Refactor composite_pixel() Pass 2: top bar zone check before edge rim check

2. servers/silk-shell/src/main.rs:
   a. Extend send_frame_tab_info(): pack chrome_flags = frame_has_top_bar(frame_id) << 8 into arg2
   b. Extend [shell.frame.tab.info.send] marker to log chrome=N

3. docs/handoff/FRAME_TOP_BAR_RENDER_V1.md

Forbidden:
- Text rendering
- New opcodes/constants in sex-pdx
- Kernel edits
- Settings app
- Dynamic allocation
- Broad refactor

PASS:
- Default build passes
- [sexdisplay.surface.tab.info] includes chrome=N
- [shell.frame.tab.info.send] includes chrome=N
- Focused surface renders 16px top bar with 8px lights and tab strip
- Lights click targets match rendered positions (verified by click)
- Tab strip click targets match rendered positions
- Minimize/zoom/close still work
- Minimal mode (chrome_flags=0) still renders 4px rim
- Tab switching still works
- No panic/#PF/#GP
- No kernel/sex-pdx changes
```
