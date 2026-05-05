# L1: Multi-Rect Display STOP FIRST Design

**Status:** STOP FIRST design only — no code changes.
**Date:** 2026-05-05
**Purpose:** Design the smallest safe path for multiple visible fill-rect regions
per surface in sexdisplay, enabling per-row visual feedback for Linen object
lists, Quil buffer lists, command palette rows, Bell events, Mesh diagnostic
facts, and Collar grant status.

## Verdict

```
╔══════════════════════════════════════════════════════════════════╗
║   SAFE_EXISTING_OPS + MINIMAL_STATE_CHANGE (see category rows)  ║
╠══════════════════════════════════════════════════════════════════╣
║                                                                  ║
║  SAFE_EXISTING_OPS:             YES (reuse 0xEF, no ABI edit)   ║
║  SAFE_SHELL_BATCH_ONLY:         NO  (sexdisplay single rect)     ║
║  BLOCKED_ABI_REQUIRED:          NO  (rect_index avoids new op)   ║
║  BLOCKED_RENDERER_POLICY_RISK:  LOW (array iterate, no policy)  ║
║  Existing 0xEF opcode:          SAFE (reuse, no ABI change)      ║
║  Sexdisplay state change:       REQUIRED (array fill rects)      ║
║  Sex-pdx ABI change:            NOT REQUIRED                     ║
║  Kernel change:                 NOT REQUIRED                     ║
║  Shell-side change:             REQUIRED (send N 0xEF calls)     ║
║  Renderer policy boundary:      INTAKT (sexdisplay still dumb)   ║
║  Framebuffer bounds checks:     PRESERVED                        ║
║  MAX_RECTS:                     8                                ║
║                                                                  ║
╚══════════════════════════════════════════════════════════════════╝
```

**Chosen approach:** Add a rect-index field to existing 0xEF opcode (reusing
bits 24-27 of arg2, currently zero-padding), and change sexdisplay's `Surface`
struct to store an array of `MAX_RECTS` fill rects.
No new opcode. No sex-pdx ABI symbol change. No kernel edit.
Sexdisplay remains a dumb rectangle renderer. Silk-shell sends N sequential
0xEF calls, one per rect, each specifying which rect slot to write.

## Current 0xEF Behavior

**Source:** `servers/sexdisplay/src/main.rs` lines 1073-1125

### Call Format

```
0xEF: arg0=surface_id, arg1=(sy<<32)|sx, arg2=(color<<32)|(sh<<16)|sw
```

All fields are packed into three 64-bit register arguments (standard PDX ABI).

### Current Sexdisplay Handler (line 1073)

| Step | Operation | Source Reference |
|------|-----------|-----------------|
| 1 | Extract surface_id from arg0 | line 1076 |
| 2 | Skip if surface_id=0 or framebuffer not live | lines 1077-1078 |
| 3 | Extract sx, sy, sw, sh, color from arg1/arg2 | lines 1080-1084 |
| 4 | Skip if sw=0 or sh=0 | line 1085 |
| 5 | Search SURFACES for active slot matching surface_id | lines 1089-1091 |
| 6 | Verify caller_pd == owner_pd (authorization gate) | lines 1093-1099 |
| 7 | **Clamp** sw=sw.min(slot.w), sh=sh.min(slot.h) | lines 1102-1103 |
| 8 | **Clamp** fill_sx=sx.clamp(0, max_sx), fill_sy=sy.clamp(0, max_sy) | lines 1106-1109 |
| 9 | **Overwrite** single fill rect: slot.fill_sx/fill_sy/fill_sw/fill_sh/fill_color/fill_active | lines 1111-1116 |
| 10 | Call redraw_surface_area() | line 1122 |

### Current Storage (lines 28-49)

```rust
struct Surface {
    // ... (geometry, ownership, chrome fields)
    // Per-surface fill rect (V1: single rect, last 0xEF wins)
    fill_sx: i32,
    fill_sy: i32,
    fill_sw: u32,
    fill_sh: u32,
    fill_color: u32,
    fill_active: bool,
}
```

**Key insight:** Each surface stores exactly ONE fill rect. A second 0xEF call
to the same surface overwrites it. Only the last 0xEF call's rect is visible.

### Composite Pixel Flow (lines 164-178, 303-317)

`composite_pixel()` calls `fill_rect_color()` for each surface. `fill_rect_color()`
checks **one** fill rect per surface. Returns `fill_color` if the pixel falls within
that rect, otherwise returns `base_color`.

## Sexdisplay Bounds-Check Proof

