# FRAME_TAB_STRIP_PLAN_V1

## Status

Design (2026-05-04). Analysis of tab strip feasibility. No code changed.

---

## Verdict

### TAB_STRIP_MODEL_SAFE_NOW ✅

The ShellFrame/ShellTab model already supports multi-tab. The hit-target model already defines `FRAME_CHROME_TAB_STRIP` (gated on `FRAME_TAB_STRIP_PX > 0`). What's needed for implementation:

| Requirement | Feasible? | How |
|-------------|-----------|-----|
| Tab strip geometry in top rim | ✅ | Set `FRAME_TAB_STRIP_PX = 4` (matches rim height) |
| Tab strip hit targets | ✅ | Already implemented in `hit_test_surface_chrome()`, gated |
| Frame Lights exclusion | ✅ | Lights checked first in `frame_light_at()` before tab strip |
| Tab strip rendering in sexdisplay | ⚠️ | Needs IPC for tab count + active tab, or V1-only assumption |
| Text labels on tabs | ❌ | No text renderer — **TAB_STRIP_LABELS_BLOCKED_BY_TEXT** |
| Active tab switching | ⏳ | Future phase (model exists, action not implemented) |

---

## Current ShellFrame/ShellTab State

### ShellTab (`servers/silk-shell/src/main.rs`, line 200)

```rust
struct ShellTab {
    surface_id: u64,
    title_id: u64,     // Reserved. Intended as string handle or content ID.
    flags: u32,        // Reserved (pinned, muted, loading, etc.)
}
```

- `title_id = 0` for the boot tab (surface 100). Never written.
- No title string storage anywhere.
- `title_id` could represent an index into a fixed set of known titles, but no such set exists.
- For text labels, a string storage mechanism would be needed (no heap, no strings in V1).

### ShellFrame (`servers/silk-shell/src/main.rs`, line 215)

```rust
struct ShellFrame {
    frame_id: u32,
    active_tab: u8,
    tab_count: u8,
    tabs: [Option<ShellTab>; MAX_TABS_PER_FRAME as usize],  // MAX_TABS_PER_FRAME = 8
    flags: u32,
    normal_x: i32,
    normal_y: i32,
    normal_w: u32,
    normal_h: u32,
}
```

- Active tab indexed by `active_tab` into `tabs[]`.
- `tab_count` tracks valid entries.
- V1: `tab_count = 1`, `active_tab = 0`, single tab wrapping surface 100.
- Tab array is fixed-size (8 slots), no dynamic allocation.
- Tab switching: set `active_tab` to different index, update sexdisplay surface focus to the new tab's `surface_id`.

### Frame Constants (line 192)

```rust
const MAX_TABS_PER_FRAME: u8 = 8;
const MAX_FRAMES: usize = 4;
```

---

## Current Hit-Target Model

### `hit_test_surface_chrome()` (line 1198)

Currently gated on `FRAME_TAB_STRIP_PX > 0` (disabled in V1):

```rust
if FRAME_TAB_STRIP_PX > 0 {
    let strip_top = sy;
    let strip_bot = sy + FRAME_TAB_STRIP_PX;
    if y >= strip_top && y < strip_bot && x >= sx && x < (sx + sw as i32) {
        return Some(HitTarget::FrameChrome { frame_id, kind: FRAME_CHROME_TAB_STRIP });
    }
}
```

**Issue:** The tab strip check covers the ENTIRE top band of the surface, including the zone where Frame Lights are drawn. Lights must take priority over tab strip. The fix: tab strip check should exclude the light zone (leftmost ~18px), or the FrameChrome dispatch in `click_hit_test_and_focus()` should check lights first (which it currently does — lights are checked before rim drag in the `FRAME_CHROME_RIM` arm, but `FRAME_CHROME_TAB_STRIP` hits skip the light check entirely).

### `click_hit_test_and_focus()` dispatch (line 1329)

```rust
HitTarget::FrameChrome { frame_id, kind } => {
    if kind == FRAME_CHROME_RIM {
        let light = frame_light_at(frame_id, px, py);
        // lights → action; else → rim drag
    }
    // FRAME_CHROME_TAB_STRIP falls through to line 1431:
    // } else if matches!(target, FrameChrome { kind: FRAME_CHROME_TAB_STRIP, .. }) {
    //     serial_println!("[shell.drag.skip.chrome] kind=tab_strip ...");
    // }
}
```

