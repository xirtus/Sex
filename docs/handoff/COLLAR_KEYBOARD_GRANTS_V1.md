# COLLAR_KEYBOARD_GRANTS_V1

Status: PASS
Date: 2026-05-14

## Scope
- Added keyboard-proof usability for Collar grant inspection in `silk-shell`.
- No security policy or grant semantics changes.
- Approve/reject actions are explicitly skipped in proof to preserve policy.

## Files Changed
- servers/silk-shell/src/main.rs
- docs/handoff/COLLAR_KEYBOARD_GRANTS_V1.md

## What Changed
1. Added default-off proof gate:
- `SEXOS_COLLAR_KEYBOARD_GRANTS_PROOF`
- `maybe_run_collar_keyboard_grants_proof()`
- Invoked in existing proof-run chain.

2. Added minimal Collar UI state for keyboard proof:
- `COLLAR_SELECTED_GRANT_IDX`
- `COLLAR_OVERLAY_ENABLED`

3. Added bounded read-only grant navigator helpers over existing `COLLAR_GRANTS`:
- `collar_grant_count()`
- `collar_grant_at_visible_index(idx)`
- `collar_select_next_grant()`
- `collar_select_prev_grant()`
- `collar_emit_selected_grant_detail()`

4. Added required markers:
- `[collar.key.recv] code=N down=N mod=N`
- `[collar.grant.nav] old=N new=N count=N`
- `[collar.grant.detail] idx=N grant_id=N ok=N reason=...`
- `[collar.grant.action] action=skip grant_id=N ok=1 reason=policy_preserved_no_auto_grant`
- `[collar.overlay.toggle] enabled=N ok=N reason=...`
- `[collar.keyboard.grants.proof] stage=N action=NAME ok=N reason=...`
- `[collar.keyboard.grants.proof.done] ok=N`

5. Overlay marker integration:
- `toggle_collar()` now emits `collar.overlay.toggle` open/minimize markers.

## Build
- `SEXOS_COLLAR_KEYBOARD_GRANTS_PROOF=1 ./scripts/entrypoint_build.sh` -> PASS
- `./scripts/entrypoint_build.sh` -> PASS

## Runtime (Headless)
Log:
- `/tmp/sexos_collar_keyboard_grants_v1.log`

Observed:
- Open/focus: `[collar.overlay.toggle] enabled=1 ok=1 reason=opened_or_focused`
- Next/prev nav markers with count=12.
- Detail marker: `[collar.grant.detail] idx=0 grant_id=1 ok=1 reason=ok`
- Action marker (safe skip):
  - `[collar.grant.action] action=skip grant_id=1 ok=1 reason=policy_preserved_no_auto_grant`
- Close/back: `[collar.overlay.toggle] enabled=0 ok=1 reason=minimized`
- Done: `[collar.keyboard.grants.proof.done] ok=1`

Counts:
- `collar.key.recv`: 4
- `collar.grant.nav`: 2
- `collar.grant.detail`: 1
- `collar.grant.action`: 1
- `collar.overlay.toggle`: 2
- `collar.keyboard.grants.proof`: 7
- `collar.keyboard.grants.proof.done`: 1
- faults (`fault.kill|#PF|#GP|panic|KERNEL PANIC`): 0

## Outcome
- Collar open/focus/toggle proven.
- Grant navigation/detail proven.
- Approve/reject safely skipped (policy preserved, no auto-grant mutation).
- No kernel/ABI/USB/input/display/Quil changes.