Sexdisplay applies **two levels** of bounds clamping before any fill rect
is drawn, making it impossible for a shell-provided rect to exceed the
surface's actual dimensions or the framebuffer:

### Level 1: Width/Height Clamping (line 1102-1103)
```rust
sw = sw.min(slot.w);
sh = sh.min(slot.h);
```
Any rect wider or taller than the surface is silently clamped to the surface
dimensions. If sw or sh becomes 0 after clamping, the fill is skipped entirely.

### Level 2: Position Clamping (lines 1106-1109)
```rust
let max_sx = slot.w.saturating_sub(sw) as i32;
let max_sy = slot.h.saturating_sub(sh) as i32;
let fill_sx = sx.clamp(0, max_sx);
let fill_sy = sy.clamp(0, max_sy);
```
The rect position is clamped so that the rect always fits within the surface.
If the surface is smaller than the rect, position clamps to (0,0) and the
size clamp from Level 1 already handles the overflow.

### Level 3: Framebuffer Clipping (in `clamp_surface`, lines 148-161)
```
x = surf.x.max(0).min(fb_w - 1)
y = surf.y.max(BAR_H).min(fb_h - 1)
w = surf.w.min(fb_w - x)
h = surf.h.min(fb_h - y)
```
The surface itself is clamped to framebuffer bounds before compositing.
Since fill rects are relative to the surface and clamped to surface bounds,
they are transitively bounded by the framebuffer.

**Verdict: DOUBLE-BOUNDED SAFETY. No change to bounds checking needed.**

## Option Analysis

### Option A: Array Fill Rects in Sexdisplay (RECOMMENDED)

**Approach:** Change `Surface` struct fill fields to arrays of `MAX_RECTS=8`.
Each 0xEF call specifies which rect slot to write via a rect_index field.
`fill_rect_color()` iterates all rects, last-match wins.

**Changes to sexdisplay:**
- `fill_sx` → `fill_sx: [i32; MAX_RECTS]`
- `fill_sy` → `fill_sy: [i32; MAX_RECTS]`
- `fill_sw` → `fill_sw: [u32; MAX_RECTS]`
- `fill_sh` → `fill_sh: [u32; MAX_RECTS]`
- `fill_color` → `fill_color: [u32; MAX_RECTS]`
- `fill_active: bool` → `fill_count: u8` (0 = no rects, 1-8 = active rects)
- 0xEF handler: extract rect_index from bits 24-27 of arg2, write to slot[rect_index]
- `fill_rect_color()`: iterate 0..fill_count, update color for each rect hit

**BLOCKED_RENDERER_POLICY_RISK: LOW.** The sexdisplay renderer change is limited to iterating a fixed-size array in fill_rect_color(). No policy interpretation (sexdisplay does not know what a row is). No alpha blending. No z-ordering between rects. The renderer remains a dumb compositor. Bounds checks unchanged. This is a low-risk renderer change: it replaces a single-rect check with an array iteration of the same check.

**Storage cost:** 5 fields × 8 rects = 40 values. At ~4 bytes each = ~160 bytes.
For 16 surfaces = ~2560 bytes total. Negligible for a kernel with 4G+ RAM.

**Backward compatibility:** If rect_index=0 and fill_count was 0, behavior is
identical to current (single rect). New shell can set rect_index 0..7.

**Advantages:**
- No new opcode. No sex-pdx ABI edit.
- Sexdisplay remains dumb renderer (just stores and iterates rects).
- Bounds checks unchanged.
- Shell controls rect policy (what, where, color, order).
- MAX_RECTS=8 is a hard compile-time bound; no heap.

**Disadvantages:**
- Requires sexdisplay source change (Surface struct + handler).
- Requires shell change to send N 0xEF calls per render.

### SAFE_SHELL_BATCH_ONLY Analysis

**Verdict: NOT VIABLE as standalone approach.**

Shell-local batching without sexdisplay changes cannot produce multiple visible rectangles per surface because:

1. **0xEF overwrites, not appends.** Each call overwrites the single fill rect.
2. **No accumulation mode.** Sexdisplay has no add-rect-to-list mode.
3. **No framebuffer readback.** Shell cannot composite rows locally.

Shell batching is only useful when combined with sexdisplay array storage (Option A).
The shell batches N 0xEF calls with rect_index 0..N-1, and sexdisplay stores all N rects.

If implemented as standalone (no sexdisplay change), shell batching produces **zero** visible multi-rect output. Proof markers show N rows, but framebuffer shows only the last rect.

### Option B: Multiple Small Surfaces (NOT RECOMMENDED)

**Approach:** Create one shell surface per row. Each surface gets its own
single fill rect.

