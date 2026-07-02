# SILKBAR_ABI_EXTENSION_PLAN_V1

Date: 2026-05-15
Status: PLANNING (docs-only, zero source changes)
Scope: silkbar-model ABI, silk-shell send path, silkbar receive path, sexdisplay render path

## 1. PASS/STOP FIRST

| Status | Meaning |
|--------|---------|
| **PASS** | Audit complete. Proposal produced. Zero code changes. |
| **STOP FIRST** | No ABI implementation. No kernel/sex-pdx changes. No sexdisplay framebuffer ownership changes. All work deferred to phased prompts. |

### STOP FIRST Boundaries (unconditionally preserved)

| Boundary | Rationale |
|----------|-----------|
| kernel | No opcode size/alignment changes needed — existing SilkBarUpdate is 16 bytes, fits PDX arg0/arg1/arg2. |
| sex-pdx | No new opcodes needed — `OP_SILKBAR_UPDATE (0xF2)` carries all variants. Slot `SLOT_SILKBAR (7)` unchanged. |
| sexdisplay FB ownership | `render()` and `redraw_top_strip()` own y<51 region. New variants only add color data to bar_color() / clock_fg_at() / bell_badge_at() pattern — no new pixel regions, no DMA path. |
| sexinput / sexusb | Not relevant. |
| Quil / Linen / Spindle / Bell / Collar / Mesh / Atlas app code | Not relevant (shell is the producer, not apps). |

## 2. Files Inspected

| File | Role | What was audited |
|------|------|-----------------|
| `crates/silkbar-model/src/lib.rs` | Model/ABI authority | UpdateKind variants, SilkBar struct, apply_update(), contract validation, ABI_VERSION/SILK_DE_BAR_ABI_V1, queue, UpdateQueue constants |
| `servers/silk-shell/src/main.rs` (~17k lines) | Producer: focus/workspace/status sends | try_set_focus() send path (line ~13596), switch_scene() send path (~7680), maybe_run_silkbar_keyboard_status_proof(), maybe_run_silkbar_palette_status_proof(), OP_SILKBAR_FOCUS_STATE/OP_SILKBAR_WORKSPACE_ACTIVE call sites |
| `servers/silkbar/src/main.rs` (~690 lines) | Producer: SilkBar daemon loop | send_update()/send_update_status() calling OP_SILKBAR_UPDATE, SetClock/SetSelectedOptions/SetBellPresence/workspace/chip init, boot deferred send, contract validation |
| `servers/sexdisplay/src/main.rs` (~2000 lines) | Consumer: receive + render | handle_silkbar_update() (line 1375), OP_SILKBAR_UPDATE handler (~1511), render() (~1006), redraw_top_strip() (~1102), bar_color(), clock_fg_at(), bell_badge_at() |
| `crates/sex-pdx/src/lib.rs` | Opcode/slot registry | OP_SILKBAR_* opcodes (0xF0-0xF4), SLOT_SILKBAR (7), SILKBAR_ABI_VERSION |
| `docs/handoff/SILKBAR_KEYBOARD_STATUS_INTEGRATION_V1.md` | Prior art: keyboard status proof + blockers | Documents active_app_name and tint/accent ABI gaps |
| `docs/handoff/SILKBAR_COMMAND_PALETTE_STATUS_RENDER_V1.md` | Prior art: palette status proof + blockers | Documents palette_state/visible/selected/available ABI gaps |
| `scripts/daily_driver_master_gate.sh` | Gate scanner | silkbar_status gate (marker-based) |
| `scripts/run_daily_driver_proof.sh` | Proof orchestration | SEXOS_SILKBAR_KEYBOARD_STATUS_PROOF=1, SEXOS_SILKBAR_PALETTE_STATUS_PROOF=1 |

## 3. Current ABI Summary

### 3.1 SilkBarUpdate (16-byte PDX wire struct)

```
#[repr(C)]
struct SilkBarUpdate {
    kind: u32,   // UpdateKind discriminant
    index: u8,   // slot index (workspace, chip, theme token)
    a: u32,      // primary value
    b: u32,      // secondary value (clock mm/ss packed)
}
// Compile-time assert: size_of::<SilkBarUpdate>() == 16
```

