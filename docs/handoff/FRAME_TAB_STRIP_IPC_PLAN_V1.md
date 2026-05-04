# FRAME_TAB_STRIP_IPC_PLAN_V1

## Status

Design (2026-05-04). Tab metadata IPC path from silk-shell to sexdisplay. No code changed.

---

## Route Decision

### Route A: Direct 0xFD opcode ✅ RECOMMENDED

```
silk-shell (PDX 6)                            sexdisplay (PDX 4)
    │                                              │
    │ 0xFD (OP_SURFACE_TAB_INFO)                    │
    │  arg0 = surface_id                            │
    │  arg1 = tab_count                             │
    │  arg2 = active_tab                            │
    │ ────────────────────────────────────────────→│
    │                                              │
    │ (sent after 0xEC create, on tab changes)     │
    │                                              │
    │                                              │ store in Surface.tab_count
    │                                              │ store in Surface.active_tab
    │                                              │ render tab blocks in
    │                                              │ composite_pixel() Pass 2
```

### Routes Rejected

| Route | Reason |
|-------|--------|
| **B: Via silkbar** | Mixes concerns (bar UI state vs. compositor metadata). Adds unnecessary hop. SilkBar model would need new UpdateKind + ABI_VERSION bump. |
| **C: Reuse OP_SILKBAR_UPDATE** | Same as B — SilkBar updates go through silkbar server, not direct. Would still need forwarding. |
| **D: STOP** | Unnecessary. 0xFD is free, pattern is clean, ~15 lines each side. |

---

## Design Decisions

### 1. Opcode: 0xFD (OP_SURFACE_TAB_INFO)

**Confirmed free.** Not used in any .rs source file (only mentioned in design docs and memory map docs at 0xfd000000-0xfd3e8000 which is unrelated physical memory range).

Opcode space audit (`crates/sex-pdx/src/lib.rs`):

| Range | Usage |
|-------|-------|
| 0xE4-0xE8 | Legacy window operations (unused in V1) |
| 0xEB | OP_SURFACE_UPDATE (silk-shell → sexdisplay, position only) |
| 0xEC | OP_SURFACE_CREATE_ID (silk-shell → sexdisplay, create/upsert) |
| 0xED | OP_SET_FOCUS (silk-shell → sexdisplay) |
| 0xEE | OP_SURFACE_DESTROY (silk-shell → sexdisplay) |
| 0xEF | OP_SURFACE_FILL_RECT (silk-shell → sexdisplay) |
| 0xF0-0xF4 | SilkBar protocol (reserved) |
| 0xFD | **FREE** ← OP_SURFACE_TAB_INFO |
| 0x202 | HID event |
| 0x260 | USB mouse report |

Sexdisplay dispatch (line 791): match on `msg.type_id`. 0xFD handler inserts naturally between existing 0xEF and the `_ =>` catch-all.

### 2. Payload Layout

```
Opcode: 0xFD
arg0: surface_id   (u64) — which surface this tab info applies to
arg1: tab_count    (u64) — number of tabs (0 = no tabs, max 8)
arg2: active_tab   (u64) — index of active tab (0 to tab_count-1)
```

**Validation (sexdisplay):**
- `surface_id == 0` → silently ignore
- `tab_count > MAX_TABS_PER_FRAME (8)` → clamp to MAX_TABS_PER_FRAME
- `active_tab >= tab_count` → clamp to tab_count-1 (if tab_count > 0)
- `tab_count == 0` → set both to 0 (clear tab info)

**No authentication check.** Tab info is chrome metadata, not surface content. Follows the same pattern as 0xED (set focus), which is "compositor state — open to all callers." The shell is the sole authority for chrome policy, and in practice the shell owns all surfaces it creates via 0xEC.

### 3. Storage: Embed in Surface struct

```rust
struct Surface {
    surface_id: u64,
    owner_pd: u32,
    x: i32, y: i32, w: u32, h: u32,
    color: u32,
    active: bool,
    // Per-surface tab info (V1: updated by 0xFD from shell)
    tab_count: u8,
    active_tab: u8,
    // Per-surface fill rect
    fill_sx: i32,
    fill_sy: i32,
    fill_sw: u32,
    fill_sh: u32,
    fill_color: u32,
    fill_active: bool,
}
```

**Why embed in Surface (Option A) vs. separate array (Option B)?**