**Problems:**
- Each row surface triggers chrome compositing (top bar, tab blocks, frame
  lights, rim). Many of these are irrelevant for row surfaces.
- Surface slot pressure: MAX_SURFACES=16. 6 row surfaces would consume nearly
  half the pool.
- Each surface requires full geometry management in silk-shell (position,
  z-order, visibility tracking).
- Frame/tab/chrome model is not designed for sub-surface row primitives.
- No clear ownership boundary: do rows have frames? Tabs? Lifecycle states?

**Verdict: BLOCKED. Too much architectural friction for too little gain.**

### Option C: New PDX Opcode (NOT RECOMMENDED)

**Approach:** New opcode like 0xF5 OP_SURFACE_FILL_RECTS that sends multiple
rect definitions in a single call.

**Problems:**
- Requires sex-pdx ABI edit (new opcode constant, new dispatch arm).
- Requires sexdisplay to parse a batched argument format.
- More complex than Option A for equivalent end result.
- No architectural benefit over packing a rect index into existing 0xEF.

**Verdict: BLOCKED_ABI_REQUIRED — avoid unless Option A is impossible.**

### Option D: Shell-Local Compositing Surface (IMPOSSIBLE)

**Approach:** Shell creates a temporary surface, draws all rows via repeated
0xEF, reads back the composited result, transfers to target surface.

**Problems:**
- Each 0xEF overwrites the previous fill rect. Only the last rect is visible.
- sexdisplay has no "accumulate blend" mode. Each fill replaces.
- No framebuffer readback API exists (sexdisplay is sole writer by design).
- Fundamentally incompatible with current sexdisplay architecture.

**Verdict: REJECTED — architecturally impossible without shared-memory
framebuffer access (forbidden).**

## Recommended Approach: Option A (Array Fill Rects)

### Exact Sexdisplay Changes

**1. Surface struct change (line 28-49):**
```rust
const MAX_RECTS: usize = 8;

struct Surface {
    surface_id: u64,
    owner_pd: u32,
    x: i32, y: i32, w: u32, h: u32,
    color: u32,
    active: bool,
    tab_count: u8,
    active_tab: u8,
    chrome_flags: u8,
    // Multi-rect fill storage (up to MAX_RECTS fill rects per surface)
    fill_count: u8,           // 0 = no fill rects, 1-8 = how many rects active
    fill_sx: [i32; MAX_RECTS],
    fill_sy: [i32; MAX_RECTS],
    fill_sw: [u32; MAX_RECTS],
    fill_sh: [u32; MAX_RECTS],
    fill_color: [u32; MAX_RECTS],
}
```

**2. 0xEF handler change (line 1073-1125):**
```rust
// Extract rect_index from bits 24-27 of arg2 (currently zero padding)
// Format: arg2 = (color << 32) | (rect_index << 24) | (sh << 16) | sw
let rect_index = ((msg.arg2 >> 24) & 0xF) as usize;
if rect_index >= MAX_RECTS { continue; }

// When rect_index == 0 and fill_count == 0, this is the first rect.
// When rect_index > 0, this is an additional rect.
// Shell sends rects in order 0, 1, 2, ... N-1.
// fill_count = max(fill_count, rect_index + 1)

// ... existing bounds checks ...

slot.fill_sx[rect_index] = fill_sx;
slot.fill_sy[rect_index] = fill_sy;
slot.fill_sw[rect_index] = sw;
slot.fill_sh[rect_index] = sh;
slot.fill_color[rect_index] = color;
if rect_index + 1 > slot.fill_count as usize {
    slot.fill_count = (rect_index + 1) as u8;
}
```

**3. fill_rect_color() change (line 306-317):**
```rust
fn fill_rect_color(surf: &Surface, x: usize, y: usize, base_color: u32) -> u32 {
    let mut c = base_color;
    let lx = (x as i32) - surf.x;
    let ly = (y as i32) - surf.y;
    // Iterate all active rects, last match wins (painter's algorithm).
    // Rect 0 = background (first to be checked but may be overdrawn).
    for i in 0..surf.fill_count as usize {
        if lx >= surf.fill_sx[i] && lx < surf.fill_sx[i] + surf.fill_sw[i] as i32
            && ly >= surf.fill_sy[i] && ly < surf.fill_sy[i] + surf.fill_sh[i] as i32
        {
            c = surf.fill_color[i];
            // Don't break — later rects overdraw earlier ones.
        }
    }
    c
}
```

**4. SURFACE_EMPTY change (line 52-56):**
Zero-initialized arrays: `[0i32; MAX_RECTS]`, `[0u32; MAX_RECTS]`, `fill_count: 0`.

