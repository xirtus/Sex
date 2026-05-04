# FRAME_TOP_BAR_RENDER_V1

## Status

Implemented (2026-05-04). Sexdisplay top bar rendering via extended 0xFD tab info opcode. Smallest safe diff — no sex-pdx changes.

---

## Contracts Proven

| Contract | Mechanism | Marker |
|----------|-----------|--------|
| Chrome mode sent from shell to display | `send_frame_tab_info()` packs `chrome_flags` in 0xFD arg2 bit 8 | `[shell.frame.tab.info.send] chrome=N` |
| Sexdisplay stores chrome mode | `Surface.chrome_flags` field updated via 0xFD handler | `[sexdisplay.surface.tab.info] chrome=N` |
| Top bar renders 16px band when enabled | `composite_pixel()` Pass 2: top bar zone `ly < 16` | Visual: 16px neon band |
| Lights render 8×8 with 4px gaps in top bar mode | `FRAME_TOP_BAR_LIGHT_SIZE_PX=8`, `FRAME_TOP_BAR_LIGHT_GAP_PX=4` | Visual: larger lights |
| Tab strip renders after 40px exclusion in top bar mode | `FRAME_TOP_BAR_LIGHT_EXCLUSION_PX=40` | Visual: tab blocks after lights |
| Minimal 4px rim mode preserved | Path unchanged when `chrome_flags & 1 == 0` | Existing markers fire |
| Lights > tab strip > background priority in top bar | Sequential override: bg→tabs→lights | Visual priority preserved |
| Left/right/bottom rim always render at 4px | Non-top edges always in rim band | Visual: rim visible |
| No kernel/ABI/sex-pdx changes | Userland 0xFD arg2 encoding only | Build passes |
| Backward compatible | Existing callers have high bits=0 → chrome=0 | No regression |

---

## 0xFD Payload Encoding

### Protocol: Extend existing 0xFD opcode (no new opcode)

| Arg | Field | Before | After |
|-----|-------|--------|-------|
| arg0 | surface_id | u64 | u64 (unchanged) |
| arg1 | tab_count | u64 (clamped to u8) | u64 (clamped to u8, unchanged) |
| arg2 | active_tab + chrome_flags | active_tab (u64, clamped to u8) | `active_tab \| (chrome_flags << 8)` |

### Bit layout of arg2

```
bit 0..7 : active_tab (0..7, clamped to tab_count-1)
bit 8    : chrome_flags & 1 → top_bar_enabled (0=minimal, 1=top bar)
bit 9..63: reserved (zero)
```

### Sexdisplay decode

```rust
let raw_arg2 = msg.arg2;
let active_tab_raw = raw_arg2 as u8;              // low 8 bits
let chrome_flags_raw = ((raw_arg2 >> 8) & 0xff) as u8;  // bit 8
// Clamp active_tab to valid range (min with tab_count-1)
let active_tab = if tab_count > 0 {
    active_tab_raw.min(tab_count.saturating_sub(1))
} else { 0 };
slot.chrome_flags = chrome_flags_raw;
```

### Shell encode

```rust
let chrome_flags: u64 = if frame_has_top_bar(frame_id) { 1 } else { 0 };
let arg2 = (active_tab as u64) | (chrome_flags << 8);
pdx_call(SLOT_DISPLAY, OP_SURFACE_TAB_INFO, surface_id, tab_count as u64, arg2);
```

### Backward compatibility

All existing callers pass `arg2 = active_tab` (a u8 value 0..7). The high bits are zero, so `chrome_flags_raw` decodes to 0. The surface renders in minimal mode (existing 4px rim). No behavioral change.

---

## Surface.chrome_flags

### Field added to Surface struct

```rust
struct Surface {
    // ... existing fields ...
    tab_count: u8,
    active_tab: u8,
    // NEW:
    chrome_flags: u8,   // bit 0: top bar enabled
    // ... fill rect fields ...
}
```

### Flag constant

```rust
const SURFACE_CHROME_TOP_BAR: u8 = 1 << 0;
```

### Initializers

All three Surface initialization sites include `chrome_flags: 0`:
- `SURFACE_EMPTY` constant
- 0xE4 legacy handler (line ~804)
- 0xEC create handler (line ~860)

### Deduced property

```rust
let top_bar_active = (surf.chrome_flags & SURFACE_CHROME_TOP_BAR) != 0;
```

---

## Rendering Constants (sexdisplay)

### New constants

