# SYNTHETIC_SILKBAR_CLICK_DISABLE_V1

## Root Cause
A scripted synthetic SilkBar click sequence in `sexinput` was firing launcher/status/clock/bell clicks via HID events at fixed ticks.
That repeatedly toggled shell panels and created recurring grey-panel behavior while real clock progression remained healthy.

## Change
- File: `servers/sexinput/src/main.rs`
- Added dedicated gate:
  - `SILKBAR_CLICK_PROOF_ENABLED = option_env!("SEXOS_SILKBAR_CLICK_PROOF").is_some()`
- Changed synthetic SilkBar click block to run only when:
  - `!SYNTHETIC_INPUT_PROOFS_DISABLED && SILKBAR_CLICK_PROOF_ENABLED`
- Added one-time marker when disabled:
  - `[sexinput.synthetic.silkbar_click.disabled]`

## Default Behavior
- Synthetic SilkBar click proof is now **OFF by default**.
- Real/manual keyboard/mouse input paths are unchanged.
- Other proof paths (e.g. drag proof, click-focus proof, optional F5/F6 keyboard proof) are unchanged by this patch.

## Re-enable Intentionally
Build with:
```sh
SEXOS_SILKBAR_CLICK_PROOF=1 ./scripts/entrypoint_build.sh
```

## GUI Proof Checklist
1. Build and boot normally (without `SEXOS_SILKBAR_CLICK_PROOF`).
2. Verify no recurring launcher/status/clock panel auto-toggle loop.
3. Verify clock still counts.
4. Verify tiled windows still open.
5. Verify manual real input still works.
6. Verify logs:
```sh
rg -n "sexinput.synthetic.silkbar_click.disabled|sexinput.synthetic.silkbar_click" /tmp/sexos.log
```
Expected in default mode:
- disabled marker present
- no `sexinput.synthetic.silkbar_click target=...` sequence.
