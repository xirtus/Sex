# FRAME_LIGHT_STARTUP_CHROME_LIVENESS_V1

## Root Cause
Early startup chrome metadata for Quil/Linen was sent with hardcoded `OP_SURFACE_TAB_INFO` flags that omitted close-allowed bit 5.

Result:
- startup render used default disabled close-light appearance,
- later shell updates eventually sent policy-correct chrome, so red enabled "halfway through" boot.

This is the same early/late state split class as clock startup liveness.

## Early/Late State Split Found
Early path:
- boot sends `OP_SURFACE_TAB_INFO` with only top-bar/hover bits for initial frames.
- close_allowed bit not present, so sexdisplay renders red as disabled.

Late path:
- later tab/chrome updates include policy-derived state and red becomes enabled.

## Files Changed
- `servers/silk-shell/src/main.rs`
- `servers/sexdisplay/src/main.rs`
- `scripts/daily_driver_master_gate.sh`

## Minimal Diff Summary
### silk-shell
- Boot-time Quil/Linen chrome sends now include close_allowed bit 5 immediately.
- Added startup marker per boot seed:
  - `[shell.frame.light.startup.seed] frame=N sid=S close_allowed=C sent=1`

### sexdisplay
- Added startup chrome receive marker in `OP_SURFACE_TAB_INFO` handler:
  - `[sexdisplay.frame.light.chrome.recv] frame=N sid=S close_allowed=C flags=0x..`
- Added bounded startup render marker when rendering frame lights:
  - `[sexdisplay.frame.light.startup.render] sid=S red=enabled/disabled close_allowed=C reason=...`
- Renderer still only uses model flags; no policy inference was added.

### gate
- Hardened `frame_lights_stub` startup-liveness rule:
  - requires bounded early enabled red render marker distance,
  - fails if early window is only disabled renders,
  - still requires protected/system disabled proof.
- Reports first startup frame-light render line, first enabled line, and distance.

## Gate Rule
`frame_lights_stub` PASS now requires:
1. startup render marker exists,
2. first enabled startup render appears within bounded distance,
3. protected/system frame remains `close_allowed=0`.

## Proof Result
Command:
- `./scripts/run_daily_driver_proof.sh /tmp/sexos_frame_light_startup_chrome_liveness_v1.log`

Result:
- `FINAL: PASS (123 gates proved, 0 skipped, 0 faults)`
- `clock_visible_seconds` remains PASS
- frame-lights startup rule PASS with bounded distance

Key markers:
- `[shell.frame.light.startup.seed] ... close_allowed=1 sent=1`
- `[sexdisplay.frame.light.chrome.recv] ... close_allowed=1 flags=0x...`
- `[sexdisplay.frame.light.startup.render] ... red=enabled close_allowed=1 ...`
- `[silk.frame.lights.state] ... reason=protected_system_frame ... close_allowed=0`

## Remaining Risks
- Startup liveness proof currently checks bounded marker distance in serial logs; true GUI timing can still vary by host load.
- No protocol ABI change was introduced; only existing bit-5 semantics were used earlier in boot.
