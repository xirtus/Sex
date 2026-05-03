# BELL_PANEL_TOGGLE_V1

**Status:** PASS (2026-05-03)
**Prerequisite:** SILKBAR_ACTION_SLOT_EXPANSION_V1 (Bell ABI v2 proven)
**Boundary:** silk-shell only (panel toggle via existing `toggle_os_panel()` API)

---

## What It Proves

The Bell module slot in the SilkBar issues a click action that toggles a shell-owned panel surface at id=0x95, using the same `toggle_os_panel()` mechanism as launcher/status/clock panels.

## Implementation

**Files changed:**
- `servers/silk-shell/src/main.rs`
  - Added `SURFACE_ID_BELL: u64 = 0x95`
  - Added `BELL_ACTIVE: bool` toggle state
  - Wired `Action::OpenBell` → `toggle_os_panel(&mut BELL_ACTIVE, SURFACE_ID_BELL, "bell", 600, 55, 240, 300)`

**No changes to:** kernel, sex-pdx, sexdisplay, silkbar-model, sexinput, sexusb

## Proof Chain

1. Synthetic SilkBar click targets Bell slot (x=1025, y=25)
2. Shell hit-test dispatches to `Action::OpenBell`
3. `toggle_os_panel()` creates or destroys surface at id=0x95
4. sexdisplay renders the panel surface via generic surface path

## Proof Markers

```
[sexinput.synthetic.silkbar_click] target=bell
[shell.silkbar.click] target=bell x=1025 y=25
[shell.bell.open.start] id=0x95
[shell.bell.open.ok] id=0x95
```

## Invariants

- Bell is a **shell-owned panel surface toggle** — not a popup, not a toast, not a notification system
- Bell uses the same `toggle_os_panel()` helper as launcher/status/clock panels — no new rendering or IPC protocols
- Bell position (600, 55, 240, 300) is within the generic surface rendering bounds
- No focus/drag stealing — Bell panel follows same surface lifecycle as other panels
- sexdisplay renders Bell panel via existing generic surface path — no Bell-specific renderer logic

## Negative Proof

- Clicking Bell a second time closes the panel (toggle semantics)
- Bell panel open does not affect drag operations on other surfaces
- Bell panel open does not steal focus from focused app surfaces

## Verification

```bash
./scripts/entrypoint_build.sh
./dev.sh run-nographic 2>/tmp/bell-panel.trace | tee /tmp/bell-panel.log
rg "target=bell|bell.open|bell.close" /tmp/bell-panel.log
rg "fault|panic|GP|PF" /tmp/bell-panel.log
```
