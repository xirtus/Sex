# SELECTED_WINDOW_OPTIONS_DISPLAY_PLAN_V1

## Status

Design (2026-05-04). No code changed. The existing `OP_SILKBAR_FOCUS_STATE` (0xF4) from
silk-shell → silkbar, combined with a new `UpdateKind::SetSelectedOptions` variant in the
silkbar-model crate, can carry selected-window options to sexdisplay **without kernel ABI
changes, without new PDX opcodes.**

**Verdict: SAFE_IMPLEMENTATION_WITHOUT_KERNEL_ABI**

---

## Current Data Path

```
silk-shell (PDX 6)             silkbar (PDX ?)              sexdisplay (PDX 4)
    │                              │                              │
    │ OP_SILKBAR_WORKSPACE_ACTIVE  │                              │
    │  (0xF3, arg0=ws_idx)         │                              │
    │ boot: workspace=0            │                              │
    │ on click: workspace=N        │                              │
    │ ────────────────────────────→│                              │
    │                              │ OP_SILKBAR_UPDATE (0xF2)     │
    │                              │  SetWorkspaceActive(i,0/1)   │
    │                              │ ────────────────────────────→│
    │                              │                              │
    │ OP_SILKBAR_FOCUS_STATE       │                              │
    │  (0xF4, arg0=state)          │                              │
    │  ONLY ONCE AT BOOT           │                              │
    │  arg0=1 ("shell" focus)      │                              │
    │  arg1=0 (unused)             │                              │
    │  arg2=0 (unused)             │                              │
    │ ────────────────────────────→│                              │
    │                              │ focus_state = arg0           │
    │                              │ maps to urgent_ws:           │
    │                              │   1→ws0, 2→ws1, 3→ws2       │
    │                              │ OP_SILKBAR_UPDATE            │
    │                              │  SetWorkspaceUrgent(i,0/1)   │
    │                              │ ────────────────────────────→│
    │                              │                              │
    │                              │ (every 1s) SetClock          │
    │                              │ (every 120s) SetChipKind     │
    │                              │ ────────────────────────────→│
```

### Key gaps

| Gap | Detail |
|-----|--------|
| `OP_SILKBAR_FOCUS_STATE` is boot-only | silk-shell sends it once at line 1050, never again. silkbar never receives live focus changes. |
| arg1 is unused | Three PDX args available (arg0/arg1/arg2). Only arg0 is consumed. |
| No selected-options update kind | `UpdateKind` has 5 variants (0-5). No variant for options mask. |
| ChipKind has no option semantics | `{Net,Wifi,Battery,Clock}` are status indicators only. |

---

## Proposed Extension

### Principle: reuse, don't add

- **No new PDX opcode**: extend `OP_SILKBAR_FOCUS_STATE` usage from boot-only to live
- **No new ChipKind variants**: options are orthogonal to status chips
- **New UpdateKind**: `SetSelectedOptions = 6` carries the options mask to sexdisplay
- **New rendering**: options shown as small colored indicators in the top strip

### Silk-shell changes

**Current** (line 1050, boot only):
```rust
pdx_call(SLOT_SILKBAR, OP_SILKBAR_FOCUS_STATE, 1, 0, 0);
```

**Proposed** — send on every `try_set_focus()`:
```rust
// Within try_set_focus(), after successful set (sid != 0):
let mask = selected_window_options_mask();
pdx_call(SLOT_SILKBAR, OP_SILKBAR_FOCUS_STATE, 1, mask as u64, 0);

// On clear (sid == 0):
pdx_call(SLOT_SILKBAR, OP_SILKBAR_FOCUS_STATE, 0, 0, 0);
```

- `arg0` = 1 (shell-focus state, unchanged semantics)
- `arg1` = `selected_window_options_mask()` (new: options bitmask)
- Boot-only send is replaced by per-focus-change send

Remove the boot-only `OP_SILKBAR_FOCUS_STATE` send at line 1050 (redundant — first
`try_set_focus()` happens during boot init when `FOCUSED_SURFACE_ID = SURFACE_ID_APP`
is set, but `try_set_focus` is NOT called during boot for the initial focus; the
`FOCUSED_SURFACE_ID = SURFACE_ID_APP` is a static assignment). Need a boot-time call
or send the initial options after boot init completes.

### Silkbar-model changes

#### 1. Add `UpdateKind::SetSelectedOptions = 6`

```rust
#[repr(u32)]
pub enum UpdateKind {
    SetWorkspaceActive = 0,
    SetWorkspaceUrgent = 1,
    SetChipVisible = 2,
    SetChipKind = 3,
    SetClock = 4,
    SetThemeToken = 5,
    /// Carries a selected-window options mask to sexdisplay.
    /// index=0 reserved, a=options_mask, b=0.
    SetSelectedOptions = 6,
}
```

#### 2. Add `apply_update` handler for kind=6

