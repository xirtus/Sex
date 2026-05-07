# QUIL_STATIC_TEXT_ONLY_V1

## Files Touched
- `servers/quil/src/main.rs`
- `docs/handoff/QUIL_STATIC_TEXT_ONLY_V1.md`

## Proof Method
- Kept strict scope to Quil only.
- Did not touch kernel, sex-pdx, sexinput, silk-shell, SilkBar, lifecycle/F9, or SexFiles save/restore logic.
- Added a static bounded visual proof badge inside Quil's title area using existing fill-rect primitive (`OP_SURFACE_FILL_RECT` / `0xEF`).
- Added marker log: `[quil.static.proof.v1] mode=visual_badge`.

## Why Not Real Text
- Current `sexdisplay` in this tree has no active public glyph/text opcode path for Quil.
- Existing opcode `0xED` is focus operation in sexdisplay, not text draw.
- Therefore no full text renderer/font subsystem was added.
- This phase uses visual bounded proof only, per guardrail.

## What Was Not Attempted
- No keyboard/input edits.
- No autosave/restore or lifecycle wiring.
- No shell/F9 hooks.
- No sexdisplay policy rewrite.
- No framebuffer ownership change.

## Build Result
- `./scripts/entrypoint_build.sh` => `[SEXOS ENTRYPOINT] success`

## GUI Verification Checklist
1. Clock still counts in SilkBar.
2. Tiled windows still open and behave as before.
3. Quil shows static visual proof badge in its surface/title region.
4. System remains stable (no visible regressions).
5. Optional serial grep for faults if log available:
   - no `PAGE FAULT`
   - no `GENERAL PROTECTION`
   - no `EXCEPTION`
