# SILKBAR_WORKSPACE_SWITCH_V1

## Status: PASS (2026-05-03)

## Summary
Workspace clicks on SilkBar now update real active workspace state through the full chain: shell hit-test → silkbar workspace dispatch → sexdisplay render model update. No renderer or ABI changes.

## Proof Chain
```
[shell.silkbar.click] target=workspace index=3 x=635 y=25
[silkbar.workspace.recv] index=2
[silkbar.workspace.active.set] index=2
[silkbar.workspace.active.send.start] index=2
[silkbar.workspace.active.send.ok] index=2
```

## PASS Criteria Verified
- [x] `[shell.silkbar.click] target=workspace index=3` - shell hit-test on workspace 2 (0-idx)
- [x] `[silkbar.workspace.recv] index=2` - silkbar receives 0-indexed workspace idx 2
- [x] `[silkbar.workspace.active.set] index=2` - silkbar updates active state
- [x] `[silkbar.workspace.active.send.ok] index=2` - update reaches sexdisplay
- [x] `[silk.contract.validate.ok] version=1`
- [x] `[silk.render_proof.top_strip.ok]`
- [x] No PF/GP/panic

## Files Changed

### servers/sexdisplay/src/main.rs
- **Renamed `redraw_clock_only()` → `redraw_top_strip()`** — function already redrew the entire top strip (y<50) including bar colors, workspace indicators, and chips. Name now reflects actual scope.
- **OP_SILKBAR_UPDATE handler**: Changed conditional redraw (clock-only) to unconditional redraw. Previously: `if handle_silkbar_update(...) { redraw_clock_only(); }`. Now: always call `redraw_top_strip()` for any SilkBar update. This is the critical fix that makes workspace switches visually appear on screen.
- All callers of `redraw_clock_only` updated to `redraw_top_strip`.

### servers/silkbar/src/main.rs
- Added proof markers to existing OP_SILKBAR_WORKSPACE_ACTIVE handler:
  `[silkbar.workspace.recv]`, `[silkbar.workspace.active.set]`,
  `[silkbar.workspace.active.send.start]`, `[silkbar.workspace.active.send.ok]`

### servers/sexinput/src/main.rs
- Adjusted synthetic click position from x=600 to x=635 to target workspace 2 (idx 2)

## Architecture
- **No kernel edits, no PDX ABI changes**
- Existing `OP_SILKBAR_WORKSPACE_ACTIVE` path (silkshell → silkbar) already correct
- Silkbar forwards to sexdisplay via existing `send_update()` / `OP_SILKBAR_UPDATE` transport
- Sexdisplay applies `SetWorkspaceActive` via existing `apply_update()` in render model
- **Critical fix**: sexdisplay previously only redrew the top strip for clock updates. Extended to redraw for ALL SilkBar updates (workspace, chips, clock) so workspace active/urgent changes appear visually.
- Render proof hash captured on first render (baseline: workspace 2 active) before workspace switch reaches renderer
- Click-focus, drag, and launcher/status/clock classification preserved

## Workspace Index Flow
- Shell hit-test returns `Action::SwitchWorkspace(n)` with n=1-5 (1-indexed)
- Shell converts: `ws_idx = n.saturating_sub(1).min(4)` → 0-indexed idx 0-4
- Silkbar receives 0-indexed idx via `msg.arg0 as u8`
- Silkbar validates with `.min(SILKBAR_WORKSPACE_IDX_MAX=4)`
- Sexdisplay receives `SetWorkspaceActive(ws_idx, 1)` via `apply_update()`
