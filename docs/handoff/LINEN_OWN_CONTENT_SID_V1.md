# LINEN_OWN_CONTENT_SID_V1

## Result: PASS

## Problem

Linen's `_start` 0xEC-created sid 200 — same sid the shell 0xEC-upserts as
Linen's tiling frame. First creator wins `owner_pd`; loser's draws get
AUTH-rejected. Boot-order varied between runs (documented in
APP_SURFACE_PACK_V1). Race, not deterministic.

## Fix (servers/linen only, backup `.bak.linen_own_sid_v1`)

- New `LINEN_CONTENT_SID = 157` (0x9D — verified free across servers/apps).
- `_start` creates sid 157 (same rect 900,500 300x150) and repoints the
  coral fill to it. Linen never touches sid 200 again; shell owns the
  frame sid outright — race gone by construction.
- `SURFACE_ID_LINEN = 200` kept (`#[allow(dead_code)]`) as documentation of
  the shell-frame sid; shell-side focus/route logic (sid 200) unchanged.
- Markers: `[linen.content.sid.ok] sid=157 reason=own_content_sid_no_shell_race`
  and `[linen.surface.visible.ok] sid=157`.

## Slot budget

+1 sexdisplay slot (sid 200 shell frame + sid 157 linen content are now two
slots). Steady state 14/16 after SPINDLE_GRID_EXPAND_V1's 13/16. Headroom 2
for on-demand collar/bell/browser — next surface addition must re-audit
against the silent 0xEC fall-through at 16/16.

## Proof

Combined lane with QUIL_TEXT_BUFFER_STUB_V1 + MESH_READONLY_PD_GRAPH_V1
(one boot): `linen_content_sid` / `linen_visible_157` / `linen_no_auth_200`
all PASS; whole-boot faults=0, AUTH=0, rsp0 gate PASS.

```sh
grep -E "\[linen\.(content\.sid|surface\.visible)\.ok\]" LOG
grep "AUTH:" LOG   # must be empty
```

## Trade-off

Fixed geometry: sid 157 does not follow shell retiling of frame 200 — same
accepted trade-off as quil (156) and spindle (154/160-162).

## Changelog

- 2026-07-18: linen moved off shell-frame sid 200 to own content sid 157.
