# K11: Command Palette Stub

**Status:** Handoff (code + docs)
**Commit:** *(to be committed)*
**Purpose:** Implement the K10-designed command palette as a shell-owned action router.
No text input. No fuzzy search. No app manifests. No new execution paths.

## 1. Changes

### 1.1 Constants (`servers/silk-shell/src/main.rs`)

| Addition | Value | Location |
|----------|-------|----------|
| `SURFACE_ID_COMMAND_PALETTE` | `0x98` | ~line 87 |
| `MAX_FRAMES` | `8` (was 7) | ~line 2280 |
| `ATLAS_MAX_FRAMES_PER_SCENE` | `8` (was 7) | ~line 3029 |
| `APP_SURFACES` | `[AppSurfaceSpec; 6]` (was 5) | ~line 121 |
| `COMMAND_PALETTE_FRAME_ID` | `7` | ~line 5651 |
| Boot geometry | 400,200,480,240 | ~line 5653-5656 |

### 1.2 Command Model

```rust
enum Command {
    OpenSelectedInQuil = 0,
    FocusLinen = 1,
    FocusQuil = 2,
    SceneNext = 3,
    OpenAtlas = 4,
}
```

5 commands maximum. Static const array, no heap, no dynamic registration.

### 1.3 State

| Variable | Type | Purpose |
|----------|------|---------|
| `COMMAND_PALETTE_OPEN` | `bool` | Whether palette is currently shown |
| `COMMAND_PALETTE_SELECTED` | `u8` | Index of selected command (0-4) |

### 1.4 Functions Added

| Function | Purpose |
|----------|---------|
| `ensure_command_palette_frame()` | Create/find ShellFrame for palette (frame_id=7) |
| `palette_render_list()` | Draw header via 0xEF + proof-marker rows |
| `palette_show()` | Geometry upsert via 0xEC + render |
| `toggle_command_palette()` | Open/close the palette overlay |
| `palette_select_next()` | Advance selection (wrap around) |
| `palette_select_prev()` | Move selection backward (wrap around) |
| `palette_execute_selected()` | Route selected command to existing action path |

### 1.5 Dispatch Integration

| Location | Change |
|----------|--------|
| `SurfaceAction` enum | Added `ToggleCommandPalette` variant |
| `scancode_to_action()` | Added `0x29` → `ToggleCommandPalette` |
| Keyboard intercept (line ~8399) | Added palette mode: J/K navigate, Enter execute, Escape/backtick close |
| Action handler (line ~8739) | Added `ToggleCommandPalette` → `toggle_command_palette()` |

### 1.6 Keyboard Model (When Palette Open)

| Key | Scancode | Action |
|-----|----------|--------|
| J | 0x24 | Select next command (wrap) |
| K | 0x25 | Select previous command (wrap) |
| Enter | 0x1C | Execute selected command + close palette |
| Escape | 0x01 | Close palette |
| Backtick | 0x29 | Close palette |

## 2. Execution Routing

| Command | Routes To | Gate |
|---------|-----------|------|
| OpenSelectedInQuil | `open_linen_object_in_quil()` | `FOCUSED_SURFACE_ID == SURFACE_ID_LINEN` |
| FocusLinen | `open_linen_in_active_scene()` | None (toggle always works) |
| FocusQuil | `open_quil_in_active_scene()` | None (toggle always works) |
| SceneNext | `switch_scene()` | Scene existence |
| OpenAtlas | `atlas_toggle()` | None (toggle always works) |

All commands reuse existing action paths. Zero new execution paths.

## 3. Proof Markers

| Marker | When |
|--------|------|
| `[command_palette.attach.frame]` | Frame creation |
| `[command_palette.attach.tab]` | Tab attach |
| `[command_palette.open]` | Palette opened |
| `[command_palette.close]` | Palette closed |
| `[command_palette.render]` | Header render |
| `[command_palette.row]` | Per-command proof row (index, cmd, name, selected) |
| `[command_palette.select]` | Selection changed |
| `[command_palette.execute]` | Command executed |
| `[command_palette.reject]` | Execution rejected (e.g. not focused) |
| `[command_palette.done]` | Render complete |

## 4. Files Changed

- `servers/silk-shell/src/main.rs` — constants, enums, state, helpers, dispatch (~200 lines)
- `docs/handoff/K11_COMMAND_PALETTE_STUB_V1.md` — this document

## 5. Verification

- **Build:** `./scripts/entrypoint_build.sh` passes, ISO produced
- **MAX_FRAMES:** 7→8 (shell-local, no ABI)
- **ATLAS_MAX_FRAMES_PER_SCENE:** 7→8 (internal, no ABI)
- **APP_SURFACES:** 5→6 (internal, no ABI)
- **No changes:** kernel/ABI/sex-pdx, sexdisplay, lifecycle, storage, editor
- **Zero new execution paths:** All commands route through existing handlers
