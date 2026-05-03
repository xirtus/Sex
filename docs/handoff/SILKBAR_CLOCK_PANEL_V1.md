# SILKBAR_CLOCK_PANEL_V1

## Status: PASS (2026-05-03)

## Summary
Clock click on SilkBar now toggles a simple OS-owned clock panel surface (id=0x94).
No calendar/time subsystem — pure surface toggle proof, same pattern as launcher/status.
Clock panel uses existing 0xEC/0xEE surface create/destroy path.

## Proof Chain
```
[shell.silkbar.click] target=clock x=1100 y=25
[shell.clock.open.start] id=0x94
[shell.clock.open.ok] id=0x94
[shell.silkbar.click] target=clock x=1100 y=25
[shell.clock.close.start] id=0x94
[shell.clock.close.ok] id=0x94
```

## PASS Criteria Verified
- [x] `[shell.silkbar.click] target=clock` — clock hit-test dispatch
- [x] `[shell.clock.open.ok] id=0x94` — clock surface created
- [x] `[shell.clock.close.ok] id=0x94` — clock surface destroyed (second click toggle)
- [x] launcher/status/workspace all preserved
- [x] `[silk.contract.validate.ok] version=1`
- [x] `[silk.render_proof.top_strip.ok]`
- [x] No PF/GP/panic

## Files Changed

### servers/silk-shell/src/main.rs
- Added `SURFACE_ID_CLOCK = 0x94` constant (after `SURFACE_ID_STATUS = 0x93`)
- Added `CLOCK_ACTIVE: bool` static toggle state (after `STATUS_ACTIVE`)
- Modified `Action::OpenClock` handler in `handle_silkbar_click()`:
  - Opens clock panel at (1000, 55) with size 240×300 via 0xEC
  - Closes clock panel via 0xEE (second click toggles)
  - Emits `[shell.clock.open.start/ok]` and `[shell.clock.close.start/ok]` markers

### servers/sexinput/src/main.rs
- Added synthetic close click for clock panel (stages 19-21 at ticks 27-29)
  - Proves the close/toggle path works end-to-end

## Architecture
- **No kernel edits, no PDX ABI changes, no sexdisplay edits**
- Clock panel uses same 0xEC/0xEE surface path as launcher and status panels
- Panel id=0x94 is OS-owned, distinct from launcher (0x92), status (0x93), cursor (0x90)
- Position: x=1000, y=55, w=240, h=300 — under the right-side clock area
- Toggle behavior: click → open, click again → close (same as launcher/status)
- No real calendar UI — action proof only

## Surface ID Allocation
| ID  | Surface        | Owner      |
|-----|----------------|------------|
| 0x90| Cursor         | OS (shell) |
| 0x92| Launcher panel | OS (shell) |
| 0x93| Status panel   | OS (shell) |
| 0x94| Clock panel    | OS (shell) |
| 100+| App surfaces   | Apps       |

## Consolidation Note
After all three panel toggles (launcher/status/clock) are proven, the next step is to consolidate the duplicated panel toggle state into a single shared mechanism before adding more features (Bell, etc.).