| Criterion | Option A: Embed | Option B: Separate |
|-----------|-----------------|-------------------|
| Lookup cost | Zero (fields are on the Surface being iterated) | O(n) scan or hash |
| Code complexity | +2 fields, ~20 bytes total | New type, new array, new scan |
| Memory waste | 2 bytes per surface (32 bytes total) | 10+ bytes per entry + array overhead |
| Cache behavior | Co-located with surface data | Separate cache line |

**Verdict: Option A.** The composite_pixel loop already has the `surf` reference. Reading `surf.tab_count` and `surf.active_tab` is free. A separate array requires a second lookup during pixel compositing, which is called per-pixel.

**SURFACE_EMPTY initializer update:**
```rust
const SURFACE_EMPTY: Surface = Surface {
    surface_id: 0, owner_pd: 0, x: 0, y: 0, w: 0, h: 0,
    color: 0, active: false,
    tab_count: 0, active_tab: 0,  // NEW
    fill_sx: 0, fill_sy: 0, fill_sw: 0, fill_sh: 0,
    fill_color: 0, fill_active: false,
};
```

### 4. Tab Block Rendering in composite_pixel()

**Location:** Pass 2 (focused surface), top rim band (`ly < FRAME_RIM_PX`), after Frame Lights checks, before the rim-color fallback.

**Current code (line 119-147):**
```rust
if ly < FRAME_RIM_PX {
    // CLOSE check
    if lx >= FRAME_LIGHT_GAP_PX && lx < FRAME_LIGHT_GAP_PX + FRAME_LIGHT_SIZE_PX {
        c = FRAME_LIGHT_CLOSE_COLOR;
    }
    // MINIMIZE check
    else if lx >= FRAME_LIGHT_GAP_PX + FRAME_LIGHT_SIZE_PX + FRAME_LIGHT_GAP_PX
        && lx < ... + FRAME_LIGHT_SIZE_PX
    {
        c = FRAME_LIGHT_MINIMIZE_COLOR;
    }
    // ZOOM check
    else if lx >= FRAME_LIGHT_GAP_PX + 2 * (FRAME_LIGHT_SIZE_PX + FRAME_LIGHT_GAP_PX)
        && lx < ... + FRAME_LIGHT_SIZE_PX
    {
        c = FRAME_LIGHT_ZOOM_COLOR;
    } else {
        c = FRAME_RIM_COLOR;  // ← tab strip renders here
    }
}
```

**Modified: tab strip inserted before rim fallback:**
```rust
if ly < FRAME_RIM_PX {
    // Light checks (unchanged)...
    if lx >= FRAME_LIGHT_GAP_PX ... { c = FRAME_LIGHT_CLOSE_COLOR; }
    else if ... { c = FRAME_LIGHT_MINIMIZE_COLOR; }
    else if ... { c = FRAME_LIGHT_ZOOM_COLOR; }
    // Tab strip: after light exclusion zone, before right rim
    else if surf.tab_count > 0
        && lx >= TAB_STRIP_LIGHT_EXCLUSION_PX
        && lx < rim_right
    {
        let tab_strip_start = TAB_STRIP_LIGHT_EXCLUSION_PX;
        let available = rim_right - tab_strip_start;
        let slot_w = available / surf.tab_count as usize;
        if slot_w > 0 {
            let tab_idx = (lx - tab_strip_start) / slot_w;
            if tab_idx == surf.active_tab as usize {
                c = TAB_ACTIVE_COLOR;
            } else {
                c = TAB_INACTIVE_COLOR;
            }
        } else {
            c = FRAME_RIM_COLOR;
        }
    }
    else { c = FRAME_RIM_COLOR; }
}
```

**Tab colors:**

| Role | Color | Value |
|------|-------|-------|
| Active tab | FOCUS_SURFACE_COLOR (cyan) | `0x00A8E0FF` |
| Inactive tab | Tab inactive (dimmer cyan) | `0x006080B0` |

**Constants to add in sexdisplay:**
```rust
const TAB_STRIP_LIGHT_EXCLUSION_PX: usize = 20; // matches shell FRAME_TAB_LIGHT_EXCLUSION_PX
const TAB_ACTIVE_COLOR: u32 = FOCUS_SURFACE_COLOR; // 0x00A8E0FF
const TAB_INACTIVE_COLOR: u32 = 0x006080B0;
```

### 5. Send Timing

**Boot (silk-shell _start, after frame init):**
```rust
// After 0xEC surface 100 create and 0xED focus set (line ~1692-1703):
pdx_call(SLOT_DISPLAY, 0xFD, SURFACE_ID_APP, 1, 0); // 1 tab, active tab 0
```

