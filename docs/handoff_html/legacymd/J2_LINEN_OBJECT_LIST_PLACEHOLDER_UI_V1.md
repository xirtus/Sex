# J2: Linen Object List Placeholder UI

**Status:** Approved
**Commit:** *(pending)*
**Build:** Passed (ISO produced)
**Behavior:** Unchanged (additive UI + proof markers only)

## Purpose

Add a minimal placeholder object list UI inside the existing Linen surface path.
Uses existing display/shell primitives only. No storage, no new PDX ops, no
sexdisplay changes.

## Rendering Constraint

Sexdisplay supports exactly ONE fill rect (0xEF) per surface. This limitation
makes multi-row text lists impossible with current primitives. J2 works within
this constraint:

- **Header bar** drawn via 0xEF fill rect at top of Linen surface (teal-green)
- **Per-object rows** emitted as proof markers only (serial output)
- No individual row rectangles — deferred to future fill-rect expansion or
  text rendering in sexdisplay

## Changes to `servers/silk-shell/src/main.rs`

### Visual constants (Change 1)
- `LINEN_LIST_MAX_ROWS: u8 = 8` — max proof marker rows
- `LINEN_LIST_ROW_H: u32 = 24` — row height (reserved for future visual expansion)
- `LINEN_LIST_ROW_GAP: u32 = 2` — gap between rows
- `LINEN_LIST_HEADER_COLOR: u32 = 0x0038563A` — teal-green header
- `LINEN_LIST_HEADER_H: u32 = 28` — header bar height

### Kind-to-color mapping (Change 2)
- `linen_kind_color(kind)` — returns accent color per LinenObjectKind
  - Project: blue, Document: green, CodeFile: amber, MediaAsset: magenta,
    BuildArtifact: brown, Folder: grey, Reference: indigo,
    ImportPlaceholder: orange, BellEventReference: red,
    QuilWorkspaceReference: cyan, MeshDiagnosticReference: violet

### Render function (Change 3)
- `linen_render_object_list()` — draws header bar via 0xEF, emits per-object
  proof markers, loops LINEN_OBJECTS up to LINEN_LIST_MAX_ROWS

### Call sites (Change 4)
- Duplicate guard in `open_linen_in_active_scene()` — renders on focus
- Main open path — renders after `[linen.object_table.ready]`, before
  `snap_capture_layout()`

## Proof Markers Added

| Marker | Location | Trigger |
|--------|----------|---------|
| `[linen.object_list.render]` | linen_render_object_list() | Render start, includes geometry |
| `[linen.object_list.row]` | linen_render_object_list() | Per visible object (id, kind, state, name) |
| `[linen.object_list.skip]` | linen_render_object_list() | Object skipped (max_rows exceeded) |
| `[linen.object_list.done]` | linen_render_object_list() | Render complete (count + rows emitted) |

## Invariants

1. No new sexdisplay primitives — uses existing 0xEF only
2. No sexdisplay understanding of Linen objects — shell owns policy
3. No storage/filesystem access
4. No heap allocation
5. No kernel/ABI/sex-pdx edits
6. Visual: single fill rect (header bar). Row rendering is proof-only until
   sexdisplay supports multiple fill rects or text rendering.

## Deferred

- Per-row visual rectangles (requires sexdisplay multi-fill-rect or text support)
- Object selection/highlight
- Object list scrolling
- Search/filter

## Dependencies

- **Requires:** J1 (Linen object table)
- **Blocks:** J4 (open Linen object into Quil buffer)
