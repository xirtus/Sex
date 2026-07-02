# ATLAS_PHASE_E1_EXPLICIT_PROFILE_GATE_V1

## 1. Exact default log evidence

From `/tmp/sexos_daily_driver_proof.log`:

- `9650:[silk.atlas.phase_e1.begin] active=0 scenes=1`
- `9855:[silk.atlas.phase_e1.done] from=0 to=1 ok=1`
- `9970:[silk.atlas.phase_e1.negative.empty_click] ok=1`

No explicit E1 proof-profile begin sentinel is present in this default log:

- `[silk.atlas.phase_e1.proof.begin]`
- `[silk.atlas.phase_e1.click_scene_switch.proof.begin]`
- `[atlas.phase_e1.proof.begin]`

## 2. Why normal phase_e1/click/hit markers are insufficient

`[silk.atlas.phase_e1.begin]` and related runtime markers can occur during normal/default Atlas runtime activity. Using those generic runtime markers as proof enablement causes unrelated default dailies to evaluate the E1 proof gate when no explicit proof profile was requested. Gate hygiene requires an explicit profile sentinel to distinguish requested proof runs from ordinary runtime behavior.

## 3. New explicit proof-profile rule

`atlas_phase_e1_click_scene_switch` now behaves as:

- `SKIP` unless at least one explicit sentinel exists:
  - `[silk.atlas.phase_e1.proof.begin]`
  - `[silk.atlas.phase_e1.click_scene_switch.proof.begin]`
  - `[atlas.phase_e1.proof.begin]`
- `PASS` only when explicit sentinel exists and completion exists:
  - `silk.atlas.phase_e1.done.*ok=1`
  - or `silk.atlas.phase_e1.negative.empty_click.*ok=1`
- `FAIL` only when explicit sentinel exists but completion is missing, or when fault markers are present.

## 4. Storage AP gates unaffected confirmation

Unchanged storage gates on default run:

- `sexdrive_storage_ioq_ready   SKIP   storage AP2 proof not requested or begin marker missing`
- `sexdrive_storage_single_block_rw SKIP   storage AP3 proof not requested`
- `sexdrive_storage_multiblock_rw SKIP   storage AP4 proof not requested`
- `sexdrive_storage_reboot_persistence SKIP   storage AP5a proof not requested`
- `sexdrive_storage_flush_durability SKIP   storage AP5b proof not requested`

## 5. Verification output

Syntax check:

- `bash -n scripts/daily_driver_master_gate.sh` -> `syntax_ok`

Gate run on current default log:

- `atlas_phase_e1_click_scene_switch SKIP   phase_e1 proof not enabled (missing explicit proof begin marker)`
- `atlas_phase_e2_keyboard_scene_cycle SKIP   phase_e2 proof not enabled (missing explicit proof begin marker)`
- `FAIL gates: 0`
- `FINAL: PASS (255 gates proved, 104 skipped, 0 faults)`

Fresh default run (`./scripts/run_daily_driver_proof.sh` then gate):

- `atlas_phase_e1_click_scene_switch SKIP   phase_e1 proof not enabled (missing explicit proof begin marker)`
- `atlas_phase_e2_keyboard_scene_cycle SKIP   phase_e2 proof not enabled (missing explicit proof begin marker)`
- `sexdrive_storage_ioq_ready   SKIP   storage AP2 proof not requested or begin marker missing`
- `sexdrive_storage_single_block_rw SKIP   storage AP3 proof not requested`
- `sexdrive_storage_multiblock_rw SKIP   storage AP4 proof not requested`
- `sexdrive_storage_reboot_persistence SKIP   storage AP5a proof not requested`
- `sexdrive_storage_flush_durability SKIP   storage AP5b proof not requested`
- `FAIL gates: 0`
- `FINAL: PASS (255 gates proved, 104 skipped, 0 faults)`
