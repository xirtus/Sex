# SILKBAR_ACTION_SLOT_EXPANSION_V1

## Status: PASS (2026-05-03)

## Summary
Expanded SilkBar ABI contract from v1 to v2 to add a Bell bar slot between Battery chip and Clock. Established the hit-test target, layout geometry, and rendering color for the Bell module. No Bell panel behavior (toggle open/close) — that comes in a separate `BELL_PANEL_TOGGLE_V1` patch.

## Proof Chain
```
[silk.contract.validate.ok] version=2
[sexinput.synthetic.silkbar_click] target=bell
[shell.silkbar.click] target=bell x=1025 y=25
[silk.render_proof.top_strip.hash] value=0x70a68011ec352490
[silk.render_proof.top_strip.ok]
```

## PASS Criteria Verified
- [x] `[silk.contract.validate.ok] version=2` — both sexdisplay and silkbar print ABI v2 contract
- [x] `[shell.silkbar.click] target=bell x=1025 y=25` — shell hit-test dispatches Bell action
- [x] `[sexinput.synthetic.silkbar_click] target=bell` — synthetic Bell click fires at ticks 31-33
- [x] `[silk.render_proof.top_strip.ok]` — top strip renders correctly (hash includes Bell slot)
- [x] Existing clicks preserved (launcher, workspace switch, status, clock panel toggles)
- [x] No panel toggle behavior (OpenBell handler returns true without toggle_os_panel())
- [x] No PF/GP/panic during runtime

## Files Changed

### crates/silkbar-model/src/lib.rs
- Added `CHIP_X_BELL = 1020` constant (between Battery at 1004 and Clock at 1090)
- Increased `LAYOUT_COUNT` from 10 to 11
- Bumped `ABI_VERSION`, `SILK_DE_BAR_ABI_V1` from 1 to 2
- Bumped `SILKBAR_ABI_VERSION` from 1 to 2
- Added `ModuleSlot::Bell = 10`
- Added `Module::Bell` variant
- Added `Action::OpenBell` variant
- Added Bell layout box to `DEFAULT_SILK_BAR` at `(CHIP_X_BELL, CHIP_Y, CHIP_W, CHIP_H)` with `Module::Bell, Action::OpenBell`

### servers/silk-shell/src/main.rs
- Added `Action::OpenBell` arm in `handle_silkbar_click()`:
  - Prints `[shell.silkbar.click] target=bell x={} y={}`
  - Returns `true` (no panel toggle — reserved for future patch)

### servers/sexdisplay/src/main.rs
- Added Bell rendering in `bar_color()`: checks `module_rect(bar, ModuleSlot::Bell)` and returns gold `0x00FFD700` for pixels in the Bell slot

### servers/sexinput/src/main.rs
- Added synthetic Bell click stages (22-24) at ticks 31-33:
  - Stage 22: EV_ABS at (1025, 25)
  - Stage 23: BTN down (left press)
  - Stage 24: BTN up (left release)
  - Prints `[sexinput.synthetic.silkbar_click] target=bell`

## Architecture
- **Bell slot position**: x=1020, y=18, w=18, h=22 — between Battery chip (end at x=1004) and Clock (start at x=1090)
- **Bell is NOT a chip**: It occupies `ModuleSlot::Bell = 10`, not a chip slot. MAX_CHIPS stays 4.
- **ABI v2 contract**: Both silkbar (producer) and sexdisplay (consumer) validate `SILKBAR_ABI_VERSION == 2` at startup via `validate_silkbar_contract()`.
- **Hit-test dispatch**: `hit_test_action()` iterates layout[0..LAYOUT_COUNT] (now 11 boxes) — Bell is last. Shell's `handle_silkbar_click()` matches `Action::OpenBell`.
- **No renderer changes**: Bell color comes from `bar_color()` inline, same as launcher and chips. No new render passes.

## Bell Panel (Next Patch)
After this ABI expansion is proven runtime, a separate `BELL_PANEL_TOGGLE_V1` patch will:
1. Add `SURFACE_ID_BELL = 0x95` to silk-shell
2. Add `static mut BELL_ACTIVE: bool = false`
3. Wire `Action::OpenBell` to `toggle_os_panel(&mut BELL_ACTIVE, SURFACE_ID_BELL, "bell", ...)`
4. Add Bell panel rendering in sexdisplay's `draw_launcher_panel()`-style function
5. Add Bell surface surface area redraw trigger
