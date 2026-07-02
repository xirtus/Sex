# SexOS Silk DE Roadmap (Current)

## Scope
This roadmap reflects the active, in-repo plan for finishing Silk DE on current `master`.
It supersedes older COSMIC-port phase text.

Primary references:
- `docs/SILK_DE_EXECUTION_PLAN.md`
- `HANDOFF.md`
- `docs/SILK_DE_GLASS_VISUAL_LANGUAGE.md`

## Current State (2026-05-02)
- Contract gates added and live in producer/renderer startup.
- Top-strip slot geometry in `sexdisplay` now derives from `silkbar-model` layout slots.
- Clock-freeze-at-2s regression fixed via strict framebuffer bounds checks.
- Build pipeline authority is `./scripts/entrypoint_build.sh`.

## Milestones

### M1: Contract Lock (Done)
1. Versioned Silk DE contract in `crates/silkbar-model`.
2. Shared slot semantics (`ModuleSlot`, chip/module counts, update size/cap invariants).
3. Startup-time contract validation in:
   - `servers/silkbar/src/main.rs`
   - `servers/sexdisplay/src/main.rs`

### M2: Renderer Conformance (In Progress)
1. Remove local renderer assumptions that bypass model semantics.
2. Keep top-strip rendering model-driven (layout + state).
3. Preserve strict bounds checks on all framebuffer writes.

### M3: Deterministic Verification (Next)
1. Deterministic top-strip render harness (headless, no GUI/QEMU dependency).
2. Fixed update vectors (workspace/chip/clock/theme transitions).
3. Golden hash compare + first-mismatch diagnostics.
4. Build-gate integration to fail fast on contract/render drift.

### M4: Visual Polish + Interaction Stability
1. Apply glass visual language via shared tokens only.
2. Controlled animation cadence without event floods.
3. Validate focus/workspace/chip transitions under sustained runtime.

## Ownership / Delegation
- Codex: integrate contract, conformance patches, build-gate wiring.
- Claude: ABI correctness/invariant audit and assertion recommendations.
- DeepSeekClaude: deterministic verification harness architecture + vectors.
- Gemini: cross-file slot/index drift audit and minimal mismatch patch list.

## Immediate Next Actions
1. Implement M3 deterministic harness skeleton.
2. Prompt Claude for ABI audit now (see prompt below).
3. Integrate actionable Claude findings as additive assertions.

## Prompt For Claude (Use As-Is)
```
Audit update ABI correctness for SilkBar pipeline.
Scope:
- SilkBarUpdate packing/unpacking
- UpdateKind discriminant handling
- queue push/pop/drain semantics
- renderer apply path.
Find silent data-loss or drift risks and propose strict invariants/assertions.
Return concrete patch-ready recommendations with exact file/line targets.
No refactor.
```
