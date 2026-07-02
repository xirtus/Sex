# SELECTED_WINDOW_OPTIONS_DISPLAY_V1

## Status

Implemented (2026-05-04). Selected-window options (close, zoom, minimize, move) now flow from
silk-shell through silkbar to sexdisplay as visual indicators in the SilkBar top strip.
Display-only — no action behavior in V1.

---

## Protocol Extension

### Data flow

```
silk-shell (PDX 6)             silkbar (PDX ?)              sexdisplay (PDX 4)
    │                              │                              │
    │ OP_SILKBAR_FOCUS_STATE (0xF4)│                              │
    │  arg0 = focus_state (1=shell)│                              │
    │  arg1 = options_mask (NEW)   │                              │
    │  on every try_set_focus()    │                              │
    │ ────────────────────────────→│                              │
    │                              │ msg.arg1 extracted as mask   │
    │                              │ OP_SILKBAR_UPDATE (0xF2)     │
    │                              │  SetSelectedOptions(0,mask)  │
    │                              │ ────────────────────────────→│
    │                              │                              │
    │                              │ (existing clock/chip paths   │
    │                              │  unchanged)                  │
```

### Key design decisions

| Decision | Rationale |
|----------|-----------|
| Reuse `OP_SILKBAR_FOCUS_STATE` (0xF4) | No new PDX opcode needed. arg1 was unused. silkbar already has the handler. |
| Per-focus-change send (not boot-only) | `try_set_focus()` now sends 0xF4 on every focus set/clear. Boot send also carries mask. |
| `UpdateKind::SetSelectedOptions = 6` | New discriminant in existing enum. ABI_VERSION 2→3. `apply_update` stores mask in SilkBar. |
| No action behavior | Options are visual only. Clicking dots does nothing. |

---

## Changes

### 1. crates/silkbar-model/src/lib.rs

| Change | Detail |
|--------|--------|
| ABI_VERSION | 2 → 3 (SILK_DE_BAR_ABI_V1 also bumped) |
| Option bit constants | `OPTION_CLOSE=1, OPTION_ZOOM=2, OPTION_MINIMIZE=4, OPTION_MOVE=8` |
| SilkBar struct | New field `selected_options_mask: u32` |
| UpdateKind | Added `SetSelectedOptions = 6` |
| apply_update | Handler for kind=6: `bar.selected_options_mask = update.a` |
| DEFAULT_SILK_BAR | Includes `selected_options_mask: 0` |
| validate_deterministic_vectors | Added SetSelectedOptions test vector (7 updates, verifies mask=OPTION_MOVE) |

### 2. servers/silk-shell/src/main.rs

| Change | Detail |
|--------|--------|
| Boot-time (line ~1050) | `selected_window_options_mask()` called in `unsafe {}` block, passed as arg1 to 0xF4 |
| `try_set_focus()` clear path | `pdx_call(SLOT_SILKBAR, OP_SILKBAR_FOCUS_STATE, 0, 0, 0)` after log |
| `try_set_focus()` set path | `pdx_call(SLOT_SILKBAR, OP_SILKBAR_FOCUS_STATE, 1, mask, 0)` after options log + budgeted `[shell.selected.options.send]` marker |

### 3. servers/silkbar/src/main.rs

| Change | Detail |
|--------|--------|
| State | Added `last_options_mask: u32 = 0` variable |
| 0xF4 handler | Extracts `msg.arg1 as u32` as options_mask. Compares to `last_options_mask`. Sends `SetSelectedOptions` on change. |
| Markers | `[silkbar.selected.options.recv] mask=0xN`, `[silkbar.selected.options.forward] mask=0xN` |

### 4. servers/sexdisplay/src/main.rs

| Change | Detail |
|--------|--------|
| `handle_silkbar_update()` | Budgeted `[sexdisplay.selected.options.update] mask=0xN` marker on kind=6 |
| `bar_color()` | Early check: if `bar.selected_options_mask != 0`, render 3×3-pixel colored dots at fixed position (x=135, y=19, 5px apart) |

### Visual rule

