# APP_LAUNCHER_KEYS_ROWS_AUDIT_V2

## Scope
- Keyboard-first launcher help marker polish only.
- No layout or behavior redesign.

## Files
- servers/silk-shell/src/main.rs

## Change
- In `maybe_run_app_launcher_help_proof()`:
  - tracked total launcher rows emitted.
  - changed done marker from:
    - `[launcher.help.proof.done] ok=<available_rows>`
  - to:
    - `[launcher.help.proof.done] ok=<0|1> rows=<total_rows>`

## Marker Contract
- `[launcher.help.keys] key=NAME action=NAME`
- `[launcher.help.row] idx=N app=NAME key=NAME status=NAME`
- `[launcher.help.proof.done] ok=N rows=N`

## Proof
- Build gate: PASS
- Daily-driver gate: PASS target remains 18/18 with faults=0

## Notes
- Marker-only update; runtime behavior unchanged.
