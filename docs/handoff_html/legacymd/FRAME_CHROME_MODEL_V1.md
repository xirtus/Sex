# FRAME_CHROME_MODEL_V1

## Status

Implemented (2026-05-04). Minimal shell-side Frame/Tab model added to silk-shell. No renderer, protocol, or ABI changes. All existing drag/focus proofs preserved.

---

## Canonical Frame/Tab/Surface Distinction

| Layer | Responsibility | Owner | Mutability |
|-------|---------------|-------|------------|
| **Surface** | Display object (position, size, buffer). Composited by sexdisplay via PDX calls. | sexdisplay / silk-shell (policy) | Position updated by drag, resize. Created/destroyed via 0xEC/0xEE. |
| **Tab** | Shell membership wrapper around a surface_id. Policy metadata (tab title, flags). | silk-shell only | Changed when tab is added/removed/activated within a frame. |
| **Frame** | Tiled container owning one or more tabs. Determines z-order, split layout, chrome rendering policy. | silk-shell only | Changed on split/merge, tab switch, frame move. |

### V1 Model

In V1, the mapping is 1:1:1:
- 1 Frame (`frame_id=1`)
- 1 Tab (`surface_id=100`, `title_id=0`)
- 1 Surface (existing hardcoded `SURFACE_ID_APP = 100`)

All existing surface_id constants remain valid. No surface ID allocation or registry changes. The Frame/Tab model is purely shell-side advisory state — drag, focus, and hit-test continue to target `surface_id` directly.

### Future (V2+)

- **Multi-tab per frame**: `active_tab` selects which surface is visible/interactable. Tab strip chrome renders non-active tabs as clickable headers.
- **Multi-frame layout**: Multiple frames tile the workspace. `active_frame` determines which frame's tab strip has keyboard focus.
- **Chrome hit target**: Hit-test returns `HitTarget::FrameChrome { frame_id, element }` for clicks on tab strip, resize handles, close buttons.

---

## Current Minimal Model

### Structs (silk-shell only, no sexdisplay changes)

```rust
const MAX_TABS_PER_FRAME: u8 = 8;
const MAX_FRAMES: usize = 4;

#[repr(C)]
struct ShellTab {
    surface_id: u64,
    title_id: u64,     // reserved
    flags: u32,        // reserved
}

#[repr(C)]
struct ShellFrame {
    frame_id: u32,
    active_tab: u8,    // index into tabs[]
    tab_count: u8,
    tabs: [Option<ShellTab>; 8],
    flags: u32,        // reserved
}

static mut FRAMES: [Option<ShellFrame>; 4] = [None; 4];
```

### Helpers

```rust
/// Returns the frame_id that owns a tab with the given surface_id.
unsafe fn frame_for_surface(surface_id: u64) -> Option<u32>

/// Returns the active surface_id for the given frame_id.
unsafe fn active_surface_for_frame(frame_id: u32) -> Option<u64>
```

### Init (during boot)

```rust
FRAMES[0] = Some(ShellFrame {
    frame_id: 1,
    active_tab: 0,
    tab_count: 1,
    tabs: [Some(ShellTab { surface_id: SURFACE_ID_APP, .. }), None, ...],
    flags: 0,
});
serial_println!("[shell.frame.model.init] frames=1 tabs=1");
```

### Invariants

1. **No surface ID replacement**: `try_set_focus()`, `point_in_surface()`, `drag_move_focused()`, `click_hit_test_and_focus()`, and all surface-alive queries continue to use hardcoded `SURFACE_ID_*` constants directly. Frame/Tab helpers are query-only; no code path reads `FRAMES` to resolve a surface action.

2. **No heap allocation**: `FRAMES` is a fixed-size `[Option<ShellFrame>; 4]` array — no Vec, no alloc. Each `ShellFrame` contains a fixed `[Option<ShellTab>; 8]`. Total static memory: ~832 bytes.

3. **No renderer dependence**: The model exists entirely in silk-shell policy. `sexdisplay` sees no new opcodes, no new descriptor types, no new surface kinds.

4. **No behavior change for existing paths**: Drag, focus, hit-test, cursor update, snapshot emission — all unchanged. The frame model is purely additive.

5. **Backward-compatible with synthetic proofs**: `SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1` builds and runs unchanged. Drag/focus markers still fire.