PDX transport: `pdx_call(SLOT_DISPLAY, OP_SILKBAR_UPDATE, kind as u64, (index<<32)|a, b)`

### 3.2 UpdateKind Variants (current)

```
#[repr(u32)]
enum UpdateKind {
    SetWorkspaceActive = 0,  // index=ws_idx, a=0|1
    SetWorkspaceUrgent = 1,  // index=ws_idx, a=0|1
    SetChipVisible    = 2,  // index=chip_idx, a=0|1
    SetChipKind       = 3,  // index=chip_idx, a=ChipKind as u32
    SetClock          = 4,  // a=hh, b=(mm<<8)|ss
    SetThemeToken     = 5,  // GATE: no-op in V1 (tokens via OP_APPEARANCE_TOKENS)
    SetSelectedOptions= 6,  // a=options bitmask
    SetBellPresence   = 7,  // a=packed(total_visible|redacted<<8|flags<<16)
}
```

### 3.3 SilkBar Data Model (current consumer struct)

```
struct SilkBar {
    layout: [LayoutBox; 11],          // launcher + 5 workspaces + 4 chips + bell
    workspaces: [WorkspaceState; 5],  // index, active, urgent
    chips: [ChipState; 4],           // kind, visible
    clock_hh: u8, clock_mm: u8, clock_ss: u8,
    selected_options_mask: u32,       // OPTION_CLOSE|ZOOM|MINIMIZE|MOVE
    bell_state: BellState,            // total_visible, redacted_count, flags
}
```

### 3.4 Contract Validation (validate_contract)

- `ABI_VERSION == SILK_DE_BAR_ABI_V1` (both 3)
- `LAYOUT_COUNT == SILK_DE_REQUIRED_MODULES` (11)
- `MAX_CHIPS == SILK_DE_REQUIRED_CHIPS` (4)
- `SILKBAR_UPDATE_SIZE == 16`
- `UPDATE_QUEUE_CAP == 32`
- ChipSlot discriminants match array indices
- Rejected at startup if any check fails (contract_err = 1)

### 3.5 Send Path (silk-shell → silkbar → sexdisplay)

```
silk-shell                           silkbar                           sexdisplay
─────────                            ───────                           ──────────
try_set_focus(sid)
  └─ pdx_call(SLOT_SILKBAR,          ┌─ pdx_try_listen_raw(0)          ┌─ pdx_try_listen_raw(0)
      OP_SILKBAR_FOCUS_STATE,        │  OP_SILKBAR_FOCUS_STATE         │  OP_SILKBAR_UPDATE
      1, options_mask, 0)            │   → SetSelectedOptions          │   → apply_update(bar, u)
                                     │   → SetWorkspaceUrgent(ws0..4)  │   → redraw_top_strip()
switch_scene(idx)                    │  OP_SILKBAR_WORKSPACE_ACTIVE    │
  └─ pdx_call(SLOT_SILKBAR,          │   → SetWorkspaceActive(ws0..4)  │
      OP_SILKBAR_WORKSPACE_ACTIVE,   │                                 │
      idx, 0, 0)                     └─ send_update(SetClock) per sec  └─ bar_color() per pixel

silk-shell ALSO emits markers:
  [shell.silkbar.status.send] focus={sid} app={label} tint={idx} bell={count}
  [shell.palette.statusbar] open={0|1} selected={idx} available={count}
```

### 3.6 Current Blockers (documented, not fixed)

| Feature | Existing gap | Fallback |
|---------|-------------|----------|
| Active app name | No UpdateKind variant. Shell emits `app={label}` in marker only. SilkBar has no field for it. | SilkBar displays nothing (no chip, no label) |
| Tint/accent | No UpdateKind variant. Shell emits `tint={idx}` in marker only. SilkBar has no field for it. | SilkBar displays nothing |
| Palette open/close | No UpdateKind variant. Shell emits `[shell.palette.statusbar]` marker only. Focus unchanged. | SilkBar cannot display palette open state |
| Palette selected | No UpdateKind variant. Shell maintains `COMMAND_PALETTE_SELECTED`. | SilkBar cannot display selection |
| Palette available | No UpdateKind variant. Shell computes from `palette_item_status()`. | SilkBar cannot display availability |

## 4. Proposed Additive Variants

### Design Principles

