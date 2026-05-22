# SILK_DE_INTEGRATED_INTERACTION_PROOF_V1

## Scope
Integrated proof/gate/handoff for Silk DE interaction stack after:
- `d0b1296e` silk: lock Silk DE bar contract
- `3d20cc16` silk: add deterministic top strip proof
- `09606b59` silk: prove renderer conformance

No kernel edits. No sex-pdx ABI edits. No protocol/ABI edits. No broad refactor.

## Baseline
- Baseline commit in expected chain: `09606b59`

## Files Changed
- `scripts/daily_driver_master_gate.sh`
- `docs/handoff/SILK_DE_INTEGRATED_INTERACTION_PROOF_V1.md`

## Integrated Proof Model
- Type: script rollup gate with explicit sentinel semantics
- Gate name: `silk_de_integrated_interaction`
- Runtime marker requirement for strict mode: `[silk.de.integrated.interaction.begin]`

## Evidence Categories
The integrated gate requires these categories when begin is present:
- contract: `silk_de_contract_lock=PASS`
- topstrip: `silk_de_topstrip_deterministic=PASS`
- renderer: `silk_de_renderer_conformance=PASS`
- clock/chip liveness: `clock_visible_seconds=PASS`
- pointer/focus: `silk-shell.pointer.recv` and `shell.interact.focus|shell.focus.set`
- drag/resize/snap/lifecycle:
  - `shell.interact.drag.(begin|move|end)`
  - `silk.resize.(begin|apply|end)`
  - `silk.snap.(hit|apply|none)`
  - `silk.close.(request|allowed|tombstone)|lifecycle.destroy.record|tombstone.event.record`
- faults: `faults_zero=PASS`

## SKIP Semantics
`silk_de_integrated_interaction` is `SKIP` when either is true:
- `[silk.de.integrated.interaction.begin]` absent
- explicit `[silk.de.integrated.interaction.skip] reason=not_requested` present

This keeps normal boots non-blocking.

## FAIL Semantics
`silk_de_integrated_interaction` is `FAIL` when begin is present and any is true:
- `[silk.de.integrated.interaction.fail]` present
- any required category missing
- lifecycle corruption/reject markers present:
  - `focus.reject.tombstoned`
  - `lifecycle.tombstone.reject_*`
  - `tombstone.close.reject.dead`
- `faults_zero != PASS`

## PASS Semantics
`silk_de_integrated_interaction` is `PASS` only when:
- begin present
- no integrated fail marker
- no lifecycle corruption/reject markers
- all required categories/dependencies above pass

If an explicit pass marker exists:
- `[silk.de.integrated.interaction.pass] contract=1 topstrip=1 renderer=1 clock=1 pointer=1 focus=1 lifecycle=1 faults=0`
it is accepted only if required evidence also passes.

## Proof Commands
```bash
./scripts/entrypoint_build.sh
./scripts/run_daily_driver_proof.sh /tmp/silk_de_integrated_interaction_v1.log
./scripts/daily_driver_master_gate.sh /tmp/silk_de_integrated_interaction_v1.log | tee /tmp/silk_de_integrated_interaction_v1_gate.txt
rg -n "silk.de.contract|silk.de.topstrip|silk.de.renderer.conformance|silk.de.integrated.interaction|silk_de_integrated_interaction|silk_combined_interaction|clock_visible_seconds|FINAL:|FAIL|#PF|#GP|panic|KERNEL PANIC|fault.kill" /tmp/silk_de_integrated_interaction_v1.log /tmp/silk_de_integrated_interaction_v1_gate.txt
```

## Final Gate Result
- To be filled from the proof run in this mission execution.

## Fault Scan
- Required lane: no `#PF`, `#GP`, `panic`, `KERNEL PANIC`, `fault.kill`.

## Remaining Silk DE 100 Phases
1. Frame Lights explicit proof or current-tier deferral
2. safe glass color polish
3. final Silk DE 100 release handoff/tag
