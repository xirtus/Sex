# SILKBAR_ABI_PHASE1_MODEL_V1

Date: 2026-05-15
Status: PASS
Scope: crates/silkbar-model/src/lib.rs only — model/ABI layer, zero producer/consumer changes

## 1. PASS/STOP FIRST

| Status | Meaning |
|--------|---------|
| **PASS** | Phase 1 model changes compile. 16/16 daily-driver gates PASS. Zero faults. |
| **STOP FIRST** | No kernel/sex-pdx edits. No silk-shell/silkbar/sexdisplay edits. No opcode renumbering. Additive only. Existing UpdateKind 0-7 preserved exactly. |

## 2. Attempts Used

| Attempt | Result | Notes |
|---------|--------|-------|
| 1 | PASS | All edits applied correctly first try. Build + proof profile clean. |

## 3. ABI Delta Table

### 3.1 Version Constants

| Constant | Old | New | Purpose |
|----------|-----|-----|---------|
| `ABI_VERSION` | 3 | **4** | Model layout version (bump on struct/enum change) |
| `SILK_DE_BAR_ABI_V1` | 3 | **4** | Must equal ABI_VERSION (contract validation) |
| `SILKBAR_ABI_VERSION` | 2 | **3** | PDX-facing version (consumers query via OP_SILKBAR_GET_ABI) |

### 3.2 UpdateKind Enum (new discriminants)

| Discriminant | Variant | Wire Fields | Status |
|-------------|---------|------------|--------|
| 0 | SetWorkspaceActive | index=ws_idx, a=0\|1 | **preserved** |
| 1 | SetWorkspaceUrgent | index=ws_idx, a=0\|1 | **preserved** |
| 2 | SetChipVisible | index=chip_idx, a=0\|1 | **preserved** |
| 3 | SetChipKind | index=chip_idx, a=ChipKind | **preserved** |
| 4 | SetClock | a=hh, b=(mm<<8)\|ss | **preserved** |
| 5 | SetThemeToken | (gated no-op in V1) | **preserved** |
| 6 | SetSelectedOptions | a=options bitmask | **preserved** |
| 7 | SetBellPresence | a=packed(total\|redacted<<8\|flags<<16) | **preserved** |
| **8** | **SetActiveApp** | **a=surface_id**, index=0, b=0 | **NEW** |
| **9** | **SetTintAccent** | **a=accent_idx (0-7)**, index=0, b=0 | **NEW** |
| **10** | **SetPaletteState** | **a=packed(open\|selected<<1\|available<<9)**, b=0 | **NEW** |

### 3.3 SilkBar Struct (new fields appended)

| Field | Type | Default | Populated By |
|-------|------|---------|-------------|
| `layout` | `[LayoutBox; 11]` | (geometry) | **preserved** |
| `workspaces` | `[WorkspaceState; 5]` | default states | **preserved** |
| `chips` | `[ChipState; 4]` | default chips | **preserved** |
| `clock_hh/mm/ss` | `u8`×3 | 10:42:00 | **preserved** |
| `selected_options_mask` | `u32` | 0 | **preserved** |
| `bell_state` | `BellState` | zeros | **preserved** |
| **`phase1`** | **`SilkBarPhase1Ext`** | **zeros** | **NEW** |
| ├─ `active_app_sid` | `u32` | 0 | SetActiveApp(8) |
| ├─ `accent_tint_idx` | `u8` | 0 | SetTintAccent(9) |
| ├─ `palette_open` | `bool` | false | SetPaletteState(10) |
| ├─ `palette_selected` | `u8` | 0 | SetPaletteState(10) |
| ├─ `palette_available` | `u8` | 0 | SetPaletteState(10) |
| └─ `_pad` | `[u8; 1]` | [0] | (alignment) |

### 3.4 Wire Format (unchanged)

| Struct | Size | Status |
|--------|------|--------|
| `SilkBarUpdate` { kind: u32, index: u8, a: u32, b: u32 } | **16 bytes** | **unchanged** |
| `UPDATE_QUEUE_CAP` | **32** | **unchanged** |
| PDX opcodes (0xF0-0xF4) | — | **unchanged** |
| PDX slot (SLOT_SILKBAR=7) | — | **unchanged** |

## 4. Compatibility Notes

### 4.1 Backward Compatibility (old consumer + new producer)

Old sexdisplay/silkbar compiled against ABI v3 will:
- Receive `OP_SILKBAR_UPDATE` with `kind=8/9/10`
- `apply_update()` hits `_ => false` (unknown kind silently dropped)
- No crash, no panic, no undefined state
- Renders SilkBar without app/tint/palette (status quo)
- Old contract check: `ABI_VERSION=3 == SILK_DE_BAR_ABI_V1=3` → passes

### 4.2 Forward Compatibility (new consumer + old producer)