```rust
const SURFACE_CHROME_TOP_BAR: u8 = 1 << 0;
const FRAME_TOP_BAR_HEIGHT_PX: usize = 16;
const FRAME_TOP_BAR_LIGHT_SIZE_PX: usize = 8;
const FRAME_TOP_BAR_LIGHT_GAP_PX: usize = 4;
const FRAME_TOP_BAR_LIGHT_EXCLUSION_PX: usize = 40;
const FRAME_TOP_BAR_COLOR: u32 = FRAME_RIM_COLOR; // 0x00C0F0FF
const FRAME_TOP_BAR_LIGHT_TOP: usize = 4;
const FRAME_TOP_BAR_LIGHT_BOTTOM: usize = 12; // 4 + 8
```

### Existing constants (unchanged)

| Constant | Value | Used for |
|----------|-------|----------|
| FRAME_RIM_PX | 4 | Left/right/bottom rim; top rim in minimal mode |
| FRAME_RIM_COLOR | 0x00C0F0FF | Rim and top bar background |
| FRAME_LIGHT_SIZE_PX | 4 | Minimal mode light size |
| FRAME_LIGHT_GAP_PX | 2 | Minimal mode light gap |
| TAB_STRIP_LIGHT_EXCLUSION_PX | 20 | Minimal mode tab strip start |
| TAB_ACTIVE_COLOR | 0x00A8E0FF | Active tab (both modes) |
| TAB_INACTIVE_COLOR | 0x006080B0 | Inactive tab (both modes) |
| FRAME_LIGHT_CLOSE_COLOR | 0x00FF4444 | Red close light (both modes) |
| FRAME_LIGHT_MINIMIZE_COLOR | 0x00FFCC44 | Yellow minimize light (both modes) |
| FRAME_LIGHT_ZOOM_COLOR | 0x0044FF44 | Green zoom light (both modes) |

---

## Rendering Geometry

### Default mode (top bar enabled) — top 16px

```
y=0   ┌──────────────────────────────────────────────────┐
      │  TOP BAR BAND                                     │
y=4   │  ╔══╗╔══╗╔══╗    [tab 0] [tab 1]                │
y=12  │  ╚══╝╚══╝╚══╝                                   │
y=16  ├──────────────────────────────────────────────────┤
      │  SURFACE CONTENT                                  │
      │  (focused surface color + fill rect)              │
sh-4  ├──────────────────────────────────────────────────┤
sh    └──────────────────────────────────────────────────┘
      x=0 x=4           x=40           rim_right=(sw-4)
      left              tab strip      right rim
      rim               start
```

| Element | Y-range | X-range | Color |
|---------|---------|---------|-------|
| Close light | 4..12 | 4..12 | 0x00FF4444 (red) |
| Minimize light | 4..12 | 16..24 | 0x00FFCC44 (yellow) |
| Zoom light | 4..12 | 28..36 | 0x0044FF44 (green) |
| Tab strip blocks | 0..16 | 40..rim_right | active: 0x00A8E0FF, inactive: 0x006080B0 |
| Top bar background | 0..16 | anywhere else | 0x00C0F0FF (neon rim) |

### Minimal mode (top bar disabled) — 4px rim

Unchanged from previous behavior. All existing constants and rendering paths are preserved.

---

## Render Priority (composite_pixel Pass 2)

### Execution order

```
1. TOP BAR ZONE (if top_bar_active && ly < 16):
   a. Set background = FRAME_TOP_BAR_COLOR
   b. Override with tab strip if lx >= 40 && lx < rim_right
   c. Override with light color if ly in 4..12 and lx in light range
   
2. ELSE RIM BAND (if ly < 4 || lx < 4 || lx >= rim_right || ly >= rim_bottom):
   a. If ly < 4 && !top_bar_active: minimal mode top rim
      - Existing 4px lights → tab strip → rim color
   b. Else: left/right/bottom rim → FRAME_RIM_COLOR

3. ELSE CONTENT AREA:
   a. fill_rect_color() if active
   b. FOCUS_SURFACE_COLOR
```

### Priority: lights > tab strip > background

The sequential override design ensures:
- Lights always win within their pixel ranges (checked last)
- Tab strip wins over background (checked after background)
- Background fills the remaining top bar area
- No nested if-else chains → flat override sequence

---

## Minimal Mode Compatibility

When `chrome_flags & SURFACE_CHROME_TOP_BAR == 0`:

- The top bar zone condition `top_bar_active && ly < 16` is false (short-circuit)
- Falls through to the existing rim band check
- The `if ly < FRAME_RIM_PX && !top_bar_active` guard ensures the top rim runs the original 4px lights/tab/rim code
- Left/right/bottom rim: `FRAME_RIM_COLOR` (same as before)
- Content area: `fill_rect_color()` (same as before)

**Zero behavioral change.** All existing markers, click targets, and rendering paths are preserved.

---

## Diagnostic Markers

### Extended markers

| Marker | Budget | Before | After |
|--------|--------|--------|-------|
| `[shell.frame.tab.info.send]` | 8 | `... tabs=N active=N` | `... tabs=N active=N chrome=N` |
| `[sexdisplay.surface.tab.info]` | 8 | `... tabs=N active=N` | `... tabs=N active=N chrome=N` |

