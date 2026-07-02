# ATLAS_PHASE_E1_GATE_HYGIENE_V1

## 1) Root cause
`atlas_phase_e1_click_scene_switch` gate mixed proof enablement with generic runtime click/hit/scene-switch markers. In non-proof/default boots, this could create false `FAIL` outcomes without an explicit E1 proof scenario contract.

## 2) Exact old enablement bug
Old logic in `scripts/daily_driver_master_gate.sh` failed on generic markers even without explicit proof begin:
- `silk.atlas.hit.scene]`
- `silk.atlas.click.consume]`
- `silk.atlas.mode.exit.*reason=atlas_card_click`
- `silk.scene.active.set.*reason=atlas_card_click`

These were treated as proof evidence/error triggers rather than runtime noise unless an explicit E1 scenario was asserted.

## 3) New SKIP/PASS/FAIL semantics
E1 gate now mirrors E2 hygiene semantics:
- Begin sentinel: `silk.atlas.phase_e1.begin]`
- If begin missing: `SKIP`
- If begin exists and fault markers exist: `FAIL`
- If begin exists and `silk.atlas.phase_e1.done.*ok=1` exists: `PASS`
- If begin exists and `silk.atlas.phase_e1.negative.empty_click.*ok=1` exists: `PASS`
- Else (begin exists but no done/negative completion): `FAIL`

Generic click/hit/scene-active markers no longer enable/fail the gate by themselves.

## 4) Proof that E2 hygiene remains intact
Observed after E1 patch:
- `atlas_phase_e2_keyboard_scene_cycle SKIP   phase_e2 proof not enabled (missing explicit begin marker)`

No E2 gate logic changes were made.

## 5) Proof that storage AP2 default SKIP remains correct
Observed after E1 patch:
- `sexdrive_storage_ioq_ready SKIP   storage AP2 proof not requested or begin marker missing`

Storage AP2 semantics are unchanged.

## 6) Verification output

### Syntax
- `bash -n scripts/daily_driver_master_gate.sh` -> PASS

### Existing log check (`/tmp/sexos_daily_driver_proof.log` before fresh rerun)
Command:
```bash
LOG=/tmp/sexos_daily_driver_proof.log
./scripts/daily_driver_master_gate.sh "$LOG" | grep -E "atlas_phase_e1_click_scene_switch|atlas_phase_e2_keyboard_scene_cycle|sexdrive_storage_ioq_ready|FAIL gates|FINAL"
```
Output:
- `atlas_phase_e1_click_scene_switch FAIL   phase_e1 begin without done/negative completion marker`
- `atlas_phase_e2_keyboard_scene_cycle SKIP   phase_e2 proof not enabled (missing explicit begin marker)`
- `sexdrive_storage_ioq_ready SKIP   storage AP2 proof not requested or begin marker missing`
- `FAIL gates: 1`
- `FINAL: FAIL (1 gate(s) failed)`

Interpretation: explicit E1 begin existed in that log without completion marker, so FAIL is intentional under the new sentinel-gated semantics.

### Fresh default daily run check
Commands:
```bash
./scripts/run_daily_driver_proof.sh
./scripts/daily_driver_master_gate.sh /tmp/sexos_daily_driver_proof.log | grep -E "atlas_phase_e1_click_scene_switch|atlas_phase_e2_keyboard_scene_cycle|sexdrive_storage_ioq_ready|FAIL gates|FINAL"
```
Output:
- `atlas_phase_e1_click_scene_switch PASS   click scene switch proof complete`
- `atlas_phase_e2_keyboard_scene_cycle SKIP   phase_e2 proof not enabled (missing explicit begin marker)`
- `sexdrive_storage_ioq_ready SKIP   storage AP2 proof not requested or begin marker missing`
- `FAIL gates: 0`
- `FINAL: PASS (256 gates proved, 99 skipped, 0 faults)`

## 7) Reminder: future E1 proof must use explicit begin/done sentinels
Phase E1 scenario contract must keep explicit sentinels:
- `silk.atlas.phase_e1.begin`
- `silk.atlas.phase_e1.done`

Without explicit begin, gate remains `SKIP` by design.