New sexdisplay compiled against ABI v4 will:
- Never receive `kind=8/9/10` from old producer
- `phase1` fields stay at default zeros
- Renders SilkBar with empty app/tint/palette (graceful degradation)
- New contract check: `ABI_VERSION=4 == SILK_DE_BAR_ABI_V1=4` → passes

### 4.3 Flag Day

**No flag day required.** Phases deploy incrementally:
1. Phase 1 (this): model only — old consumers reject new variants harmlessly
2. Phase 2: silk-shell sends new variants — old consumers reject harmlessly, new consumers receive
3. Phase 3: sexdisplay renders new fields
4. Phase 4: gate updated

Old and new binaries can coexist across PDX boundaries.

### 4.4 Deterministic Vector Extension

Three new vectors added to `validate_deterministic_vectors()`:
- `SetActiveApp(200)` → verifies `bar.phase1.active_app_sid == 200`
- `SetTintAccent(3)` → verifies `bar.phase1.accent_tint_idx == 3`
- `SetPaletteState(1 | (2<<1) | (7<<9))` → verifies open/selected/available

Expected applied count: 7→**10**.

## 5. Files Changed

- `crates/silkbar-model/src/lib.rs` — additive model changes:
  - Bumped `ABI_VERSION` 3→4, `SILK_DE_BAR_ABI_V1` 3→4, `SILKBAR_ABI_VERSION` 2→3
  - Added `UpdateKind::SetActiveApp = 8`, `SetTintAccent = 9`, `SetPaletteState = 10`
  - Added `SilkBarPhase1Ext` struct (active_app_sid, accent_tint_idx, palette_*)
  - Added `phase1: SilkBarPhase1Ext` field to `SilkBar` struct
  - Added `apply_update()` match arms for kinds 8/9/10
  - Extended `validate_deterministic_vectors()` with 3 new vectors + verification
  - Updated `DEFAULT_SILK_BAR` to initialize `phase1` to zeros
- `docs/handoff/SILKBAR_ABI_PHASE1_MODEL_V1.md` — this handoff

**NOT changed:**
- `SilkBarUpdate` struct (remains 16 bytes)
- `UPDATE_QUEUE_CAP` (remains 32)
- `UpdateKind` discriminants 0-7 (preserved)
- `apply_update()` arms 0-7 (preserved)
- `validate_contract()` logic except ABI_VERSION check (4==4 passes)
- `validate_invariants()` (unchanged, tests queue only)
- silk-shell, silkbar, sexdisplay, sex-pdx (zero changes)

## 6. Build/Proof Result

```
./scripts/entrypoint_build.sh → PASS (baseline, zero behavior change)
SEXOS_APP_LAUNCHER_MULTI_EXEC_PROOF=1 build → PASS (composes with proofs)

./scripts/run_daily_driver_proof.sh → 16/16 PASS, 0 faults
  launcher_multi_exec:     PASS 7/7 apps passed
  keyboard_gui:            PASS
  command_palette:         PASS
  spindle_daily:           PASS
  spindle_bridges:         PASS
  linen_nonblocking:       PASS
  linen_detail:            PASS
  quil_keyboard:           PASS
  bell_events:             PASS
  atlas_theme:             PASS
  collar_nav:              PASS
  mesh_nav:                PASS
  silkbar_status:          PASS
  palette_linen_available: PASS
  quil_status_ready:       PASS
  faults_zero:             PASS
```

## 7. Preserved Constraints

- No kernel edits
- No sex-pdx edits (opcodes, slots unchanged)
- No sexdisplay edits (render path unchanged)
- No silk-shell edits (producer unchanged)
- No silkbar server edits (producer unchanged)
- No UpdateKind renumbering (0-7 preserved)
- No SilkBarUpdate struct size change (16 bytes asserted)
- No queue capacity change (32 asserted)
- No broad refactor
- Backward compatible (unknown variants → `_ => false`)
- Compile-time assertions preserved (size, capacity, ABI_VERSION > 0)

## 8. Next Phases (not in this handoff)

| Phase | Mission | Handoff |
|-------|---------|---------|
| Phase 2 | silk-shell sends SetActiveApp/SetTintAccent/SetPaletteState | SILKBAR_ABI_PHASE2_SHELL_SENDS_V1.md |
| Phase 3 | sexdisplay receives + renders new fields | SILKBAR_ABI_PHASE3_DISPLAY_RENDER_V1.md |
| Phase 4 | proof profile gate update | SILKBAR_ABI_PHASE4_PROOF_GATE_V1.md |

## Handoff Path

```
docs/handoff/SILKBAR_ABI_PHASE1_MODEL_V1.md          ← THIS DOCUMENT
docs/handoff/SILKBAR_ABI_EXTENSION_PLAN_V1.md          ← design authority
docs/handoff/SILKBAR_KEYBOARD_STATUS_INTEGRATION_V1.md  ← documents ABI gaps (now resolved in model)
docs/handoff/SILKBAR_COMMAND_PALETTE_STATUS_RENDER_V1.md ← documents palette gap (now resolved in model)
```

