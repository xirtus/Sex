# C3: Atlas Visual Polish Tokens

**Status:** Approved
**Commit:** *(pending)*
**Build:** Passed (ISO produced)
**Behavior:** Unchanged (additive token constants + visual polish only)

## Purpose

Add visual polish token constants and per-card rim rendering to the Atlas
overview (C1/C2). No new display primitives, no sexdisplay changes, no ABI
edits, no thumbnails.

## Changes to `servers/silk-shell/src/main.rs`

### 1. New Token Constants (after `ATLAS_COLOR_SELECT`)

**Canonical naming aliases** (reference existing colors by new name):
- `ATLAS_BG_COLOR` = `ATLAS_COLOR_BG`
- `ATLAS_CARD_COLOR` = `ATLAS_COLOR_CARD_SCENE`
- `ATLAS_CARD_ACTIVE_COLOR` = `ATLAS_COLOR_CARD_ACTIVE`
- `ATLAS_CARD_EMPTY_COLOR` = `ATLAS_COLOR_CARD_EMPTY`
- `ATLAS_CARD_MINIMIZED_HINT_COLOR` = `ATLAS_COLOR_FRAME_MINIMIZED`
- `ATLAS_CARD_ZOOMED_HINT_COLOR` = `ATLAS_COLOR_FRAME_ZOOMED`

**New polish tokens** (distinct colors not previously present):
- `ATLAS_CARD_SELECTED_COLOR: u32 = 0x005050ff` — violet-blue accent for nav-selected card
- `ATLAS_CARD_ACTIVE_RIM_COLOR: u32 = 0x004090c0` — muted cyan rim for active scene card
- `ATLAS_CARD_INACTIVE_RIM_COLOR: u32 = 0x00204060` — very dim rim for inactive cards

### 2. `atlas_render_stub()` Visual Polish

**Card color selection updated:**
- Selected scene card (nav cursor): uses `ATLAS_CARD_SELECTED_COLOR` (violet-blue)
- Active scene card: uses `ATLAS_CARD_ACTIVE_COLOR` (brighter blue)
- Empty scene card: uses `ATLAS_CARD_EMPTY_COLOR` (dim)
- Default: uses `ATLAS_CARD_COLOR` (medium blue)

**Frame block colors updated:**
- `ATLAS_CARD_ZOOMED_HINT_COLOR` for zoomed scenes
- `ATLAS_CARD_MINIMIZED_HINT_COLOR` for minimized scenes

**Per-card rim rendering added:**
- Selected card: 2px bright cyan border (existing, unchanged)
- Active non-selected card: 1px muted neon rim (`ATLAS_CARD_ACTIVE_RIM_COLOR`)
- Inactive non-empty card: 1px very dim rim (`ATLAS_CARD_INACTIVE_RIM_COLOR`)
- Empty cards: no rim (keeps visual hierarchy clean)

### 3. Preserved
- C1 snapshot lifecycle filtering (unchanged)
- C2 navigation/focus behavior (unchanged)
- B3 tiling (unchanged)
- B4 tab chrome (unchanged)
- No new opcodes
- No renderer policy changes
- No sexdisplay changes

## Proof Markers Added

| Marker | Location | Trigger |
|--------|----------|---------|
| `[atlas.visual.tokens]` | atlas_render_stub() | Render begins, tokens in use |
| `[atlas.visual.card]` | Per-card loop | Card color resolved and drawn |
| `[atlas.visual.selected]` | Per-card loop | Card is the nav-selected scene |
| `[atlas.visual.active]` | Per-card loop | Card is the active (non-selected) scene |
| `[atlas.visual.flags]` | Per-card loop | Scene flags and frame count reported |

## Invariants

1. All token constants are `const u32` — zero runtime cost, no allocation
2. Aliases reference existing `ATLAS_COLOR_*` constants — no color duplication
3. Rim drawing only adds 0xEF calls — no new opcodes or display protocol
4. Empty scenes get no rim — clean visual distinction
5. All existing behavior paths preserved (non-Atlas rendering unaffected)

## Deferred

- Thumbnail-style card previews (requires renderer support)
- Scene-specific accent color application per SCENE_APPEARANCE_STATE
- Card label text rendering (requires sexdisplay text support)
- Mouse-based Atlas interaction
- Scene pinning visual indicator

## Dependencies

- **Requires:** C1 (Atlas snapshot/view), C2 (Atlas navigation/focus)
- **Blocks:** None (visual polish only, no downstream dependency)
