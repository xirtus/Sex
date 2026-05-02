# SILK DE Execution Plan

## Objective
Ship a stable Silk DE top-strip stack where `silkbar` produces contract-valid updates, `sexdisplay` renders strictly from shared model state, and regressions are blocked before runtime.

## Success Criteria
1. ABI/layout/theme contract is explicit and versioned in `crates/silkbar-model`.
2. `silkbar` and `sexdisplay` both enforce contract gates at startup.
3. A deterministic render verification path exists for top-strip pixels.
4. Build/run gate catches ABI mismatch and missing required modules/chips.
5. Clock + chips remain live for sustained runtime without renderer fault.

## Phase Plan

### Phase 1: Contract Lock (in progress)
1. Add `SILK_DE_BAR_ABI_V1` contract constants and validation API in `silkbar-model`.
2. Freeze module slot/chip slot naming in model so all producers/consumers use the same index semantics.
3. Add runtime contract checks in both `silkbar` and `sexdisplay` startup paths.

### Phase 2: Renderer Conformance
1. Remove remaining local render assumptions in `sexdisplay` that bypass model semantics.
2. Ensure redraw paths only use model-derived state/tokens for top strip.
3. Add defensive bounds checks in all top-strip writes (already partly done).

### Phase 3: Deterministic Verification
1. Add a fixed test vector of `SilkBarUpdate` messages.
2. Render top-strip buffer in a deterministic test mode.
3. Hash and compare output against golden value in CI/build gate.

### Phase 4: Silk DE Visual Polish
1. Apply glass-language token set through model/theme fields only.
2. Add controlled animation cadence (no flood) from `silkbar` producer.
3. Validate focus/workspace/chip transitions under load.

## Parallel Work Assignment

### Codex (local, now)
1. Implement contract constants + gate functions in `silkbar-model`.
2. Wire contract checks into `servers/silkbar/src/main.rs` and `servers/sexdisplay/src/main.rs`.
3. Add minimal deterministic contract self-check command in build flow.

### Gemini (best for broad system cross-check)
1. Audit all places where module/chip indices are assumed implicitly.
2. Produce a mismatch report: expected-by-model vs used-in-code.
3. Propose smallest patch set to eliminate index drift.

### Claude (best for deep code consistency + invariants)
1. Audit update ABI handling (`UpdateKind`, packing/unpacking, queue semantics).
2. Verify no silent lossy transform between producer and renderer.
3. Draft strict invariants and compile/runtime assertions for queue + update decode.

### DeepSeekClaude (best for deterministic test design and edge-case matrix)
1. Design deterministic top-strip verification harness strategy.
2. Define stable test vectors covering workspace/chip/clock/theme transitions.
3. Provide exact expected hash workflow and failure diagnostics format.

## Prompt: DeepSeekClaude
Use this exact prompt:

```
You are working in /home/xirtus_arch/Documents/microkernel.
Task: design a deterministic verification harness for Silk DE top-strip rendering.

Context:
- Shared contract lives in crates/silkbar-model/src/lib.rs.
- Producer: servers/silkbar/src/main.rs.
- Consumer/renderer: servers/sexdisplay/src/main.rs.
- Goal: catch ABI/layout/render regressions before runtime boot.

Deliverables:
1. A concrete test architecture that does NOT require GUI/QEMU window.
2. A deterministic input vector set of SilkBarUpdate messages that covers:
   - workspace active + urgent transitions
   - chip visible + kind transitions
   - clock transitions (including rollover boundaries)
   - theme token transition behavior
3. A golden-output strategy:
   - buffer dimensions for top-strip test
   - byte/pixel hashing algorithm
   - exact failure report format showing first mismatch coordinates.
4. A minimal patch plan with file paths and function entry points.

Constraints:
- No broad refactor.
- Keep compatibility with existing ABI and model types.
- Prefer additive changes and bounded loops.
- Any unsafe write path must be explicitly bounds-checked.

Return format:
- Section A: test architecture
- Section B: vector table
- Section C: hash+diff format
- Section D: minimal patch list by file path
```

## Prompt: Gemini
```
Audit index/slot contract drift for Silk DE.
Compare model constants/enums in crates/silkbar-model/src/lib.rs against all read/write usage in:
- servers/silkbar/src/main.rs
- servers/sexdisplay/src/main.rs
- any other consumer/producer touching OP_SILKBAR_UPDATE.
Output only:
1) mismatch list,
2) risk level per mismatch,
3) smallest safe patch list.
No refactor.
```

## Prompt: Claude
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

## Immediate Next Actions
1. Finish wiring `validate_contract()` into silkbar + sexdisplay startup.
2. Add build-gate check that fails on contract mismatch.
3. Run `./scripts/entrypoint_build.sh` and capture result in handoff notes.