The tab strip arm currently skips drag (captures as no-op). No action behavior implemented.

---

## Proposed Tab Strip Geometry

### Layout within the top rim band

```
┌─────────────────────────────────────────────────────────────┐
│ CLOSE MIN ZOOM  │  TAB 1  │  TAB 2  │  TAB 3  │ (rim color)│ ← 4px top rim
└─────────────────────────────────────────────────────────────┘
 ←── 18px lights ─→← 2px gap →← 8-12px per tab →← remainder →
```

| Region | Position | Width | Height |
|--------|----------|-------|--------|
| Frame Lights | `sx+2` | 18px (close+gap+min+gap+zoom) | 4px |
| Gap | after lights | 2px | 4px |
| Tab block (each) | after gap | 8-12px per tab | 4px |
| Gap between tabs | between blocks | 2px | 4px |
| Rim fill | remaining space | `sw - lights - tabs - gaps` | 4px |
| Right rim | rightmost 4px | 4px | 4px |

### Single-tab frame (V1)

With 1 tab and `FRAME_TAB_STRIP_PX = 4`:
- One tab block after lights + 2px gap
- Tab block is ~8-12px wide
- Tab color: distinct from rim (e.g., `FOCUS_SURFACE_COLOR` or `RIM_COLOR` with 50% brightness)
- Active tab color: brighter/selected
- Hover on tab block: currently maps to `HOVER_TAB_STRIP` kind (already defined)

### Multi-tab frame (future)

With N tabs:
- Each tab gets a colored block, sized proportional to available width or fixed 8-12px
- Active tab uses selected color
- Inactive tabs use dimmer color
- If tabs exceed available width... scroll? clip? → V1 does not handle overflow (single tab case only)

### Tab strip height

`FRAME_TAB_STRIP_PX = 4` matches the existing rim height. The tab strip IS the top rim band. Tab blocks replace the rim color in the top band (except the light zone and the right edge rim).

**Note:** 4px is very small for text. For colored blocks it's fine — same as Frame Lights.

---

## Rendering Feasibility

### Option A: Colored blocks only ✅ SAFE NOW

Tab strip tabs are rendered as solid colored rectangles in the top rim band. No text. Sexdisplay already has all the rendering primitives needed (colored pixel output in `composite_pixel`).

**What sexdisplay needs to know:**
- Which surfaces have tabs (frame-owned surfaces)
- How many tabs
- Which tab is active

**IPC requirement:** Sexdisplay currently has no tab metadata. The shell must communicate:
- `tab_count` for each frame-owned surface
- `active_tab` index

**IPC options:**
1. **New opcode (0xFD):** Shell sends `(surface_id, tab_count, active_tab)` to sexdisplay. Sexdisplay stores in a parallel array.
2. **Encode in surface metadata:** Use a reserved field in an existing opcode.
3. **V1-only assumption:** Sexdisplay hardcodes "surface 100 has 1 tab, active tab 0". Fragile, not extensible, but zero IPC.

**Recommended for V1:** New opcode `0xFD` with `arg0=surface_id`, `arg1=tab_count`, `arg2=active_tab`. Minimal — 1 new opcode, ~15 lines on each side. If block on ABI change, Option 3 can be a temporary bridge but is not recommended for production.

### Option B: Digit/title_id only ⚠️ FEASIBLE WITH CONSTRAINTS

`title_id: u64` could encode a small integer 0-9, which sexdisplay could render using the existing 5×7 clock font digit bitmap. Each tab would show a single digit.

**Limitations:**
- Only 10 distinct tab identities (0-9)
- 5×7 digit at 4px height doesn't fit (digit is 7px tall, rim is 4px)
- Would need to increase `FRAME_TAB_STRIP_PX` to at least 8px
- No semantic meaning (what does "3" mean as a tab label?)
- Tab strip would no longer be same height as rim

**Not recommended for V1.** Better to use colored blocks until a proper text renderer exists.

### Option C: Blocked pending text renderer ❌