### Pre-existing markers that must still fire

| Marker | Status |
|--------|--------|
| `[shell.frame.topbar.model]` | Top bar model proof ✅ |
| `[shell.frame.light.model]` | Lights model proof ✅ |
| `[shell.frame.tab.model]` | Tab strip model proof ✅ |
| `[shell.frame.tab.switch]` | Tab switching |
| `[shell.frame.light.close/minimize/zoom]` | Light actions |
| `[shell.drag.start/move/end]` | Rim drag |
| `[shell.focus.set]` | Focus changes |

---

## Files Changed

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | Extended `send_frame_tab_info()` to pack `chrome_flags` in arg2 bit 8. Extended marker to log `chrome=N`. |
| `servers/sexdisplay/src/main.rs` | Added `chrome_flags: u8` to `Surface` struct. Added `SURFACE_CHROME_TOP_BAR`, 7 top bar constants. Updated `SURFACE_EMPTY`, 0xE4 create, and 0xEC create initializers with `chrome_flags: 0`. Updated 0xFD handler to extract and store `chrome_flags` from arg2 high bits. Extended marker to log `chrome=N`. Refactored `composite_pixel()` Pass 2: top bar zone check before rim band, sequential override for light/tab/bg priority. Added `!top_bar_active` guard to minimal mode top rim path. |

### Files NOT Modified

Kernel, sex-pdx, silkbar, silkbar-model, sexusb, sexinput — all untouched.

---

## Build

```bash
./scripts/entrypoint_build.sh
```

Default build passes. Synthetic build passes. No new warning types. Pre-existing warnings unchanged.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-top-bar-render-v1.log

for m in \
  shell.frame.tab.info.send \
  sexdisplay.surface.tab.info \
  shell.frame.topbar.model \
  shell.frame.tab.switch \
  shell.frame.light.close \
  shell.frame.minimize \
  shell.frame.zoom \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end
do
  printf "%-46s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-top-bar-render-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-top-bar-render-v1.log
```

### Pass criteria

- Default build passes ✅
- Synthetic build passes ✅
- `[shell.frame.tab.info.send]` fires with `chrome=1` ✅
- `[sexdisplay.surface.tab.info]` fires with `chrome=1` ✅
- `[shell.frame.topbar.model]` fires ✅
- Top bar renders 16px neon band (visual check: sexdisplay now draws 16px instead of 4px)
- Lights rendered at 8px with 4px gaps (visual check: larger colored squares)
- Tab strip rendered after 40px exclusion (visual check: tab blocks start after wider gap)
- Minimal mode code path preserved (compile-time proof through `!top_bar_active` guard)
- No panic/#PF/#GP ✅
- No kernel/sex-pdx changes ✅
- Only silk-shell, sexdisplay, and handoff doc changed ✅

---

## Risks and Limitations

- **No glass/alpha effects:** Top bar background is solid neon rim color. No transparency or glass blur. Deferred.
- **No text labels:** Tab blocks are colored rectangles without titles. Text pipeline not implemented.
- **No toggle mechanism:** Top bar is always ON at boot. No way to switch to minimal mode without code change. Toggle deferred to FRAME_CHROME_MODE_SETTINGS_V1.
- **Light priority exactness:** Lights use sequential override (bg→tabs→lights). The 8px lights with 4px gaps occupy x=4..36, tab strip starts at x=40, so there is no overlap. Priority is clean.
- **Tab strip in lights band:** The tab strip check covers the full 16px height (y=0..16), including the lights vertical band (y=4..12). Pixels in both the lights x-range and tab strip x-range are impossible (non-overlapping), but if they did overlap, lights would win (checked last).
- **Remove the visual mismatch:** Before this phase, sexdisplay rendered 4px rim while shell hit targets expected 16px top bar. Now rendering matches hit targets. The visual mismatch is resolved.

---

## Next Recommended Phase

### FRAME_TOP_BAR_TOGGLE_PLAN_V1

Design a mechanism to toggle top bar mode per-frame or globally:

1. Keyboard shortcut (e.g., `Ctrl+Shift+T`) to toggle `FRAME_FLAG_TOP_BAR` on focused frame
2. Send updated `chrome_flags` via 0xFD to sexdisplay
3. No settings app — V1 toggle is keyboard-only
4. Consider: should toggle be per-frame or global? Should minimized frames remember their top bar state?

Alternatively, if settings app is prioritized:

### SCENE_THEME_SETTINGS_MODEL_PLAN_V1

Design the theme/settings model: per-scene chrome mode, per-monitor overrides, rim color/thickness, light style. See `SILK_CHROME_SETTINGS_PLAN_V1.md` for roadmap.