```rust
6 => {
    // SetSelectedOptions: a = options mask (bitfield)
    // No bar state change needed — mask is extracted by handle_silkbar_update
    // and stored for rendering. Return true to acknowledge.
    bar.selected_options_mask = update.a;
    true
}
```

Wait — the `SilkBar` struct doesn't have a `selected_options_mask` field. This means we need to add one. Let me check...

Actually, `SilkBar` currently has:
```rust
pub struct SilkBar {
    pub layout: [LayoutBox; LAYOUT_COUNT],
    pub workspaces: [WorkspaceState; WORKSPACE_COUNT],
    pub chips: [ChipState; MAX_CHIPS],
    pub clock_hh: u8,
    pub clock_mm: u8,
    pub clock_ss: u8,
}
```

Adding a `selected_options_mask: u32` field is a struct change. This changes the struct layout,
which is shared across the PDX boundary as a model (not wire format). The `SilkBar` is NOT
sent over PDX — it's reconstructed on each side from `DEFAULT_SILK_BAR` and updated via
`SilkBarUpdate` messages. So adding a field to `SilkBar` doesn't change the wire ABI.

But it does require:
- Updating `SURFACE_EMPTY`-style default initializers if any
- Bumping `ABI_VERSION` if the contract validation checks struct size

Actually, `SilkBar` is NOT part of the PDX ABI — only `SilkBarUpdate` (16 bytes) is. The
`SilkBar` struct is a local model reconstructed in silkbar and sexdisplay independently.
Adding a field is safe.

