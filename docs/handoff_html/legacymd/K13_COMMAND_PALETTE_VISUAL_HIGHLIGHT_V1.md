# K13: Command Palette Visual Selection Highlight

**Status:** Handoff (code + docs)
**Commit:** *(to be committed)*
**Purpose:** Give the command palette visible selection feedback within the current
single-0xEF-fill-rect constraint. Mirrors K6 Linen selection highlight pattern.

## 1. Changes

### 1.1 Accent Helper (`servers/silk-shell/src/main.rs`)

| Addition | Location | Description |
|----------|----------|-------------|
| `command_palette_selected_accent()` | ~line 5912 | Returns color for selected command via match; falls back to default muted blue-grey |

### 1.2 Updated Function

| Function | Change |
|----------|--------|
| `palette_render_list()` | Header fill rect now uses `command_palette_selected_accent()` instead of hardcoded `0x00404060`. Emits `[command_palette.selection_visual.header]` with command index and color. |

### 1.3 Accent Color Mapping

| Command | Color | Hex |
|---------|-------|-----|
| OpenSelectedInQuil | Amber (matching CodeFile) | `0x00C0A040` |
| FocusLinen | Green (matching Document) | `0x0040C080` |
| FocusQuil | Cyan (matching QuilWorkspaceRef) | `0x0040C0C0` |
| SceneNext | Indigo (matching Reference) | `0x006060C0` |
| OpenAtlas | Violet (matching MeshDiagnosticRef) | `0x00A060C0` |
| Fallback (invalid index) | Muted blue-grey | `0x00404060` |

Each color is visually distinct and reused from the existing shell color palette.

### 1.4 Refresh Wiring

Already wired from:
- `toggle_command_palette()` → `palette_show()` → `palette_render_list()`
- `palette_select_next()` → `palette_render_list()`
- `palette_select_prev()` → `palette_render_list()`

No new refresh points needed.

## 2. Proof Markers

| Marker | When |
|--------|------|
| `[command_palette.selection_visual.header] command=N index=N color=0xXXXXXX` | Each header draw |
| Existing `[command_palette.render]` | Renders start |
| Existing `[command_palette.row] ... selected=true/false` | Per-command proof |
| Existing `[command_palette.done]` | Renders complete |

## 3. Key Behavior

- **Header changes color** when J/K navigates commands, providing immediate visual feedback
- **No new 0xEF calls** — the header was already drawn each render; only the color changes
- **No sexdisplay changes** — sexdisplay treats the fill rect identically regardless of color
- **Deterministic** — same selection always produces same accent color

## 4. Files Changed

- `servers/silk-shell/src/main.rs` — added `command_palette_selected_accent()`, updated header color (~20 lines)
- `docs/handoff/K13_COMMAND_PALETTE_VISUAL_HIGHLIGHT_V1.md` — this document

## 5. Verification

- **Build:** `./scripts/entrypoint_build.sh` passes, ISO produced
- **Proof markers:** `[command_palette.selection_visual.header]` emitted on each palette render
- **No new primitives:** Still single 0xEF fill rect per surface
- **No changes:** kernel/ABI/sex-pdx, sexdisplay, command list, execution paths, J4-J7 semantics