1. **Additive only** — no renumbering of existing variants (0-7 preserved)
2. **Numeric tokens over strings** — active app is identified by a `u32` surface ID (already known by sexdisplay), not a string name
3. **Compact** — reuse existing 16-byte SilkBarUpdate fields, no struct expansion
4. **ABI version bump** — `ABI_VERSION` and `SILK_DE_BAR_ABI_V1` from 3→4
5. **Backward-compatible queue** — old receivers silently reject unknown kinds via `_ => false`

### 4.1 New UpdateKind Variants

```
#[repr(u32)]
enum UpdateKind {
    // existing (0-7 preserved)
    SetWorkspaceActive   = 0,
    SetWorkspaceUrgent   = 1,
    SetChipVisible       = 2,
    SetChipKind          = 3,
    SetClock             = 4,
    SetThemeToken        = 5,   // still gated no-op
    SetSelectedOptions   = 6,
    SetBellPresence      = 7,

    // ── new (Phase 1 additive) ──
    SetActiveApp    = 8,   // a=surface_id, index=0, b=0
    SetTintAccent   = 9,   // a=accent_tint_idx (0-7), index=0, b=0
    SetPaletteState = 10,  // a=packed(open|selected<<1|available<<9), b=0
}
```

### 4.2 Variant Detail: SetActiveApp (kind=8)

| Field | Value | Meaning |
|-------|-------|---------|
| kind | 8 | SetActiveApp discriminant |
| index | 0 (reserved) | Unused |
| a | `surface_id` (u32) | Focused surface ID from silk-shell's `FOCUSED_SURFACE_ID` (e.g., 200=Linen, 201=Quil, 202=Mesh, 204=Bell, 153=Spindle, 0=none) |
| b | 0 | Unused |

**Why surface ID over string name**: sexdisplay already knows surface IDs (part of the compositor). It can map surface_id → label locally. No 32-byte string marshalling needed. ABI stays numeric/compact.

### 4.3 Variant Detail: SetTintAccent (kind=9)

| Field | Value | Meaning |
|-------|-------|---------|
| kind | 9 | SetTintAccent discriminant |
| index | 0 (reserved) | Unused |
| a | `accent_idx` (u32, range 0-7) | Active accent/tint index from silk-shell's `ACTIVE_TINT_IDX` |
| b | 0 | Unused |