**Future (tab switching):**
When `active_tab` changes, re-send 0xFD for the frame's surface with updated `active_tab`.
When `tab_count` changes (tab open/close), re-send with updated count.

### 6. No ABI_VERSION Change

The ABI_VERSION in `silkbar-model` (currently 3) covers the SilkBar model protocol. The 0xFD opcode is a direct silk-shell → sexdisplay convention, orthogonal to the SilkBar model. No version negotiation needed.

### 7. No Kernel/PDX ABI Change

0xFD is a userland opcode. It uses the existing `pdx_call(SLOT_DISPLAY, ...)` path. No new syscalls, no capability slot changes, no kernel modifications.

---

## Files Changed

| File | Changes |
|------|---------|
| `crates/sex-pdx/src/lib.rs` | Add `OP_SURFACE_TAB_INFO = 0xFD` constant |
| `servers/sexdisplay/src/main.rs` | Add `tab_count: u8, active_tab: u8` to Surface struct. Add `TAB_STRIP_LIGHT_EXCLUSION_PX`, `TAB_ACTIVE_COLOR`, `TAB_INACTIVE_COLOR` constants. Handle 0xFD opcode in dispatch. Render tab blocks in `composite_pixel()` Pass 2. |
| `servers/silk-shell/src/main.rs` | Send `pdx_call(SLOT_DISPLAY, 0xFD, SURFACE_ID_APP, 1, 0)` after boot surface creation. (Future: send on tab changes.) |

### NOT Modified

- `kernel/` — no kernel ABI changes
- `crates/silkbar-model/` — no model changes
- `servers/silkbar/` — no forwarding changes
- `servers/sexusb/` — no synthetic proof changes
- `servers/sexinput/` — untouched
- `servers/sexdisplay/src/main.rs` render proof — unchanged
- Any framebuffer path — untouched

---

## Tab Block Geometry

Tab blocks in the top rim band (4px height), after the light exclusion zone (20px from surface left edge):

```
Surface left edge (sx = 100)
  │
  ├── 0-19:  Frame Lights zone (close/minimize/zoom + gaps)
  │
  ├── 20+:   TAB STRIP zone (lx relative to sx)
  │           equal-width slots filling (rim_right - 20) pixels
  │           rim_right = sw - FRAME_RIM_PX
  │
  └── right rim (last 4px): excluded from tab strip
```

For boot geometry (sx=100, sw=800, fr=4):
- `tab_strip_start` = 20 (lx)
- `rim_right` = 800 - 4 = 796 (lx)
- `available` = 796 - 20 = 776px
- `slot_w` = 776 / 1 = 776px
- Tab 0 covers lx=20..796 (entire strip)

For 4 tabs at same geometry:
- `slot_w` = 776 / 4 = 194px
- Tab 0: lx=20..214, Tab 1: lx=214..408, etc.

---

## Sexdisplay Opcode Handler (0xFD)

Insert between 0xEF handler (line ~979) and `_ =>` catch-all (line ~980):

```rust
0xFD => {
    // OP_SURFACE_TAB_INFO: arg0=surface_id, arg1=tab_count, arg2=active_tab
    let surface_id = msg.arg0;
    if surface_id == 0 { continue; }
    let tab_count = (msg.arg1 as u8).min(MAX_TABS_PER_FRAME);
    let active_tab = if tab_count > 0 {
        (msg.arg2 as u8).min(tab_count.saturating_sub(1))
    } else { 0 };
    unsafe {
        for slot in SURFACES.iter_mut() {
            if slot.active && slot.surface_id == surface_id {
                slot.tab_count = tab_count;
                slot.active_tab = active_tab;
                break;
            }
        }
    }
    // No redraw trigger needed — next composite will pick up new tab info.
    // If caller wants immediate redraw, shell can send 0xED or other trigger.
}
```

**Note on redraw:** The tab info change is metadata only. It doesn't trigger an immediate redraw. The next frame composite (triggered by the next mouse/keyboard event that causes a position/size/focus change) will pick up the new tab info. If the shell needs an immediate redraw after tab info change, it can send a dummy 0xEB/0xED. This is consistent with the existing pattern (0xEF fill rect also doesn't trigger redraw — wait, actually it does: `redraw_surface_area` is called at line 976).