Full text labels on tabs require:
1. General ASCII glyph bitmap array (not just digits 0-9)
2. Glyph dimensions + layout engine
3. String storage for tab titles (no heap in V1 — fixed-size buffers needed)
4. Tab strip height must accommodate glyph height (at least 8-10px)
5. Selection/hover highlighting for text

**Blockers:**
- No string type (`no_std`, no `alloc`)
- No font beyond 5×7 digits
- No text layout engine
- 4px rim too small for any readable text

**Removed blocker (alloc):** Actually, silk-shell uses `extern crate alloc` and `Vec`. Strings via fixed-size `heapless::String` or `arrayvec` would work. But the font and layout engine are still missing.

---

## Proposed Tab Strip Hit-Target Model

### Priority order in top rim

1. Frame Lights (highest) — CLOSE, MINIMIZE, ZOOM
2. Tab strip blocks — individual tab hit targets
3. Rim drag (lowest) — non-light, non-tab rim area

### Hit target kinds

| Kind | Constant | Action |
|------|----------|--------|
| `FRAME_CHROME_RIM` | 1 | Drag (on non-light rim) |
| `FRAME_CHROME_TAB_STRIP` | 2 | Tab selection (future) |
| `FRAME_CHROME_TAB_N` | 3+ | Individual tab click targets (future) |

**V1:** Tab strip clicks are captured as no-op (same as current behavior at line 1431). No action behavior.

**Future:** Each tab block would need a distinguishable hit target so the shell knows which tab index was clicked. Options:
- Extend `FrameChrome` to include `tab_index: u8`
- Use a range of `kind` values (e.g., `FRAME_CHROME_TAB_0 = 10`, `FRAME_CHROME_TAB_1 = 11`, ...)

### Light exclusion zone

The `hit_test_surface_chrome()` tab strip check must exclude the Frame Lights zone:

```rust
// Current (exclusion missing):
// if y >= strip_top && y < strip_bot && x >= sx && x < (sx + sw as i32) {

// Proposed (exclude lights zone):
let lights_zone_end = sx + FRAME_LIGHT_GAP_PX + 3 * (FRAME_LIGHT_SIZE_PX + FRAME_LIGHT_GAP_PX);
if y >= strip_top && y < strip_bot && x >= lights_zone_end && x < (sx + sw as i32) {
```

This prevents tab strip hits from overlapping lights. Lights are already checked before rim drag in `click_hit_test_and_focus()`, but the hit target should be precise.

---

## Hover-Reveal Label Design (Single-Tab)

For single-tab frames, the tab identity could be revealed on hover:

| Aspect | V1 Option | Future Option |
|--------|-----------|---------------|
| Visual feedback | Tab block color change on hover | Text label overlay |
| Tab identity | `title_id` (0-255) → colored dot | String title → rendered text |
| Position | Same tab block, different color | Pop-out label or tooltip |
| IPC needed | None (shell-local color choice) | Text renderer in sexdisplay |

**V1:** On hover over the tab strip (`HOVER_KIND == HOVER_TAB_STRIP`), the shell changes the tab block's rendered color. This could be done via 0xEF fill rect on the tab block area — but fill rect doesn't draw on the rim band (composite_pixel checks rim first).

Actually, the shell could send a new fill rect that covers the tab block position... but composite_pixel checks rim first. The rim check exits with rim color before the fill_rect_color call.

**Simplest V1 hover:** No hover feedback on tab strip. The tab block just shows a single color. Hover state is tracked internally by the shell (via `HOVER_KIND`) but has no visible effect until a future text/tooltip phase.

---

## Implementation Files

### Modified for V1 tab strip

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | Set `FRAME_TAB_STRIP_PX = 4`, add light-exclusion zone in `hit_test_surface_chrome()`, possibly send tab IPC |
| `servers/sexdisplay/src/main.rs` | Add tab block rendering in `composite_pixel()` Pass 2 (top rim, after lights), handle tab IPC |

### New IPC (if ABI change is allowed)

| File | Changes |
|------|---------|
| `crates/sex-pdx/src/lib.rs` | Add `OP_SURFACE_TAB_INFO = 0xFD` constant |
| `servers/silk-shell/src/main.rs` | Send `pdx_call(SLOT_DISPLAY, 0xFD, surface_id, tab_count, active_tab)` on tab changes |
| `servers/sexdisplay/src/main.rs` | Handle `0xFD`: store tab info in a `TAB_INFO: [TabInfo; 16]` array, keyed by surface_id |