**Range**: 0-7 based on Atlas 8-accent palette. sexdisplay renderer can derive RGB from accent index using a pre-defined 8-entry palette table (matching Atlas's accent colors). Alternatively, `a` can directly carry the RGB u32 if preferred — but index-based saves 3 bytes and avoids color drift.

### 4.4 Variant Detail: SetPaletteState (kind=10)

| Field | Value | Meaning |
|-------|-------|---------|
| kind | 10 | SetPaletteState discriminant |
| index | 0 (reserved) | Unused |
| a | packed u32 | bits 0: palette_open (0=closed, 1=open), bits 1-8: palette_selected (0-255), bits 9-16: palette_available (0-255) |
| b | 0 | Unused |

**Why one variant for all three?** Palette open, selected, and available always change atomically (on open/close). Splitting into three variants wastes discriminants and causes triple PDX calls for one toggle. Packing keeps ABI small.

### 4.5 SilkBar Model Fields (additive)

```
struct SilkBar {
    // existing fields preserved (order, size, alignment)
    layout: [LayoutBox; 11],
    workspaces: [WorkspaceState; 5],
    chips: [ChipState; 4],
    clock_hh: u8, clock_mm: u8, clock_ss: u8,
    selected_options_mask: u32,
    bell_state: BellState,

    // ── new (appended) ──
    active_app_sid: u32,         // 0 = no active app
    accent_tint_idx: u8,        // 0-7
    palette_open: bool,          // true = command palette visible
    palette_selected: u8,        // 0-255 selected index
    palette_available: u8,       // 0-255 available item count
    _pad: [u8; 1],              // keep struct 4-byte aligned
}
```

### 4.6 apply_update() Extensions

```
match update.kind {
    // ... 0-7 unchanged ...
    8 => {
        // SetActiveApp: a = surface_id
        bar.active_app_sid = update.a;
        true
    }
    9 => {
        // SetTintAccent: a = accent_idx (0-7)
        if update.a > 7 { return false; }
        bar.accent_tint_idx = update.a as u8;
        true
    }
    10 => {
        // SetPaletteState: a = packed(open|selected<<1|available<<9)
        bar.palette_open     = (update.a & 1) != 0;
        bar.palette_selected = ((update.a >> 1) & 0xFF) as u8;
        bar.palette_available= ((update.a >> 9) & 0xFF) as u8;
        true
    }
    _ => false,
}
```

### 4.7 validate_deterministic_vectors() Extension

Add vectors covering each new variant:

```
SilkBarUpdate::new(UpdateKind::SetActiveApp as u32, 0, SURFACE_ID_LINEN, 0),
SilkBarUpdate::new(UpdateKind::SetTintAccent as u32, 0, 3, 0),
SilkBarUpdate::new(UpdateKind::SetPaletteState as u32, 0,
    1 | (2 << 1) | (7 << 9), 0),  // open=1, selected=2, available=7
```

Verify post-apply: `bar.active_app_sid == 200`, `bar.accent_tint_idx == 3`, `bar.palette_open == true`, `bar.palette_selected == 2`, `bar.palette_available == 7`.

### 4.8 New ABI Version Constants

```
pub const ABI_VERSION: u32 = 4;
pub const SILK_DE_BAR_ABI_V1: u32 = 4;
pub const SILKBAR_ABI_VERSION: u64 = 3;  // PDX-facing version bump (was 2)
```

## 5. Compatibility Plan

### 5.1 Current SILK_DE_BAR_ABI_V1 (3) Behavior

- SilkBar model v3: UpdateKind variants 0-7 only
- `apply_update()` returns `false` for `kind >= 8`
- Contract validation: ABI_VERSION=3, SILKBAR_UPDATE_SIZE=16, LAYOUT_COUNT=11
- Queue capacity: 32

### 5.2 ABI v4 Change

| What changes | How |
|-------------|-----|
| `ABI_VERSION` | 3→4 |
| `SILK_DE_BAR_ABI_V1` | 3→4 |
| `SILKBAR_ABI_VERSION` | 2→3 (PDX-facing) |
| `UpdateKind` enum | Three new discriminants (8, 9, 10) |
| `SilkBar` struct | Five new fields appended |
| `apply_update()` | Three new match arms |
| `validate_contract()` | Checks new ABI_VERSION=4 |
| `validate_deterministic_vectors()` | Three new vector entries |
| `DEFAULT_SILK_BAR` | New fields initialized to zero |

**What does NOT change**: SilkBarUpdate struct size remains 16 bytes. Queue capacity remains 32. Opcodes unchanged. Slot unchanged. LayoutBox geometry unchanged.

### 5.3 Old Receiver Behavior (backward compatibility)

If an old sexdisplay (compiled against ABI v3) receives updates with `kind=8/9/10`:

- `apply_update()` hits `_ => false`
- Update silently dropped (existing behavior for unknown kinds)
- No crash, no panic, no undefined state
- Old renderer draws SilkBar without app name/tint/palette (status quo)
- Contract check: old sexdisplay has `ABI_VERSION=3, SILK_DE_BAR_ABI_V1=3` → passes its own self-check

If a new producer sends to old consumer: graceful degradation. The old consumer skips unknown updates and renders the bar as before. The new consumer (v4) renders the full bar with app/tint/palette.

### 5.4 Migration Path

1. **Phase 1**: Bump model constants, add variants + fields, update contract validation. Old consumers reject new updates harmlessly.
2. **Phase 2**: New silk-shell sends new SetActiveApp/SetTintAccent/SetPaletteState updates alongside existing ones. Old consumers ignore them.
3. **Phase 3**: New sexdisplay receives and renders new fields. Full feature visible.
4. **Phase 4**: Gate updated to require new markers.

No flag-day: phases can be deployed incrementally. Old consumer + new producer = no regression. New producer + new consumer = full feature.

## 6. Phased Implementation Prompts

Each phase is a standalone implementation unit. Phases must be executed in order. Each phase produces its own handoff doc.

---

### Phase 1: silkbar-model Only (no sends, no renders)

```
MISSION: SILKBAR_ABI_PHASE1_MODEL_V1

Goal: Add SetActiveApp/SetTintAccent/SetPaletteState variants to silkbar-model
with zero producers and zero consumers. This is a pure model/ABI layer change.

Tasks:
1. Add UpdateKind variants 8/9/10 in crates/silkbar-model/src/lib.rs
2. Append active_app_sid, accent_tint_idx, palette_open, palette_selected,
   palette_available fields to SilkBar struct (with _pad for alignment)
3. Add apply_update() match arms for kinds 8/9/10
4. Bump ABI_VERSION=4, SILK_DE_BAR_ABI_V1=4, SILKBAR_ABI_VERSION=3
5. Add deterministic vectors for new variants in validate_deterministic_vectors()
6. Update DEFAULT_SILK_BAR with zeroed new fields
7. Full build with SEXOS_SILKBAR_KEYBOARD_STATUS_PROOF=1 and baseline

STOP FIRST:
- Do NOT modify silk-shell (no send calls)
- Do NOT modify silkbar server (no send calls)
- Do NOT modify sexdisplay (no receive/render)
- Do NOT modify sex-pdx opcodes or slots
- Do NOT modify contract validation logic except ABI_VERSION check

Validation:
- cargo check --workspace
- ./scripts/entrypoint_build.sh → PASS (old producers compose fine, new variants are dead code)
- Contract validation should still PASS (ABI_VERSION==SILK_DE_BAR_ABI_V1==4)
- validate_deterministic_vectors() PASS with new vectors

Handoff: docs/handoff/SILKBAR_ABI_PHASE1_MODEL_V1.md
```

---

### Phase 2: silk-shell Send Markers (producer side)

```
MISSION: SILKBAR_ABI_PHASE2_SHELL_SENDS_V1

Goal: silk-shell emits SetActiveApp, SetTintAccent, and SetPaletteState updates
via OP_SILKBAR_UPDATE (direct to sexdisplay, bypassing silkbar daemon for these
new variants). Preserve existing OP_SILKBAR_FOCUS_STATE and
OP_SILKBAR_WORKSPACE_ACTIVE paths to silkbar daemon.

Tasks:
1. In try_set_focus(), after existing pdx_call(SLOT_SILKBAR, ...):
   a. Send SetActiveApp to sexdisplay with surface_id in 'a' field
   b. Send SetTintAccent with ACTIVE_TINT_IDX in 'a' field
2. In toggle_command_palette():
   a. On open: send SetPaletteState(a = 1 | (COMMAND_PALETTE_SELECTED<<1) | (available<<9))
   b. On close: send SetPaletteState(a = 0)
3. In switch_scene(), send SetActiveApp and SetTintAccent (same as focus)
4. Gate all new sends behind compile-time feature (SEXOS_SILKBAR_ABI_EXTENSION=1)
   so baseline builds are unchanged
5. Add [shell.silkbar.send.new] markers for each new variant sent

STOP FIRST:
- Do NOT modify silkbar daemon (old path preserved for backward compat)
- Do NOT modify sexdisplay receive/render
- Do NOT modify silkbar-model (already done in Phase 1)
- Do NOT change OP_SILKBAR_FOCUS_STATE or OP_SILKBAR_WORKSPACE_ACTIVE semantics

Validation:
- SEXOS_SILKBAR_ABI_EXTENSION=1 ./scripts/entrypoint_build.sh → PASS
- ./scripts/entrypoint_build.sh → PASS (baseline zero-change)
- Boot: sexdisplay receives new updates → silently drops them (backward compat)
- [shell.silkbar.send.new] markers appear for focus, scene, palette toggle

Handoff: docs/handoff/SILKBAR_ABI_PHASE2_SHELL_SENDS_V1.md
```

---

### Phase 3: sexdisplay Receive + Render (consumer side)

```
MISSION: SILKBAR_ABI_PHASE3_DISPLAY_RENDER_V1

Goal: sexdisplay receives SetActiveApp/SetTintAccent/SetPaletteState updates,
mutates local SilkBar model, and renders new visual elements on the top strip.

Tasks:
1. handle_silkbar_update() already passes all OP_SILKBAR_UPDATE messages
   through apply_update() — new variants auto-populate bar fields. No change
   needed for receive path.
2. Add render helpers in sexdisplay:
   a. app_label_at(x, y, bar) — renders active app name text at panel left
      (uses surface_id → label mapping; 8 pre-defined labels for known apps,
      "App" for unknown)
   b. tint_indicator_at(x, y, bar) — renders small accent color swatch
      (e.g., 6x6 dot near chip area, color from accent_idx palette)
   c. palette_indicator_at(x, y, bar) — renders palette open indicator
      (e.g., highlight on launcher icon when palette_open==true)
3. Integrate helpers into bar_color() / clock_fg_at() / bell_badge_at()
   pixel dispatch (same layered approach as existing: check clock_fg_at,
   then bell_badge_at, then bar_color)
4. Add budgeted markers: [sexdisplay.render.app_label], [sexdisplay.render.tint],
   [sexdisplay.render.palette_indicator]
5. Gate new render behind compile-time feature (SEXOS_SILKBAR_ABI_EXTENSION=1)

STOP FIRST:
- Do NOT change framebuffer ownership (y<51 remains SilkBar territory)
- Do NOT modify silkbar daemon
- Do NOT add new PDX opcodes
- Do NOT change SilkBarUpdate struct size or alignment
- Do NOT touch kernel or sex-pdx

Validation:
- SEXOS_SILKBAR_ABI_EXTENSION=1 ./scripts/entrypoint_build.sh → PASS
- Boot: active app name appears on bar after focus
- Boot: tint swatch color changes after Atlas accent apply
- Boot: palette indicator appears/hides on Cmd+P toggle
- [sexdisplay.render.app_label], [sexdisplay.render.tint],
  [sexdisplay.render.palette_indicator] markers fire
- Zero faults

Handoff: docs/handoff/SILKBAR_ABI_PHASE3_DISPLAY_RENDER_V1.md
```

---

### Phase 4: Proof Profile Gate Update

```
MISSION: SILKBAR_ABI_PHASE4_PROOF_GATE_V1

Goal: Update daily-driver proof profile and master gate to require new SilkBar
ABI extension markers. Existing gates continue to pass with extension disabled.

Tasks:
1. Add SEXOS_SILKBAR_ABI_EXTENSION=1 to run_daily_driver_proof.sh env block
2. In daily_driver_master_gate.sh, add silkbar_abi_extension gate:
   - PASS: [sexdisplay.render.app_label] found
   - PASS: [sexdisplay.render.tint] found
   - PASS: [sexdisplay.render.palette_indicator] found
   - SKIP: SEXOS_SILKBAR_ABI_EXTENSION not enabled (maintains backward compat)
3. Update SILKBAR_KEYBOARD_STATUS_INTEGRATION_V1.md blockers section:
   - Mark active_app_name as RESOLVED (ABI v4 provides SetActiveApp)
   - Mark tint_accent as RESOLVED (ABI v4 provides SetTintAccent)
4. Update SILKBAR_COMMAND_PALETTE_STATUS_RENDER_V1.md blockers section:
   - Mark palette_state/visible/selected/available as RESOLVED
     (ABI v4 provides SetPaletteState)
5. Update spindle daily summary to reflect resolved blockers

STOP FIRST:
- Do NOT change gate logic for existing gates (keyboard_gui, command_palette, etc.)
- Do NOT require new markers when SEXOS_SILKBAR_ABI_EXTENSION is unset
- Do NOT modify kernel, sex-pdx, or app code

Validation:
- SEXOS_SILKBAR_ABI_EXTENSION=1 ./scripts/run_daily_driver_proof.sh → PASS
- ./scripts/run_daily_driver_proof.sh → PASS (baseline, without extension)
- daily_driver_master_gate.sh: silkbar_abi_extension gate PASS (when enabled),
  SKIP (when not enabled)
- Zero faults in both profiles

Handoff: docs/handoff/SILKBAR_ABI_PHASE4_PROOF_GATE_V1.md
```

## 7. Handoff Path

```
docs/handoff/SILKBAR_ABI_EXTENSION_PLAN_V1.md          ← THIS DOCUMENT
docs/handoff/SILKBAR_ABI_PHASE1_MODEL_V1.md              ← future (Phase 1 output)
docs/handoff/SILKBAR_ABI_PHASE2_SHELL_SENDS_V1.md        ← future (Phase 2 output)
docs/handoff/SILKBAR_ABI_PHASE3_DISPLAY_RENDER_V1.md     ← future (Phase 3 output)
docs/handoff/SILKBAR_ABI_PHASE4_PROOF_GATE_V1.md          ← future (Phase 4 output)
```

## Appendix A: Current Opcode Space (reference)

```
0xF0 OP_SILKBAR_PING            — ping → 0
0xF1 OP_SILKBAR_GET_ABI         — get ABI version
0xF2 OP_SILKBAR_UPDATE          — push SilkBarUpdate (arg0=kind, arg1=index|a, arg2=b)
0xF3 OP_SILKBAR_WORKSPACE_ACTIVE— shell→silkbar: scene switch
0xF4 OP_SILKBAR_FOCUS_STATE     — shell→silkbar: focus changed + options mask
```

New opcodes are NOT needed. `OP_SILKBAR_UPDATE (0xF2)` already carries all variant kinds via the `kind` field inside SilkBarUpdate. This is the intended design: new variants are data-plane additions, not control-plane additions.

## Appendix B: Glyph/Visual Suggestions (render decisions deferred to Phase 3)

| Visual Element | Location on bar (y<50 region) | Rendering approach |
|---------------|-------------------------------|-------------------|
| Active app label | Near launcher, chip area left side | `font::draw_str()` with 8-char truncated label |
| Tint accent swatch | Between chip area and clock | Small 6x6 filled rect in accent color |
| Palette open indicator | Launcher icon border highlight | Brighten launcher fill when `palette_open==true` |

Accent index → RGB lookup table (matching Atlas 8-accent palette):

```
const ACCENT_PALETTE: [u32; 8] = [
    0x0089B4FA,  // 0: Blue
    0x00A6E3A1,  // 1: Green
    0x00F9E2AF,  // 2: Yellow
    0x00FAB387,  // 3: Peach
    0x00F38BA8,  // 4: Red
    0x00CBA6F7,  // 5: Mauve
    0x00F5C2E7,  // 6: Pink
    0x0094E2D5,  // 7: Teal
];
```

## Appendix C: Surface ID → Label Mapping (for Phase 3 renderer)

```
const SURFACE_LABELS: &[(u64, &str)] = &[
    (200, "Linen"),
    (201, "Quil"),
    (202, "Mesh"),
    (203, "Collar"),
    (204, "Bell"),
    (153, "Spindle"),
    (100, "App"),
    (101, "Static"),
    (102, "Test3"),
    (103, "Test4"),
];
// Unknown surface IDs → "App"
```

Note: Phase 1 model stores only the `u32` surface ID. Phase 3 renderer maps it locally. This keeps the ABI compact.

## Appendix D: Why silkbar daemon is NOT modified

The existing silkbar daemon (`servers/silkbar/src/main.rs`) is a round-trip relay:
- Receives OP_SILKBAR_FOCUS_STATE from shell → forwards SetSelectedOptions + SetWorkspaceUrgent to sexdisplay
- Receives OP_SILKBAR_WORKSPACE_ACTIVE from shell → forwards SetWorkspaceActive to sexdisplay
- Maintains its own clock cadence → forwards SetClock to sexdisplay
- Polls Bell → forwards SetBellPresence to sexdisplay
- Bounded chip status stub → forwards SetChipKind to sexdisplay

For the new variants (SetActiveApp, SetTintAccent, SetPaletteState), the silk-shell can send directly to sexdisplay via `OP_SILKBAR_UPDATE`. No need for the silkbar relay daemon to intermediate these messages, because:
1. They are stateless from silkbar's perspective (no polling, no cadence)
2. They change on every focus/scene/palette event — no value in cache-and-forward
3. Direct send reduces PDX hop count by 1 (shell→display instead of shell→silkbar→display)
4. The silkbar daemon has zero knowledge of surface IDs, accent indices, or palette state

This is consistent with the existing architecture: `OP_SILKBAR_UPDATE` is addressed to `SLOT_DISPLAY` (sexdisplay), not `SLOT_SILKBAR`. Silkbar daemon is just one producer among many for the Update queue in sexdisplay.

If desired later, silkbar daemon could be extended as a proxy for these variants too — but Phase 2 uses direct send as the simplest path.