**5. Redraw strategy:** Each 0xEF call immediately calls `redraw_surface_area()`.
This matches current behavior and is simplest. If performance is a concern,
batch redraw can be added later (shell sets a "commit" flag on the last rect).
For V1, immediate redraw per call is acceptable (MAX_RECTS=8 max redraws).

### Exact Silk-Shell Changes

**1. Render functions** (`linen_render_object_list`, `quil_render_buffer_list`,
`palette_render_list`) change from one 0xEF call to N 0xEF calls:

```rust
// Example: linen_render_object_list sends:
// 0xEF rect 0: header bar (as now)
// 0xEF rect 1..N: one per row, with rect_index = row_index + 1
unsafe fn linen_render_object_list() {
    let header_color = linen_selected_object_accent();
    // Send header rect (rect_index=0, same as current behavior)
    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_LINEN,
        (HEADER_HEIGHT as u64) << 32 | 0,  // (sy << 32) | sx
        (header_color as u64) << 32 | (HEADER_HEIGHT as u64) << 16 | w);
    
    // Send row rects (rect_index=1..N)
    let mut row_count = 0u64;
    for y in (HEADER_HEIGHT + ROW_GAP ..).step_by(ROW_H + ROW_GAP).take(MAX_OBJECT_ROWS) {
        // rect_index = row_index + 1 (since rect 0 = header)
        let rect_index = ((row_index + 1) & 0xF) as u64;
        let color = /* per-row color based on object kind, selection state */;
        pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_LINEN,
            (y as u64) << 32 | 0,
            (color as u64) << 32 | (rect_index as u64) << 24 | (ROW_H as u64) << 16 | w);
        row_count += 1;
    }
}
```

**2. No model changes.** The row data (object list, buffer list, commands)
is already computed in proof-marker-only form. The only change is emitting
actual 0xEF fill rects for each row instead of just proof markers.

### Ownership Split

| Responsibility | silk-shell (PKEY 3) | sexdisplay (PKEY 1) |
|---------------|---------------------|---------------------|
| Which rows to show | ✅ Policy: selects objects/buffers/commands | ❌ Not interpreted |
| Row order | ✅ Sorts and orders rows | ❌ Draws in array order |
| Row colors | ✅ Selects per-row fill colors | ❌ Stores and composites |
| Rectangle count | ✅ Sets fill_count | ❌ Iterates fill_count rects |
| Geometry | ✅ Controls which surface + position | ❌ Clamps to surface bounds |
| Drawing | ❌ Never writes framebuffer | ✅ Composites rects per pixel |
| Authorization | ✅ Already owns its surfaces | ✅ Checks caller_pd == owner_pd |
| Bounds checks | ❌ Relies on sexdisplay clamping | ✅ Double-bounds all geometry |

**Sexdisplay remains a dumb rectangle renderer.** It does not interpret row
semantics, color meaning, or ordering policy. It stores what it receives,
clamps to surface bounds, and composites.

### Why MAX_RECTS = 8

| Use Case | Rect 0 | Rects 1-7 | Max |
|----------|--------|-----------|-----|
| Linen object list | Header | Up to 7 row highlights | 8 |
| Quil buffer list | Header | Up to 7 row highlights | 8 |
| Command palette | Header | Up to 7 row highlights | 8 |
| Bell event list | Header | Up to 7 event highlights | 8 |
| Mesh diagnostic facts | Header | Up to 7 link highlights | 8 |
| Collar grant status | Header | Up to 7 grant highlights | 8 |

