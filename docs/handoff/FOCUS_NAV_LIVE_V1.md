# FOCUS_NAV_LIVE_V1

## Result: PASS — Bell/Collar/Atlas/Linen-Enter keyboard paths now live

## Root cause (systemic — third instance of the same wall)

`scancode_to_action` reserves j/k/Enter/Esc/Backspace/digits/arrows as
shell UI actions. Several focused-surface branches were BOTH gated
`!reserved_ui_key` AND whitelisted only reserved scancodes → statically
dead for live input; only synthetic proofs (which call handlers directly)
ever exercised them. Quil had this (fixed in QUIL_TEXT_BUFFER_STUB_V1);
this lane fixed the remaining instances:

| Path | Before | Fix |
|------|--------|-----|
| Bell nav (j/k/Enter/Esc/Bksp/[/]) | main-dispatch branch dead, no drain block | guard dropped + drain passthrough |
| Atlas digit-select + arrows | dead (0x02-0x06, 0x4B/0x4D reserved) | guard dropped + drain passthrough (F10 still falls through to ToggleAtlas; drain acts on value==1 only) |
| Collar grant nav | proof-only, NO live branch anywhere | new focused-collar branch in both paths: j/k nav, Enter detail, Esc/Bksp toggle_collar |
| Linen Enter → OpenIntent → Quil | Enter dead (AccessActivate won), only Space worked | guard dropped; linen-focused Enter now opens object, not AccessActivate |

## Files

`servers/silk-shell/src/main.rs` only, backup `.bak.focus_nav_live_v1`.
Drain-path blocks inserted after the Mesh passthrough, before the text
sink; main-dispatch collar branch between Bell and Mesh branches.

## Proof (one boot, QMP, all PASS)

Palette-driven: idx4 Bell → j/k/Enter/Esc; idx5 Collar → j/k/Enter/Esc;
F10 Atlas → left/right/digit-1; idx2 Linen → Enter. 12/12 rows:
bell_key_recv/nav/enter, collar_key_recv/nav/detail, atlas_nav/select,
linen_enter, faults=0, AUTH=0, rsp0 PASS.

```sh
grep -E "\[(bell|collar)\.key\.recv\]|\[bell\.nav\.move\]|\[collar\.grant\.(nav|detail)\]|\[atlas\.nav\.(move|activate)\]|\[linen\.open_intent\.(send|skip)\]" LOG
```

## Notes / behavior changes

- While Bell/Collar focused: j/k/Enter/Esc/Bksp (+ [/] for Bell) no longer
  fire shell actions — same accepted trade-off as Spindle/Quil/Mesh.
- Atlas overlay open = modal: all keys except F10 go to Atlas.
- Bell/Collar nav with empty rings still emits nav markers
  (`count=0` / `reason=single_or_empty`) — delivery proven regardless.
- Bell/Collar drain blocks mirror Mesh exactly, including acting on both
  key edges (press+release) — pre-existing Mesh behavior, kept for
  consistency; Atlas gated to press only (scene switch is heavier).

## Usable-apps scoreboard after this lane

Spindle (terminal, grid, ghost/history) ✓ · Quil (live editor) ✓ ·
Linen (visible sid 157, j/k select, Enter/Space open) ✓ · Mesh (PD graph +
node nav) ✓ · Bell (toggle, nav, detail, lane cycle) ✓ · Collar (toggle,
grant nav, detail) ✓ · Atlas (overlay, arrows, digit select) ✓ ·
Palette/SilkBar/text-sink ✓ · WebStub: no surface, no network — deferred.

## Changelog

- 2026-07-18: reserved-key wall removed for Bell/Atlas/Collar/Linen-Enter;
  all registry apps now keyboard-usable live.
