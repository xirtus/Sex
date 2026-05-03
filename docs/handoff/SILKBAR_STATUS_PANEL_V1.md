# SILKBAR_STATUS_PANEL_V1

## Status: PASS (2026-05-03)

## Summary
Status chip clicks on SilkBar now toggle a simple OS-owned status panel surface (id=0x93).
No real settings controls yet — pure surface toggle proof, same pattern as launcher panel.
Status panel uses existing 0xEC/0xEE surface create/destroy path (same as launcher).

## Proof Chain
```
[shell.silkbar.click] target=status x=940 y=25
[shell.status.open.start] id=0x93
[shell.status.open.ok] id=0x93
[shell.silkbar.click] target=launcher x=100 y=25
[shell.launcher.close.start]
[shell.launcher.close.ok] id=0x92
[shell.silkbar.click] target=status x=940 y=25
[shell.status.close.start] id=0x93
[shell.status.close.ok] id=0x93
```

## PASS Criteria Verified
- [x] `[shell.silkbar.click] target=status` — status chip hit-test dispatch
- [x] `[shell.status.open.ok] id=0x93` — status surface created
- [x] `[shell.status.close.ok] id=0x93` — status surface destroyed (second click toggle)
- [x] launcher open/close still works
- [x] workspace active send still works (index=2)
- [x] `[silk.contract.validate.ok] version=1`
- [x] `[silk.render_proof.top_strip.ok]`
- [x] No PF/GP/panic

## Files Changed

### servers/silk-shell/src/main.rs
- Added `SURFACE_ID_STATUS = 0x93` constant (after `SURFACE_ID_LAUNCHER = 0x92`)
- Added `STATUS_ACTIVE: bool` static toggle state (after `LAUNCHER_ACTIVE`)
- Modified `Action::ToggleModule(_module)` handler in `handle_silkbar_click()`:
  - Opens status panel at (860, 55) with size 200×300 via 0xEC
  - Closes status panel via 0xEE (second click toggles)
  - Emits `[shell.status.open.start/ok]` and `[shell.status.close.start/ok]` markers

### servers/sexinput/src/main.rs
- Added synthetic close click for status panel (stages 16-18 at ticks 23-25)
  - Proves the close/toggle path works end-to-end

## Architecture
- **No kernel edits, no PDX ABI changes, no sexdisplay edits**
- Status panel uses same 0xEC (surface create) / 0xEE (surface destroy) path as launcher
- Panel id=0x93 is OS-owned, distinct from launcher (0x92), cursor (0x90), and app surfaces (100+)
- Position: x=860, y=55, w=200, h=300 — under the right-side status chip area
- Toggle behavior: click → open, click again → close (same as launcher)
- No real settings controls — action proof only, reserved for future quick settings / Bell / system state
- Click-focus, drag, workspace switching, and launcher all preserved

## Surface ID Allocation
| ID  | Surface       | Owner      |
|-----|---------------|------------|
| 0x90| Cursor        | OS (shell) |
| 0x92| Launcher panel| OS (shell) |
| 0x93| Status panel  | OS (shell) |
| 100+| App surfaces  | Apps       |

## Next Steps
- **Clock panel**: similar toggle for clock click (OpenClock action)
- **Real settings**: populate status panel with quick settings controls (wifi, brightness, etc.)
