# ATLAS_PHASE_E2_GATE_HYGIENE_V1

## 1) Root cause
`atlas_phase_e2_keyboard_scene_cycle` gate mixed proof enablement with generic runtime markers. It could fail on non-proof boots when generic Atlas key-cycle markers appeared without a matching `phase_e2.done` marker.

## 2) Exact old enablement bug
Old logic in `scripts/daily_driver_master_gate.sh` treated these generic markers as FAIL triggers even without explicit proof begin:
- `silk.atlas.key.scene.next]`
- `silk.atlas.key.scene.prev]`
- `silk.scene.active.set.*reason=atlas_key_cycle`
- `silk.atlas.mode.exit.*reason=atlas_key_cycle_done`

This caused false negative gate results in default daily-driver runs where E2 proof was not explicitly enabled.

## 3) New SKIP/PASS/FAIL semantics
Gate now requires explicit E2 begin sentinel first:
- Begin sentinel: `silk.atlas.phase_e2.begin]`
- If begin is missing: `SKIP`
- If begin exists and fault markers exist: `FAIL`
- If begin exists and `silk.atlas.phase_e2.done.*ok=1` exists: `PASS`
- If begin exists and `silk.atlas.key.scene.noop.*ok=1` exists: `PASS`
- Else (begin exists but no done/noop completion): `FAIL`

Generic key-next/prev/scene-cycle markers no longer enable or fail the gate by themselves.

## 4) Storage AP2 default SKIP remains correct
`sexdrive_storage_ioq_ready` gate behavior unchanged:
- Default run (no storage env): `SKIP`
- Message observed: `storage AP2 proof not requested or begin marker missing`

## 5) Verification output

### Syntax
- `bash -n scripts/daily_driver_master_gate.sh` -> PASS

### Existing log check
Command:
```bash
LOG=/tmp/sexos_daily_driver_proof.log
./scripts/daily_driver_master_gate.sh "$LOG" | grep -E "atlas_phase_e2_keyboard_scene_cycle|sexdrive_storage_ioq_ready|FAIL gates|FINAL"
```
Output:
- `atlas_phase_e2_keyboard_scene_cycle FAIL   phase_e2 begin without done/noop completion marker`
- `sexdrive_storage_ioq_ready   SKIP   storage AP2 proof not requested or begin marker missing`
- `FAIL gates: 1`
- `FINAL: FAIL (1 gate(s) failed)`

Interpretation: this log contains explicit E2 begin marker without completion marker, so FAIL is intentional under new semantics.

### Fresh default daily run check
Commands:
```bash
./scripts/run_daily_driver_proof.sh
./scripts/daily_driver_master_gate.sh /tmp/sexos_daily_driver_proof.log | grep -E "atlas_phase_e2_keyboard_scene_cycle|sexdrive_storage_ioq_ready|FAIL gates|FINAL"
```
Output:
- `atlas_phase_e2_keyboard_scene_cycle SKIP   phase_e2 proof not enabled (missing explicit begin marker)`
- `sexdrive_storage_ioq_ready   SKIP   storage AP2 proof not requested or begin marker missing`
- `FAIL gates: 1`
- `FINAL: FAIL (1 gate(s) failed)`

Note: remaining FAIL is unrelated (`atlas_phase_e1_click_scene_switch FAIL   hit markers without phase_e1.done`).

## 6) Reminder for future E2 proof
Future Phase E2 proofs must always emit explicit begin/done sentinels as scenario contract:
- `silk.atlas.phase_e2.begin`
- `silk.atlas.phase_e2.done`

Without explicit begin marker, gate intentionally stays `SKIP`.