---

## What Changed

### servers/silk-shell/src/main.rs

Added (net +120 lines):
- `ShellTab` struct (line ~174)
- `ShellFrame` struct (line ~185)
- `MAX_TABS_PER_FRAME` constant (8)
- `MAX_FRAMES` constant (4)
- `static mut FRAMES: [Option<ShellFrame>; 4]` (line ~198)
- `frame_for_surface()` helper (line ~431)
- `active_surface_for_frame()` helper (line ~447)
- Frame init and `[shell.frame.model.init]` marker during boot (line ~732)

No files other than silk-shell were modified.

---

## What Is Intentionally Deferred

| Feature | When | Why deferred |
|---------|------|-------------|
| Multi-tab per frame | V2 | No protocol for tab creation/switch; no tab strip chrome; no tab bar hit-test. |
| Multi-frame layout | V2 | No split command; no frame registry; no layout engine. |
| Frame chrome rendering | V2 | Requires sexdisplay opcode changes or new surface kinds for chrome elements. |
| `HitTarget::FrameChrome` | V2 | Hit-test currently returns `(u64, bool)` targeting surface_id; promoting to an enum requires broader refactor. |
| Tab title strings | V2 | Requires string allocation or static labels; no protocol for setting titles. |
| Surface ID registry | V3 | Frame model wraps existing IDs; a proper registry with allocate/free/lookup replaces hardcoded constants. |

---

## Build

```bash
# Default
./scripts/entrypoint_build.sh

# Synthetic (optional)
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both pass cleanly. No new warnings from changed files.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-chrome-model-v1.log

for m in \
  shell.frame.model.init \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end \
  shell.cursor_surface.move.ok \
  sexdisplay.cursor.surface.update
do
  printf "%-42s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-chrome-model-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-chrome-model-v1.log
```

Pass criteria:
- `shell.frame.model.init` == 1 (frame model booted)
- All existing drag/focus markers still present (drag > 0, cursor > 0)
- faults == 0

---

## Remaining Risks

- **Frame/Tab not consumed**: In V1, no code path reads `FRAMES`. `frame_for_surface()` and `active_surface_for_frame()` exist but are never called. This is intentional — the model is seeded for future chrome phases without changing current behavior. A future refactor that forgets to wire up the helpers may leave them dead code.
- **FRAMES out of sync with WINDOWS**: The WINDOWS Vec and FRAMES array track overlapping state (window id 2 / surface 100 appears in both). The frame init hardcodes the mapping. Future frame moves, splits, or surface creation/destruction must update both WINDOWS and FRAMES consistently.
- **No active frame state**: There's no `ACTIVE_FRAME_ID` static. In V1 with one frame, this is harmless. Multi-frame layouts need a `static mut ACTIVE_FRAME_ID: u32` to track which frame has keyboard focus.
- **`frame_for_surface()` is O(n*m)**: Linear scan over 4 frames × 8 tabs = 32 entries. Trivial for V1. If frames or tabs per frame grow significantly, a hashmap or direct slot lookup would be needed.
- **`active_surface_for_frame()` indexes without bounds check**: `frame.active_tab as usize` accesses `frame.tabs[]`. If `active_tab >= tab_count`, this reads an `None` entry and returns `None` — safe but silent. A debug assertion would catch corruption earlier.

---

## Next Recommended Phase

**FRAME_CHROME_HIT_TARGET_V1** — Promote hit-test return type from `(u64, bool)` to an enum that distinguishes app surface hits from frame chrome (tab strip, resize handle, close button):

1. Define `enum HitTarget { Surface(u64), Chrome(FrameChromeElement), None }`
2. Update `click_hit_test_and_focus()` to return `HitTarget`
3. Add tab strip region to silkbar model (y = bar_height..bar_height+tab_strip_height)
4. Extend `handle_silkbar_click()` to detect clicks on frame chrome (non-SilkBar top strip region)
5. Defer actual chrome rendering to sexdisplay phase

Alternatively: **SELECTED_WINDOW_SILKBAR_OPTIONS_V1** — Add window management actions to SilkBar (focus indicator per workspace chip, close/minimize buttons for selected window).

Recommended: **FRAME_CHROME_HIT_TARGET_V1** — the hit-target enum is a prerequisite for both chrome interaction and tab switching.