Actually, looking more carefully: 0xEF does call `redraw_surface_area()` unconditionally after update. For 0xFD, an immediate redraw would also be needed to show the tab blocks. So add:

```rust
if tab_info_changed && fb_live {
    redraw_surface_area(FB_PTR as *mut u32, FB_W as usize, FB_H as usize);
}
```

---

## Budgeted Markers

### Sexdisplay (new)

| Marker | Budget | Fires |
|--------|--------|-------|
| `[sexdisplay.tab_info.update] sid=N tabs=N active=N` | 8 | Sexdisplay receives tab metadata from shell |

### Pre-existing markers that must still fire

| Marker | Status |
|--------|--------|
| `[shell.frame.tab.model]` | Tab strip model proof (boot) |
| `[shell.frame.light.model]` | Frame Lights model proof (boot) |
| `[shell.frame.light.close/minimize/zoom]` | Lights still work |
| `[shell.frame.rim.drag.start]` | Rim drag on non-light, non-tab rim clicks |
| `[shell.hit_target.chrome]` | Chrome hit targets still produced |
| `[sexdisplay.cursor.surface.update]` | Cursor position updates |
| `[silk.render_proof.top_strip.ok]` | Top strip rendering proof |

---

## STOP Conditions

If any of these are encountered during implementation, STOP and re-assess:

1. **0xFD collides with an existing or reserved opcode** — check `crates/sex-pdx/src/lib.rs` and all server dispatch match blocks. (Confirmed: 0xFD is free.)

2. **Surface struct size increase causes stack/bloat issues** — add `tab_count: u8, active_tab: u8` may increase struct size due to alignment padding. If struct grows significantly, consider Option B (separate array).

3. **Tab strip rendering breaks light behavior** — the tab strip check must come AFTER the three light checks and must not override light colors. Priority: light > tab > rim.

4. **Tab strip extends into right rim** — the `lx < rim_right` guard prevents this.

5. **Tab_count set on non-frame surfaces** — surfaces that aren't frame-owned should never receive 0xFD. The shell controls who sends 0xFD. If a rogue PD sends 0xFD with tab_count > 0, sexdisplay will render tab blocks on that surface. Low risk — only the shell has the DISPLAY capability.

6. **0xFD arrives before surface exists** — silently ignore (surface_id not found in SURFACES). The handler iterates and just doesn't find a match.

---

## Next Phase

### FRAME_TAB_STRIP_IPC_V1

```
MISSION: FRAME_TAB_STRIP_IPC_V1

Implement tab metadata IPC protocol + tab block rendering. Shell + sexdisplay.

Design complete in FRAME_TAB_STRIP_IPC_PLAN_V1.md.

Changes:

1. crates/sex-pdx/src/lib.rs:
   - Add OP_SURFACE_TAB_INFO = 0xFD

2. servers/sexdisplay/src/main.rs:
   - Add tab_count: u8, active_tab: u8 to Surface struct
   - Update SURFACE_EMPTY initializer
   - Add TAB_STRIP_LIGHT_EXCLUSION_PX = 20 constant
   - Add TAB_ACTIVE_COLOR = FOCUS_SURFACE_COLOR constant
   - Add TAB_INACTIVE_COLOR = 0x006080B0 constant
   - Handle 0xFD opcode in dispatch (store tab info, redraw)
   - Budgeted marker [sexdisplay.tab_info.update] (budget 8)
   - In composite_pixel() Pass 2, top rim band, after lights:
     render colored tab blocks based on tab_count/active_tab

3. servers/silk-shell/src/main.rs:
   - After boot surface 100 0xEC (line ~1692):
     send pdx_call(SLOT_DISPLAY, 0xFD, SURFACE_ID_APP, 1, 0)

Forbidden:
- Text rendering on tabs
- Tab switching behavior
- Dynamic allocation
- Framebuffer path changes
- Broad compositor rewrite
- ABI_VERSION changes (orthogonal)

PASS:
- Default build passes
- Synthetic build passes
- Colored tab block visible in top rim after lights on focused surface
- Frame Lights still work when clicked (red/yellow/green visible)
- Rim drag still works on non-light, non-tab rim
- Tab block disappears if surface has no tabs (tab_count = 0 or default)
- No text rendered on tabs
- [sexdisplay.tab_info.update] fires on boot
- [shell.frame.tab.model] still fires at boot
- No panic/#PF/#GP
- 0xFD with surface_id=0 is silently ignored
- 0xFD for non-existent surface is silently ignored
```