8 is sufficient for all known use cases. If more rows are needed later,
MAX_RECTS can be increased (it's a compile-time constant). The hard upper
bound is limited only by surface height (e.g., a 720px surface with 28px
rows = ~25 rows max, but shell policy can limit to 7 data rows + 1 header).

## STOP FIRST Table

| Item | Why STOP FIRST |
|------|----------------|
| Multi-rect requires sexdisplay struct change | This document. Option A is the minimal change. |
| Any new PDX opcode | sex-pdx ABI edit. Option A avoids this. |
| sexdisplay policy interpretation | Renderer must remain dumb. Option A preserves this. |
| Shared-memory/backing-buffer redesign | Architectural change. Option A avoids this. |
| Framebuffer bounds removal | Safety invariant. Option A preserves all bounds. |
| Kernel or MPK/PKEY change | Not needed for Option A. |
| Heap-backed rect storage | Option A uses fixed array; no heap. |
| Shell-batch-only (no sexdisplay change) | Blocked: sexdisplay single rect cannot produce multi-rect output. |
| Renderer policy interpretation | Would break dumb-renderer boundary. Option A avoids (array iteration, not policy). |

## Forbidden Approaches

| Approach | Reason |
|----------|--------|
| Shared-memory framebuffer access from shell | Breaks sexdisplay sole-writer invariant; MPK violation |
| New PDX opcode for multi-rect | Unnecessary complexity when rect_index in existing 0xEF works |
| Multiple full surfaces per row | MAX_SURFACES pressure; unnecessary chrome/lifecycle overhead |
| Shell-side compositing into temp surface | Each 0xEF overwrites; no readback API; fundamental incompatibility |
| Sexdisplay interpreting row semantics | Breaks dumb-renderer boundary; policy must stay in shell |
| Dynamic/ unbounded rect count | Must be compile-time fixed array; no heap |

## Recommended Consumer Order

After L1 design approval and implementation:

### L2: Linen Selected Row Visual (First Consumer)
First consumer. L2 Linen selected row visual highlights the currently selected
Linen object by applying a distinct fill rect behind its row. Row colors derived from
`linen_kind_color()`. Selected row gets accent color from
`linen_selected_object_accent()`. Header rect + 6 row rects = 7 total (≤ 8).

### L3: Quil Buffer Row Highlights
Second consumer. Quil buffer list has 6 seed buffers + potentially dynamic
buffers. Same pattern as Linen. Header rect + buffer row rects.

### L4: Command Palette Row Highlights
Third consumer. 5 commands + header = 6 fill rects. Simple.

Later consumers (after proven pattern): Bell event list, Mesh diagnostic
row highlights, Collar grant status display.

## Exact L2 Implementation Prompt

```
MISSION: L2_LINEN_ROW_VISUALS

Goal:
Give Linen object list real per-row visual feedback using the multi-rect
display capability from L1. Each of the 6 seed object rows gets its own
0xEF fill rect with color from linen_kind_color(). Selected row gets
header accent color.

Prerequisite:
L1 sexdisplay multi-rect changes must be deployed.

Changes:
servers/silk-shell/src/main.rs:
- linen_render_object_list(): change from 1 to 7 0xEF calls
  (1 header + 6 rows, rect_index 0-6)
- Emits [linen.row.fill] N for each rendered row rect
- Emits [linen.row.fill.done] count=N

No new model state. No new functions.
Row colors from linen_kind_color() + linen_selected_object_accent().
Geometry: rows stack below header at ROW_H + ROW_GAP spacing.
Selected row gets bold accent background.
Non-selected rows get muted kind color background.

Proof markers:
- [linen.row.fill] index=N kind=K color=C selected=true/false
- [linen.row.fill.done] count=N rects=N

Forbidden:
- No sexdisplay changes (L1 deploys them first)
- No sex-pdx ABI changes
- No kernel changes
- No new display primitives
- No text rendering
- No heap
- No model changes to LinenObject or LinenObjectKind

Output:
docs/handoff/L2_LINEN_ROW_VISUALS_V1.md
Code changes to servers/silk-shell/src/main.rs only.

Build:
./scripts/entrypoint_build.sh
```

## Proof

**Document complete:** `docs/handoff/L1_MULTI_RECT_DISPLAY_STOP_FIRST_DESIGN_V1.md`

**All required sections present:**
- ✅ Verdict: SAFE_EXISTING_OPS + MINIMAL_STATE_CHANGE (4 category rows in box)
- ✅ Current 0xEF behavior summary (lines 1073-1125, fill_rect_color 306-317)
- ✅ Sexdisplay bounds-check proof (double-bounded: sw.min + sx.clamp)
- ✅ Whether repeated 0xEF calls are safe (yes, but same-surface calls overwrite)
- ✅ MAX_RECTS = 8 with per-use-case justification
- ✅ Repeated 0xEF call safety (authorized and bounds-checked per call; single rect overwrite)
- ✅ ownership split: silk-shell=policy, sexdisplay=dumb renderer
- ✅ Exact sexdisplay Surface struct change (arrays of MAX_RECTS)
- ✅ Exact sexdisplay 0xEF handler change (rect_index from bits 24-27)
- ✅ Exact fill_rect_color() change (iterate all rects, painter's)
- ✅ Exact shell-side render function change pattern
- ✅ STOP FIRST table (all clear)
- ✅ forbidden approaches: 5 rejected (shared-memory, new opcode, multi-surface, shell-local, policy)
- ✅ Recommended consumer order: L2 Linen → L3 Quil → L4 Command Palette
- ✅ Exact L2 implementation prompt
- ✅ L2 Linen selected row visual (first consumer)