**Correction: We don't need to add the mask to `SilkBar` struct.** Instead, sexdisplay can
store the mask in a separate static variable alongside the bar. The `apply_update` handler
already returns a bool — the caller (sexdisplay's `handle_silkbar_update`) can read the
kind and extract the mask from the unpacked update struct.

But the cleanest approach is to add the field. Let me keep it in the struct.

#### 3. Bump `ABI_VERSION`

Current `ABI_VERSION = 2`. Bump to `3` to signal the new `UpdateKind` variant. The
`validate_contract()` gate ensures producer and consumer are in sync.

### Silkbar server changes

In the `OP_SILKBAR_FOCUS_STATE` handler, add options mask extraction:

```rust
} else if msg.type_id == sex_pdx::OP_SILKBAR_FOCUS_STATE {
    focus_state = (msg.arg0 as u8).min(3);
    // Extract selected-window options mask (new in V1).
    // arg1 is backward compatible: old senders pass 0 (no options).
    let options_mask = msg.arg1 as u32;
    if options_mask != last_options_mask {
        send_update(SilkBarUpdate::new(
            UpdateKind::SetSelectedOptions as u32, 0, options_mask, 0,
        ));
        last_options_mask = options_mask;
    }
}
```

The `last_options_mask` variable is initialized to `0` and tracks changes to avoid sending
duplicate updates.

### Sexdisplay changes

#### 1. Handle `SetSelectedOptions` in `apply_update`

Already covered by the silkbar-model change above. Sexdisplay calls `apply_update()` which
will handle kind=6.

#### 2. Store options mask for rendering

The mask is stored in `bar.selected_options_mask` (new field). Sexdisplay's `bar_color()`
function reads it during top-strip rendering.

#### 3. Render option indicators

**V1 rendering approach:** simple colored dots/rects in the top strip, positioned near the
existing chips but visually distinct. Exact pixel position TBD during implementation, but
principles:

```
┌─ SilkBar (top strip, y=10..48) ──────────────────────────────────────┐
│  ●  │ [ws0][ws1][ws2][ws3][ws4] │ [●][●][●]  🔔 │ 10:42    │
│  ⬡  │                          │ net wifi bat │          │
│ sel  │  workspaces              │  status chips │  clock   │
└──────┴──────────────────────────┴────────────────┴──────────┘
```

The options indicator would be a small area to the left of the workspace indicators
(or right of the launcher). For V1, a single colored dot per set option bit:

| Bit | Indicator | Color |
|-----|-----------|-------|
| OPTION_CLOSE (1) | Red dot | `0x00FF4444` |
| OPTION_ZOOM (2) | Green dot | `0x0044FF44` |
| OPTION_MINIMIZE (4) | Yellow dot | `0x00FFCC44` |
| OPTION_MOVE (8) | Cyan dot | `0x0044CCFF` |

When mask=0 (no selection), no indicators are drawn.

#### 4. Bounds safety

Option indicators are rendered within the top strip (y < PANEL_Y + PANEL_H + PANEL_GLOW).
The position constants ensure all pixels stay within [PANEL_X, PANEL_X+PANEL_W) x
[PANEL_Y, PANEL_Y+PANEL_H). No framebuffer bounds changes needed.

---

## Files Allowed for Implementation

| File | Changes |
|------|---------|
| `crates/silkbar-model/src/lib.rs` | Add `UpdateKind::SetSelectedOptions = 6`, add `selected_options_mask: u32` to `SilkBar`, bump ABI_VERSION, add `apply_update` handler, update `DEFAULT_SILK_BAR` |
| `servers/silk-shell/src/main.rs` | Send `OP_SILKBAR_FOCUS_STATE` from `try_set_focus()` with arg1=mask, add boot-time initial options send |
| `servers/silkbar/src/main.rs` | Extract `options_mask` from arg1, track `last_options_mask`, send `SetSelectedOptions` on change |
| `servers/sexdisplay/src/main.rs` | Handle kind=6 rendering in `bar_color()` or dedicated function |

## Files Forbidden

- `kernel/` — no kernel changes
- `crates/sex-pdx/src/lib.rs` — no new PDX opcode constants
- `servers/sexusb/`, `servers/sexinput/` — no input changes
- Any framebuffer bounds or renderer ownership changes

---

## Compatibility Strategy

| Concern | Mitigation |
|---------|-----------|
| Old silkbar ignores arg1 | `arg1` is new. Old silkbar reads only `msg.arg0` and ignores `arg1`. No crash. Options simply never appear. |
| Old sexdisplay ignores kind=6 | `apply_update` returns `false` for unknown kinds. `handle_silkbar_update` returns `(false, 6)`. No crash. |
| ABI_VERSION mismatch | Contract validation at startup in both silkbar and sexdisplay catches drift. Startup fails with reason code. |
| silk-shell sends 0xF4 more frequently | Currently once at boot. Now on every `try_set_focus()`. silkbar already handles 0xF4 — no new listen dispatch needed. |

---

## Build and Proof Plan

```bash
./scripts/entrypoint_build.sh
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both must pass with no new warnings.

### Verification markers

| Marker | Source | Expected |
|--------|--------|----------|
| `[shell.focus.set]` | silk-shell | ≥1 (existing) |
| `[shell.selected.options]` | silk-shell | ≥1 (existing from SILKBAR_OPTIONS_V1) |
| `[silkbar.selected.options.send]` | silkbar (new) | ≥1 (new — confirms forward to sexdisplay) |
| `[sexdisplay.selected.options.render]` | sexdisplay (new, budgeted) | ≥1 (new — confirms render path) |
| faults | kernel | 0 |

---

## Action Execution Deferral

**No action behavior is implemented in this phase.** The pipeline is:

```
silk-shell model → silkbar forward → sexdisplay render
                                        ↓
                                  (no action)
```

Future phases (e.g., `SELECTED_WINDOW_OPTIONS_ACTION_V1`) would:
1. Make silkbar click on option chips actionable (currently `Action::None`)
2. Send close/zoom/minimize commands from silkbar to silk-shell
3. silk-shell executes the action on the selected window

For V1, options are visual indicators only. Clicking on them does nothing.

---

## STOP Conditions

This implementation is SAFE and requires NO kernel/PDX ABI changes if the following hold:

| Condition | Risk level |
|-----------|------------|
| Adding field to `SilkBar` struct | ✅ Safe — not wire format, reconstructed locally |
| Adding `UpdateKind` variant | ✅ Safe — enum change, ABI_VERSION gate catches mismatch |
| Sending 0xF4 more frequently | ✅ Safe — silkbar already handles it, no listen dispatch change |
| Adding rendering to `bar_color()` | ✅ Safe — within existing bounds-checked top-strip path |

**STOP FIRST if:**
- `SilkBar` layout or size needs to change for the PDX wire format (it doesn't — only `SilkBarUpdate` is wire format)
- `OP_SILKBAR_FOCUS_STATE` needs a new handler path in silkbar (it doesn't — existing match arm is reused)
- sexdisplay needs to add new module layout slots (it shouldn't — option indicators can be positioned independently)
- Action behavior would be required (it isn't — display only in V1)

---

## Next Implementation Prompt

The next phase is **SELECTED_WINDOW_OPTIONS_DISPLAY_V1** — implement the full path:

```
MISSION: SELECTED_WINDOW_OPTIONS_DISPLAY_V1.

IMPLEMENTATION ONLY. Design complete in SELECTED_WINDOW_OPTIONS_DISPLAY_PLAN_V1.md.

Files to modify (in order):
1. crates/silkbar-model/src/lib.rs — UpdateKind, SilkBar field, apply_update, ABI_VERSION
2. servers/silk-shell/src/main.rs — try_set_focus sends 0xF4 with mask, boot init
3. servers/silkbar/src/main.rs — extract mask from arg1, forward to sexdisplay
4. servers/sexdisplay/src/main.rs — render option indicators in top strip

Constraints:
- No kernel/PDX ABI changes
- No new PDX opcodes
- No action behavior
- Preserve existing clock/chip/workspace rendering
- Bounds-check all new pixel positions
- Budgeted diagnostic markers for forward/render paths