```
SilkBar top strip (y=10..48):

  [●]  [|||] [|||] [|||] [|||] [|||]  [●][●][●]   🔔  | 10:42
  sel   ws0    ws1   ws2   ws3   ws4    net wifi bat     clock
  opts
  ▲ x=135, y=19
  │ 4 dots: red(close) green(zoom) yellow(minimize) cyan(move)
```

- Dots are 3×3 pixels, 5px apart (x=135, 140, 145, 150)
- Each dot is colored only if the corresponding option bit is set
- Unset bits show panel_fill background
- The entire options row returns panel_fill for non-dot pixels (no bleed into launcher/workspace)

### Color mapping

| Bit | Option | Color | Hex |
|-----|--------|-------|-----|
| 0 | CLOSE | Red | `0x00FF4444` |
| 1 | ZOOM | Green | `0x0044FF44` |
| 2 | MINIMIZE | Yellow | `0x00FFCC44` |
| 3 | MOVE | Cyan | `0x0044CCFF` |

---

## Build

```bash
# Default
./scripts/entrypoint_build.sh

# Synthetic proof (content drag sequence unchanged)
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both pass cleanly. No new warnings.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/selected-window-options-display-v1.log

for m in \
  shell.selected.options \
  shell.selected.options.send \
  silkbar.selected.options.recv \
  silkbar.selected.options.forward \
  sexdisplay.selected.options.update \
  shell.focus.set \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end
do
  printf "%-46s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/selected-window-options-display-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/selected-window-options-display-v1.log
```

### Expected counts (synthetic proof)

| Marker | Expected | Proves |
|--------|----------|--------|
| `shell.selected.options` | ≥1 | Model emits (from prior phase) |
| `shell.selected.options.send` | ≥1 | Shell sent 0xF4 with mask to silkbar |
| `silkbar.selected.options.recv` | ≥1 | Silkbar received the mask |
| `silkbar.selected.options.forward` | ≥1 | Silkbar forwarded SetSelectedOptions to sexdisplay |
| `sexdisplay.selected.options.update` | ≥1 | Sexdisplay stored the mask |
| `shell.focus.set` | ≥1 | Focus changes still work |
| `shell.drag.start/move/end` | ≥1 | Content drag lifecycle intact |
| faults | 0 | Memory safety |

### Pass criteria

- `shell.selected.options.send` > 0
- `silkbar.selected.options.recv` > 0
- `silkbar.selected.options.forward` > 0
- `sexdisplay.selected.options.update` > 0
- `shell.drag.start/move/end` > 0
- faults == 0

---

## Remaining Risks

- **No action behavior**: Option dots are visual only. Clicking does nothing.
- **V1 mask always shows MOVE**: `selected_window_options_mask()` returns `OPTION_MOVE` for any
  frame-owned surface. In V1, frame 1 (surface 100) is the only frame. MOVE is always 0x8.
- **CLOSE/ZOOM/MINIMIZE never set**: Bits 0-2 are always 0 in V1. Their colors (red/green/yellow)
  are defined but never displayed. Code path is verified by inspection.
- **Position fixed at x=135**: The option dot position is hardcoded, not derived from SilkBar
  layout. If the layout shifts, the dots may overlap or misalign.
- **No hover state for dots**: Unlike workspace indicators, option dots do not change color on
  hover. V1 is static indicators only.
- **Old silkbar ignores arg1**: If an old silkbar binary receives the extended 0xF4, it reads
  only arg0 and ignores arg1. Options never appear. No crash.

---

## Next Recommended Phase

### FRAME_LIGHTS_MODEL_V1

This is the last silk-SilkBar-display pipeline phase for selected-window state. Now that
selected-window options are visible, the next step is frame chrome rendering of focus/hover
state:

1. Define a "frame lights" model (small colored indicators for frame state, like a tiny
   LED strip in the frame chrome area)
2. Use the existing silk-shell → sexdisplay paths (0xED focus, 0xEF fill rect) to show
   frame activity/attention state
3. No new ABI needed — existing surface fill rect can render frame lights

Alternatively, if action execution is higher priority:
- **SELECTED_WINDOW_OPTIONS_ACTION_V1**: Add click handling for the option dots. When a dot
  is clicked, silkbar sends a new opcode to silk-shell which executes the action (close/zoom/
  minimize/move). Requires a new opcode from silkbar → silk-shell.
