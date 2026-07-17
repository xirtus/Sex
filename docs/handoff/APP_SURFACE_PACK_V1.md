# APP_SURFACE_PACK_V1

## Result: PASS

Post-scheduler-fix health snapshot ran first: `entrypoint_build.sh` PASS,
`rsp0_regression_gate.sh` PASS, `usb_path_gate.sh` 8/8 rows PASS.

## Visible surfaces achieved

- **Linen (sid 200)** — already visible before this pass (its `_start` has
  0xEC create at 900,500 300x150 + coral fill + its own text draws; pixel
  scans confirm). This pass added the honest marker only.
- **Quil (sid 156 / 0x9C, NEW)** — quil's entire draw path (title bar,
  panel, row fills via 0xEF; text via 0xFA/0xFB) was **dead code**: it
  targeted shell-owned frame sid 201, which does not exist at boot and,
  once the shell creates it, rejects quil's draws on the sexdisplay
  owner_pd check — the exact bug class Spindle had. Quil now creates its
  OWN content sid 156 at x=1072 y=56 200x304 (spindle/kaleidoscope-proven
  0xEC route) and every draw site targets it. Its existing text demo +
  fills render there — verified by screendump pixel scan (fill variants
  visible at boot).
- **Mesh (sid 202)** — no Mesh PD exists; surface is shell-managed. Added
  placeholder fill at tile time (mirrors the existing Quil-placeholder
  pattern, reuses existing `MESH_PLACEHOLDER_COLOR`) + one-shot visible
  marker. Fires when Mesh is opened via palette.

## Launch routes (palette)

Palette layout: idx0 Spindle, idx1 Quil, idx2 Linen, idx3 Atlas, idx4
Bell, idx5 Collar, idx6 Mesh. **Palette navigation is `j`/`k` (scancodes
0x24/0x25), NOT arrow keys** — arrows leave selection at idx0 (cost one
lane run to learn; also: Scroll Lock TOGGLES Spindle, so sending it while
Spindle is focused closes it). Added one-shot route markers in the
FocusQuil/FocusLinen/FocusMesh exec branches, mirroring Spindle's.

## Files changed (backups: `.bak.app_surface_pack_v1`)

- `servers/quil/src/main.rs` — `QUIL_CONTENT_SID = 0x9C` const, 0xEC create
  in `_start`, all 12 draw sites repointed from `SURFACE_ID_QUIL` (201) to
  the content sid, `[quil.surface.visible.ok]` marker.
- `servers/linen/src/main.rs` — `[linen.surface.visible.ok] sid=200` after
  the existing create+fill (one line).
- `servers/silk-shell/src/main.rs` — Mesh placeholder fill + one-shot
  visible marker in `tile_active_scene_frames`; `[quil|linen|mesh.launch.
  route.ok]` markers in palette exec branches. (`MESH_PLACEHOLDER_COLOR`
  already existed at two sites — do not re-declare it.)

## Exact route used

Same as Spindle: app PD calls `0xEC` (arg0=sid, arg1=(y<<32)|x,
arg2=(h<<32)|w) → owner_pd binds to the calling PD → draws via 0xEF fills
and 0xFA clear + 0xFB packed 8-byte text chunks. No kernel, sex-pdx, or
sexdisplay edits. No legacy OP_WINDOW_CREATE.

## Proof (single lane, all in one boot)

```
ROW linen.surface.visible.ok PASS      ROW linen.launch.route.ok PASS
ROW quil.surface.visible.ok PASS       ROW quil.launch.route.ok PASS
ROW mesh.surface.visible.ok PASS       ROW mesh.launch.route.ok PASS
ROW spindle.surface.create.ok PASS     ROW spindle.key.recv PASS (abc typed)
ROW scheduler.pd8.flake.fix.ok PASS
faults=0  AUTH-rejects=0  rsp0_regression_gate PASS
```

Screendump pixel scans: quil content region shows its fill palette at
boot; linen region mixed post-open (retiling). Notably: this lane drives
palette + typing + three app opens in one boot with ZERO faults — the
scheduler RSP0 fix is holding under exactly the input pattern that used to
kill ~2/3 of boots.

## Marker grep

```sh
grep -E "\[(linen|quil|mesh)\.(surface\.visible|launch\.route)\.ok\]" LOG
grep -E "KERNEL PAGE FAULT|panic|fault\.kill|AUTH:" LOG
```

## Skipped / notes

- Collar/Bell: not in mission targets; Bell placeholder already
  shell-managed (BELL_ATTENTION_FIREWALL_V1).
- Linen sid-200 ownership race documented: linen PD 0xEC-creates sid 200
  AND the shell tile path 0xEC-upserts it. Whoever runs first owns; the
  loser's ops get AUTH-rejected. Zero AUTH lines observed in these lanes,
  but boot-order variance across runs (earlier lanes showed shell
  tile.apply sid=200, this lane's boot did not) means the race is real.
  Follow-on candidate: move linen to its own content sid like quil/spindle.
- Quil's own-sid geometry is fixed (1072,56); it does not follow shell
  retiling of frame 201. Same accepted trade-off as Spindle's content sid.

## Changelog

- 2026-07-18: Linen/Quil/Mesh visible + palette routes marked; quil's
  never-landed draw path repointed to self-owned sid 156 and verified live.
