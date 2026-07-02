# K6: Linen Selection Visual Highlight

**Status:** Handoff (code + docs)
**Commit:** *(to be committed)*
**Purpose:** Give Linen selection visible feedback within the current
single-0xEF-fill-rect constraint. No new display primitives. No text rendering.

## 1. Changes

### 1.1 Accent Helper (`servers/silk-shell/src/main.rs`)

| Addition | Location | Description |
|----------|----------|-------------|
| `linen_selected_object_accent()` | ~line 488 | Returns color for selected object's kind via `linen_kind_color()`; falls back to `LINEN_LIST_HEADER_COLOR` if no selection |

### 1.2 Updated Function

| Function | Change |
|----------|--------|
| `linen_render_object_list()` | Header fill rect now uses `linen_selected_object_accent()` instead of constant `LINEN_LIST_HEADER_COLOR`. Emits `[linen.selection_visual.header]` with object_id and color. |

### 1.3 Refresh Wiring

Selection visual automatically refreshes because `linen_render_object_list()` is already called from:
- `SelectNextLinenObject` handler (after `linen_select_next_object()`)
- `SelectPrevLinenObject` handler (after `linen_select_prev_object()`)
- Linen open/focus path (existing `open_linen_in_active_scene()` / `focus_or_open_linen()`)

No new refresh points needed.

### 1.4 Accent Color Mapping

The accent uses the existing `linen_kind_color()` table, giving each object kind a distinctive header color:

| Kind | Color | Visual |
|------|-------|--------|
| Project | `0x004080C0` | Blue |
| Document | `0x0040C080` | Green |
| CodeFile | `0x00C0A040` | Amber |
| MediaAsset | `0x00C04080` | Magenta |
| BuildArtifact | `0x00806040` | Brown |
| Folder | `0x00808080` | Grey |
| Reference | `0x006060C0` | Indigo |
| ImportPlaceholder | `0x00C06040` | Orange |
| BellEventReference | `0x00C04040` | Red |
| QuilWorkspaceReference | `0x0040C0C0` | Cyan |
| MeshDiagnosticReference | `0x00A060C0` | Violet |
| Fallback (no selection) | `0x0038563A` | Default teal-green |

## 2. Proof Markers

| Marker | When |
|--------|------|
| `[linen.selection_visual.header] object_id=N color=0xXXXXXX` | Each header draw |
| `[linen.object_select.current] id=N` | Existing — confirms selection at render time |
| `[linen.object_list.row] ... selected=true/false` | Existing — per-object row proof |

## 3. Key Behavior

- **Header changes color** when J/K advances selection, providing immediate visual feedback
- **No new 0xEF calls** — the header was already drawn each render; only the color changes
- **No sexdisplay changes** — sexdisplay treats the fill rect identically regardless of color
- **Deterministic** — same selection always produces same accent color

## 4. Files Changed

- `servers/silk-shell/src/main.rs` — added `linen_selected_object_accent()`, updated header color call (~25 lines)
- `docs/handoff/K6_LINEN_SELECTION_VISUAL_HIGHLIGHT_V1.md` — this document

## 5. Verification

- **Build:** `./scripts/entrypoint_build.sh` passes, ISO produced
- **Proof markers:** `[linen.selection_visual.header]` emitted on each Linen list render
- **No new primitives:** Still single 0xEF fill rect per surface
- **No changes:** kernel/ABI/sex-pdx, sexdisplay, lifecycle, storage, editor, J4-J7 semantics

## 6. Future

- Full row highlighting requires multi-rect display support (STOP FIRST — sexdisplay change)
- Text rendering inside rows requires sexdisplay text primitive (STOP FIRST)
- Next after K6: K7 rapid audit K4-K6, then K8 = decide next UX