### NOT Modified

- `kernel/` — no kernel ABI changes
- `crates/silkbar-model/` — no model changes
- `servers/silkbar/` — no forwarding changes
- `servers/sexusb/` — no synthetic proof changes
- `servers/sexinput/` — untouched
- Any framebuffer path — untouched

---

## Diagnostic Markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.tab_strip.hover] frame=N tab=N kind=N` | 8 | Hover over tab strip block |
| `[shell.tab_strip.click] frame=N tab=N` | 8 | Click on tab strip block (future) |
| `[sexdisplay.tab_strip.update] sid=N tabs=N active=N` | 8 | Sexdisplay receives tab metadata |

Pre-existing markers that must still fire:

| Marker | Status |
|--------|--------|
| `[shell.frame.light.close/minimize/zoom]` | Lights still work (higher priority than tabs) |
| `[shell.frame.rim.drag.start]` | Rim drag on non-light, non-tab rim clicks |
| `[shell.frame.zoom/unzoom]` | ZOOM light toggle unchanged |
| `[shell.frame.minimize/restore]` | MINIMIZE unchanged |
| `[shell.drag.start/move/end]` | Rim/content drag unchanged |
| `[shell.hit_target.chrome]` | Chrome hit targets still produced |

---

## STOP Conditions

If any of these are encountered during implementation, STOP and re-assess:

1. **Tab strip IPC blocked by ABI policy** — if the project forbids new opcodes, use V1-only hardcoded assumption (surface 100 = 1 tab) or find an alternative encoding.

2. **Tab strip height exceeds rim height** — if `FRAME_TAB_STRIP_PX > FRAME_RIM_PX`, the tab strip extends below the rim. This requires rethinking the top band layout and modifying composite_pixel structure.

3. **Light zone exclusion is incorrect** — if the computed lights zone width doesn't match actual light positions, tab strip hits could obscure light actions.

4. **Text rendering required** — if the design requires text on tabs, it is BLOCKED. The text renderer prerequisite is a separate phase.

5. **Tab strip overlaps right rim** — tabs must not extend into the right 4px rim band (or the right rim disappears). The right rim is needed for visual symmetry.

---

## Next Implementation Phase

### FRAME_TAB_STRIP_IPC_V1

```
MISSION: FRAME_TAB_STRIP_IPC_V1

Implement tab strip protocol + rendering. Shell + sexdisplay.

Design complete in FRAME_TAB_STRIP_PLAN_V1.md.

Changes:
1. servers/sexdisplay/src/main.rs:
   - Add TAB_INFO array (surface_id → {tab_count, active_tab})
   - Handle 0xFD opcode (store tab metadata)
   - In composite_pixel() Pass 2, top rim band, after lights:
     render colored tab blocks based on TAB_INFO
   - Tab block color: active = FOCUS_SURFACE_COLOR, inactive = darker

2. servers/silk-shell/src/main.rs:
   - Set FRAME_TAB_STRIP_PX = 4
   - Fix hit_test_surface_chrome() to exclude lights zone from tab strip
   - On frame init or tab change, send 0xFD with tab metadata

3. crates/sex-pdx/src/lib.rs:
   - Add OP_SURFACE_TAB_INFO = 0xFD

Forbidden:
- Text rendering
- Tab switching behavior
- Dynamic allocation
- Framebuffer path changes
- Broad compositor rewrite

PASS:
- Default build passes
- Synthetic build passes
- Colored tab block visible in top rim after lights on focused surface
- Frame Lights still work when clicked
- Rim drag still works on non-light, non-tab rim
- No text rendered on tabs
- Tab block disappears if surface has no tabs (tab_count = 0)
```

### Alternative: FRAME_TAB_STRIP_MODEL_V1 (shell-only)

If ABI changes are blocked, a shell-only phase that:
- Enables `FRAME_TAB_STRIP_PX = 4` in hit-targets
- Adds light-exclusion zone to tab strip hit test
- Adds hover tracking for tab strip
- Does NOT change sexdisplay (no visual tab strip yet)
- Defers rendering to a later ABI-allowed phase
