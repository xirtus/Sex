# MESH_READONLY_PD_GRAPH_V1

## Result: PASS

## What

Mesh (shell-managed sid 202, no Mesh PD) now renders a read-only PD/app
surface list as real text, from the shell's existing frame/registry model.
No new IPC, no kernel queries, nothing written back.

## Implementation (servers/silk-shell only)

- `mesh_render_pd_graph()` (next to `mesh_render_fact_list`): builds
  6 lines x 20 chars = 120 bytes (fits sexdisplay's 128-byte `text_buf`;
  uppercase only — glyph coverage ends 0x5A):

  ```
  PD GRAPH RO
  SPINDLE 153 V F
  QUIL    201 V -
  LINEN   200 V -
  MESH    202 V -
  BELL    204 M -
  ```

  State column from live frame model: `V` frame visible, `M` minimized,
  `-` no frame (`frame_for_surface` + `FRAME_FLAG_MINIMIZED`). Focus
  column `F` from `FOCUSED_SURFACE_ID`. Sent via existing
  `shell_draw_text` after `OP_TEXT_CLEAR`.
- Called from the Mesh placeholder block in `tile_active_scene_frames`
  (fires on every retile that includes Mesh — open, F12 toggle, scene
  changes), so the list refreshes on the events that change it most.
- Marker: `[mesh.pd_graph.render.ok] rows=6 bytes=120 ok=1`.

## Proof

Combined lane (one boot): F12 opens Mesh → marker ok=1; pixel scan found
0x00E8FFFF text glyphs outside the spindle grid region. faults=0, AUTH=0,
rsp0 PASS.

## Known bounded noise

`shell_draw_text` sends chunks lowest-offset-first, so the first 4 chunks
trip sexdisplay's `text_len <= 32` diagnostic → exactly 4
`[sexdisplay.text.draw] sid=202` lines per Mesh render. Rare event,
bounded, left as-is (fixing means touching shared `shell_draw_text`).

## Limits / follow-ons

- Focus/minimize column is a snapshot at tile time; a focus change without
  a retile leaves it stale until the next Mesh open/toggle.
- Rows are the 5 registry apps + header; no real PD ids (shell doesn't
  track kernel PD numbers — that data lives only in kernel spawn markers).
- Mesh fact-list rect rows still render beneath the text (text composites
  on top).

## Changelog

- 2026-07-18: read-only PD graph text list live on Mesh.
