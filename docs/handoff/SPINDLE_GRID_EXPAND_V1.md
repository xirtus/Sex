# SPINDLE_GRID_EXPAND_V1

## Result: (filled after lane) — grid implemented, 40x12 over 2x2 owned surfaces

## Audit findings (sexdisplay, read-only)

Three hard limits in `servers/sexdisplay/src/main.rs`, all per-surface or
global, none configurable from apps:

1. **128-byte `text_buf` per surface** (`Surface.text_buf: [u8; 128]`,
   clamped in the 0xFB handler at `max_buf = 128`).
2. **`CHARS_PER_LINE = 20` hardcoded** in `surface_text_fg_at` — wrap width
   is NOT derived from surface width. A wider surface renders no more
   columns. 5x7 glyphs + 1px spacing = 6px/char, line height 9px, text
   inset x+8 / y+24. So one surface = 20 cols x 6 full rows (120 of 128
   bytes; row 6 would give only 8 chars — not used).
3. **`MAX_SURFACES = 16`** global slot table. CRITICAL: the 0xEC create
   path falls through **silently** when all 16 slots are active — no
   error, no marker, the surface just never exists. Any new surface must
   be budgeted against this table.

Multi-surface ownership: confirmed safe. `owner_pd` binds per-slot on
first create; nothing limits surfaces-per-PD. Slot pressure audit from the
APP_SURFACE_PACK_V1 lane log: distinct live sids ~10 at steady state
(100, 144 cursor, 152 palette, 153, 154, 156, 200, 201, 202, 300), with
collar 203 / bell 204 / browser 205 / panels 146-151 as on-demand extras.

## Chosen layout: 2x2 surfaces = 40 cols x 12 rows

- 4 surfaces total (3 new): sids **154 (0x9A), 160 (0xA0), 161 (0xA1),
  162 (0xA2)**; idx = band*2 + half; sid 154 kept as idx0 so existing lane
  markers (`[spindle.surface.create.ok] sid=154`) stay valid.
- Each surface 132x80 at (1008+half*132, 632+band*80) — 264x160 footprint,
  bottom-right of the 1280x800 screen, replacing the old 200x104 single
  surface at (1072,660).
- **Why not 6 surfaces (40x18)?** Steady state would hit 16/16 → any later
  collar/bell/browser/panel open silently loses its surface. 13/16 leaves
  3-slot headroom. "Largest safe", not "largest".

## Implementation (apps/spindle only, backup `.bak.grid_expand_v1`)

- `surf_flush(sid, sub)` — per-surface 0xFA clear + 15 x 0xFB 8-byte
  chunks, highest offset first (keeps the `text_len <= 32` serial-spam
  dodge per surface).
- `content_render` — builds one 40x12 grid (scrollback rows 0-10
  bottom-aligned, prompt `> ` + line tail on row 11), splits into four
  20x6 subgrids, **dirty-diffs against `GRID_PREV` cache** and flushes
  only changed surfaces. A plain keystroke reflushes 1 surface (16 pdx
  calls), Enter/scroll reflushes up to 4 (64 calls) — bounded, no
  per-keystroke 4x cost.
- `_start` creates all 4 surfaces in a loop; markers
  `[spindle.grid.expand.begin]`, `[spindle.grid.surface.ok] sid=N idx=N`.
- Budgeted `[spindle.grid.render.ok] cols=40 rows=12 surfaces=4` replaces
  the old `[spindle.render.frame.ok]` (no gate depended on it).
- Editor state untouched: CmdLine/History/Scrollback/ghost/vi-mode logic
  unmodified; only the render target changed.

## Proof

```
./scripts/entrypoint_build.sh                      # PASS
GATE_DIR=/tmp/claude-1000/grid1 grid_proof.sh      # lane below
./scripts/rsp0_regression_gate.sh $G/l.log         # PASS
```

Lane: boot → Scroll Lock → type `help` + Enter (long help lines >20 chars
fill the right-half surfaces) → `he` + Tab (ghost accept) + Enter → up/down
(history nav) → `abc` on prompt → screendump. Checks: grid markers for all
4 idx, ghost/history/echo markers, zero faults/AUTH, rsp0 gate, and a
per-quadrant pixel scan for text color 0x00E8FFFF proving all four
surfaces render glyphs.

Marker grep:
```sh
grep -E "\[spindle\.grid\.(expand\.begin|surface\.ok|render\.ok)\]" LOG
grep -E "\[spindle\.(ghost\.accept|history\.nav|input\.echo\.ok)\]" LOG
```

## Remaining hard ceiling

- 40 cols x 12 rows is the practical max at 13/16 slots. Going wider/taller
  needs a sexdisplay text-model change (STOP FIRST): per-surface wrap
  width derived from `w`, and/or a larger `text_buf`. That single change
  would make one surface arbitrarily large and free 3 slots.
- Ghost autosuggest is functional (Tab accept + marker) but not visually
  dim on the grid: `text_color` is one color per surface, so a dim ghost
  needs either a second overlay surface (slot cost) or per-char color in
  sexdisplay (STOP FIRST).
- Glyph coverage 0x20-0x5A only (lowercase folded to uppercase).
- Fixed geometry: grid does not follow shell retiling of frame 153 (same
  accepted trade-off as before).

## Changelog

- 2026-07-18: 20x6 single-surface terminal → 40x12 across 4 owned
  surfaces with dirty-diff flush. sexdisplay untouched.
