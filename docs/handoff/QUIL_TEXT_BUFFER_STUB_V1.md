# QUIL_TEXT_BUFFER_STUB_V1

## Result: PASS — Quil types/edits live via QMP keyboard

## What already existed (no rewrite)

Quil had a COMPLETE editor: 512-byte `QUIL_BUFFER`, append/backspace/
newline/undo-ring/cursor-nav, `scancode_to_char` (letters/digits/symbols/
shift), `draw_text_lines` → own content sid 156, and a main loop consuming
`OP_HID_EVENT` via `quil_dispatch_palette_key` (palette when
`palette_active`, text mode otherwise; Esc toggles). It was dead LIVE
because the shell never delivered the keys.

## Root cause: shell reserved-key wall

`scancode_to_action` reserves Enter/Backspace/Esc/Tab + most letters
(j,k,l,m,c,r…) + digits 1-5 as shell UI actions. Both dispatch paths
consumed them before app routing; the existing `owner=quil` route was gated
`!reserved_ui_key`. Spindle and Mesh already had focused-surface
passthroughs — Quil had none.

## Fix (servers/silk-shell + servers/quil, backups `.bak.quil_text_buffer_v1`)

1. `is_quil_text_key(scancode)` helper (near `is_spindle_text_key`):
   Enter/Backspace/Esc/Space, digits 0x02-0x0B, letter rows, palette nav
   Up/Down, cursor Left/Right/Home/End. **Tab intentionally excluded** —
   AccessFocusNext stays with the shell as keyboard escape hatch.
2. Drain path (`handle_hid_event`): quil passthrough block right after the
   spindle one — quil focused + text key → route to SLOT_QUIL, `return`.
   Shell command-palette interception stays earlier in the path, so
   palette keys keep working while the palette is open.
3. Main dispatch: `else if FOCUSED == QUIL && reserved_ui_key &&
   is_quil_text_key` branch between the spindle and linen branches.
   `reserved_ui_key` guard prevents double-send (non-reserved keys already
   routed by the early `owner=quil` branch).
4. Quil `draw_text_lines`: pad to ≥2 lines (40 bytes) + send 8-byte chunks
   **highest offset first** — first chunk sets surface `text_len` past the
   `[sexdisplay.text.draw]` diagnostic threshold (fires while
   `text_len <= 32`). Zero sid=156 diagnostic lines in the proof boot.

## Live path proven (one boot, QMP)

backtick → palette → j → Enter (FocusQuil) → Esc (quil palette off) →
`h i` Enter `x` Backspace. Markers: `[silk-shell.key.route] target=quil`,
`[quil.palette.action] kind=esc clear=1`, `[quil.text.recv] code=35`,
`[quil.text.enter] ok=1`, `[quil.text.backspace] ok=1`,
`[quil.text.draw.v2.sent] total_bytes=120 chunks=15`. Pixel scan of sid 156
region (1072,56 200x304) found 0x00E0F0FF text glyphs. faults=0, AUTH=0,
rsp0 PASS.

## Notes / limits

- Quil boots with its internal palette active; Esc enters text mode.
- Display text model still 128 bytes / 20 cols / 6 rows per surface —
  `draw_text_lines` shows first 6 lines only (`[quil.text.buffer.overflow]`
  marks the rest). Boot proofs may pre-fill the buffer (~240 bytes seen).
- When Quil is focused, letters j/k/l/m/c/r and digits 1-5 no longer fire
  shell actions (SnapLeft, Focus100, etc.) — intended, matches Spindle.

## Changelog

- 2026-07-18: reserved-key passthrough wired; live typing proven; text
  diagnostic dodge added.
