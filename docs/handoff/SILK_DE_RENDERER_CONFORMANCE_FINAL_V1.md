# SILK_DE_RENDERER_CONFORMANCE_FINAL_V1

## Baseline
- Baseline commit target: `3d20cc16` (`silk: add deterministic top strip proof`)
- Verified recent chain includes:
  - `3d20cc16` silk: add deterministic top strip proof
  - `d0b1296e` silk: lock Silk DE bar contract

## Files Changed
- `servers/sexdisplay/src/main.rs`
- `scripts/daily_driver_master_gate.sh`
- `docs/handoff/SILK_DE_RENDERER_CONFORMANCE_FINAL_V1.md`

## Audit Summary

### Allowed Renderer Responsibilities (confirmed)
- sexdisplay consumes validated model state from `silkbar-model` and `OP_SILKBAR_UPDATE`.
- sexdisplay performs bounded framebuffer writes for top strip, chips, clock, frame chrome visuals.
- deterministic top-strip proof path remains renderer-only and emits proof markers.

### Forbidden Policy Searched
- Searched for explicit renderer policy ownership drift markers in target files (focus/drag/resize/snap/close/minimize/zoom/session/input ownership marker strings).
- No newly introduced explicit policy-ownership marker patterns found.

### Bounded Write Audit
- `clamp_surface()` used for surface geometry clamping.
- framebuffer writes consistently guarded with per-pixel checks (`idx < total_pixels`) and width/height guards (`px >= w`, `py >= h`).
- framebuffer range guard present before full render path (`checked_add` guard on FB range).
- deterministic top-strip proof writes are bounded to fixed `PROOF_W * PROOF_H` with explicit index guard.

### ABI/Layout/Theme Drift
- Contract constants remain sourced from `silkbar-model` in sexdisplay/silkbar paths.
- No ABI/protocol edits applied.
- No kernel edits applied.

## Conformance Markers
- begin:
  - `[silk.de.renderer.conformance.begin]`
- pass:
  - `[silk.de.renderer.conformance.pass] model=1 renderer_only=1 bounds=1 policy=0 drift=0`
- fail:
  - `[silk.de.renderer.conformance.fail] reason=...`

## Gate
- Gate name: `silk_de_renderer_conformance`
- PASS requires:
  - conformance pass marker
  - renderer contract pass marker
  - topstrip deterministic pass marker
  - no conformance fail marker
  - no silkbar/sexdisplay faults
- FAIL on:
  - any conformance fail marker
  - contract/topstrip fail markers
  - silkbar/sexdisplay faults
  - missing required pass markers after explicit begin
- SKIP when:
  - conformance begin marker absent

## Proof Commands
```bash
./scripts/entrypoint_build.sh
./scripts/run_daily_driver_proof.sh /tmp/silk_de_renderer_conformance_final_v1.log
./scripts/daily_driver_master_gate.sh /tmp/silk_de_renderer_conformance_final_v1.log | tee /tmp/silk_de_renderer_conformance_final_v1_gate.txt
rg -n "silk.de.contract|silk.de.topstrip|silk.de.renderer.conformance|silk_de_renderer_conformance|FINAL:|FAIL|#PF|#GP|panic|KERNEL PANIC|fault.kill" \
  /tmp/silk_de_renderer_conformance_final_v1.log \
  /tmp/silk_de_renderer_conformance_final_v1_gate.txt
```

## Final Gate Result
- Pending runtime execution in this phase; see generated `/tmp` logs for final PASS/FAIL.

## Fault Scan
- Pending runtime execution in this phase; must be zero for final PASS.

## Remaining Silk DE 100 Phases
1. integrated interaction scenario proof
2. Frame Lights explicit proof or current-tier deferral
3. safe glass color polish
4. final Silk DE 100 release handoff/tag
