# VISIBLE_CLOCK_REDRAW_SOURCE_FIX_V1

## Root Cause
`sexdisplay` had a timing/order visibility issue in the top-strip redraw path: redraw markers could emit repeated `s=0` while later SetClock/fallback updates had already advanced internal state.

This showed up as:
- early repeated `[sexdisplay.clock.redraw] ... s=0 source=silkbar`
- later `[sexdisplay.clock.source.silkbar.apply] ... ss=1..N`

So the visible redraw path needed an explicit canonical clock source check and synchronization point.

## Variable Split Found
State writers:
- SetClock apply path writes `bar.clock_hh/mm/ss`
- fallback tick path writes `bar.clock_hh/mm/ss`

State reader for visible top-strip:
- `redraw_top_strip(..., &bar)` reads `bar.clock_hh/mm/ss` for redraw marker + digit draw.

Fix: introduce canonical latch (`CLOCK_CANON_*`) updated at all clock-write points, then apply canonical values to `bar` immediately before post-drain redraw.

## Files Changed
- `servers/sexdisplay/src/main.rs`

## Minimal Diff Summary
- Added canonical clock globals:
  - `CLOCK_CANON_HH/MM/SS`
  - `clock_canon_store(...)`
  - `clock_canon_apply_to_bar(...)`
- Update canonical clock at:
  - startup init from `DEFAULT_SILK_BAR`
  - fallback raw-tick increment path
  - fallback synthetic-loop increment path
  - SetClock apply path
- Before `redraw_top_strip`, call `clock_canon_apply_to_bar(&mut bar)`.
- Added bounded proof marker in redraw:
  - `[sexdisplay.clock.redraw.source_check] redraw_ss=S canonical_ss=C source=...`

No kernel/ABI/sex-pdx/scheduler/geometry/frame-light edits.

## Proof Commands
1. `./scripts/entrypoint_build.sh`
2. `./scripts/run_daily_driver_proof.sh /tmp/sexos_visible_clock_redraw_source_fix.log`
3. attempted GTK visual boot:
   - `LOG=/tmp/sexos_clock_visual_verify_v2.log`
   - `qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku -cdrom ./sexos-v1.0.0.iso -device nec-usb-xhci,id=xhci -device usb-mouse,bus=xhci.0 -serial file:"$LOG" -display gtk -boot d`

## Proof Markers (from daily proof log)
From `/tmp/sexos_visible_clock_redraw_source_fix.log`:
- Early boot (before clock updates):
  - `redraw_ss=0 canonical_ss=0`
- After SetClock starts arriving:
  - `[sexdisplay.clock.source.silkbar.apply] ... ss=3`
  - `[sexdisplay.clock.redraw] ... s=3`
  - `[sexdisplay.clock.redraw.source_check] redraw_ss=3 canonical_ss=3`
- Continued progression examples:
  - `... s=14 canonical_ss=14`
  - `... s=30 canonical_ss=30`
  - `... s=37 canonical_ss=37`

This proves redraw state equals canonical state and advances.

## Runtime Result
- Daily proof: `PASS` (`FINAL: PASS`, `faults_zero PASS`)
- Fault scan: no `#PF/#GP/panic/fault.kill`

## Visual Result
- GTK visual boot command in this environment failed with `gtk initialization failed`.
- Therefore direct interactive visual confirmation was not possible here.
- Serial evidence in proof log confirms redraw seconds advance and match canonical state.
