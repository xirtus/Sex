# K3: Quil Buffer List Placeholder UI

**Status:** Handoff (implemented)
**Commit:** *(to be committed)*
**Build:** *(verified)*

## 1. Purpose

Show Quil buffer table as a minimal placeholder buffer list inside the existing
Quil surface path. Uses existing display/shell primitives only. No editor, no
text rendering, no new PDX ops. Mirrors the J2 Linen object list pattern.

## 2. Changed Files

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | +QUIL_LIST constants, +quil_render_buffer_list(), wired into open_quil_in_active_scene() and open_linen_object_in_quil() |
| `docs/handoff/K3_QUIL_BUFFER_LIST_PLACEHOLDER_UI_V1.md` | This document |

## 3. Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `QUIL_LIST_MAX_ROWS` | 8 | Maximum visible rows |
| `QUIL_LIST_HEADER_COLOR` | 0x00302E56 | Deep blue-purple, distinct from Linen teal-green |
| `QUIL_LIST_HEADER_H` | 28 | Header bar height in pixels |

## 4. Render Behavior

`quil_render_buffer_list()` performs:

1. 0xEF fill rect at (0,0) with header bar color covering full width × 28px
2. Iterates QUIL_BUFFERS[0..16], emits [quil.buffer_list.row] for each slot
   with buffer_id, kind, state_name, linen_object_ref, linked_surface_id,
   display_name
3. Skips rows beyond QUIL_LIST_MAX_ROWS with [quil.buffer_list.skip] marker
4. Emits [quil.buffer_list.done] with count and rows_emitted

Expected output on boot:
```
[quil.buffer_list.render] w=640 h=480
[quil.buffer_list.row] buffer_id=1 kind=Code state=Open linen_ref=0 surface_id=201 name=main.rs
[quil.buffer_list.row] buffer_id=2 kind=Text state=Open linen_ref=2 surface_id=201 name=Compositor Lifecycle Spec
[quil.buffer_list.row] buffer_id=3 kind=DesignNote state=Open linen_ref=0 surface_id=0 name=Frame Tiling Design
[quil.buffer_list.row] buffer_id=4 kind=BuildOutput state=ReadOnly linen_ref=5 surface_id=0 name=Current ISO Build
[quil.buffer_list.row] buffer_id=5 kind=ReviewNote state=Open linen_ref=0 surface_id=0 name=Review: A7 Opcode Audit
[quil.buffer_list.row] buffer_id=6 kind=AgentTask state=Allocated linen_ref=0 surface_id=0 name=Refactor tiling loop
[quil.buffer_list.done] count=6 rows=6
```

After J4 link (PrintScreen → open object 3 → new dynamic buffer 1003):
```
[quil.buffer_list.row] buffer_id=1003 kind=Code state=Open linen_ref=3 surface_id=201 name=Silk Shell main.rs
[quil.buffer_list.done] count=7 rows=7
```

## 5. Wire Points

1. `open_quil_in_active_scene()` — after `[quil.placeholder.open]` and
   `[quil.buffer_table.ready]`, before `snap_capture_layout()`. Renders on
   every fresh Quil surface open (not on duplicate-focus).
2. `open_linen_object_in_quil()` — step 10, after J6 mesh diagnostic and J7
   bell event. Refreshes the buffer list to show the newly created dynamic
   buffer.

## 6. Proof Markers

| Marker | Location | Trigger |
|--------|----------|---------|
| `[quil.buffer_list.render]` | quil_render_buffer_list() | Start of render with geometry |
| `[quil.buffer_list.row]` | quil_render_buffer_list() | Per visible buffer with metadata |
| `[quil.buffer_list.skip]` | quil_render_buffer_list() | Buffer beyond max_rows limit |
| `[quil.buffer_list.done]` | quil_render_buffer_list() | Render complete with counts |

## 7. Safety Invariants

1. Uses existing 0xEF fill rect primitives only — no sexdisplay changes.
2. No heap allocation — stack-only iteration of static table.
3. Read-only — never mutates QUIL_BUFFERS or LINEN_OBJECTS.
4. Safe degradation — empty table produces `count=0 rows=0`, not panic.
5. No editor/parser/compiler — buffer list metadata only.
6. Additive — existing Quil surface lifecycle, fill rect, and focus paths unchanged.

## 8. Forbidden Areas Untouched

- kernel/ crates/sex-pdx/ servers/sexdisplay/ servers/linen/ servers/quil/
- PDX ABI/opcodes, lifecycle enum, tombstone ring
- Real editor/parser/compiler/build code

## 9. STOP FIRST Status

**No STOP FIRST triggers hit.**
