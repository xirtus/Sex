# LINEN_TIMING_SKIP_STABILIZE_V1 — Handoff

## Goal
Stabilize 3 Linen proof gates that intermittently SKIP due to proof ordering:
`linen_object_workflow`, `linen_object_persist`, `linen_object_schema`.

## Root Cause
Proof execution order in `_start`:
```
BEFORE:  linen_init_session() → run_session_proof() [fills 16-slot table]
         → run_linen_object_workflow_proof()  [TABLE FULL — FAIL]
         → run_linen_object_persist_proof()    [no objects from workflow]
         → run_linen_object_schema_proof()     [timing-dependent]
```

`run_session_proof()` stage 7 fills all remaining object table slots in a loop:
```rust
loop {
    match SESSION.create(session::ObjectKind::Document, fill_name, 42) {
        Ok(_) => fill_ok += 1;
        Err(e) => break;  // table full
    }
}
```

After `linen_init_session()` (5 objects) + `run_session_proof()` (fills to 16),
the object workflow proof has zero slots available to create its 3 test objects.

## Fix
Reorder proofs so workflow/persist/schema run BEFORE the table-filling session proof:
```
AFTER:   linen_init_session() [5 objects]
         → run_linen_object_workflow_proof()  [3 objects, slots 5-7]
         → run_linen_object_persist_proof()    [finds 3 workflow objects]
         → run_linen_object_schema_proof()     [emits taxonomy]
         → run_session_proof()                 [fills remaining slots: 8→16]
```

Slot allocation with fix: 5 (init) + 3 (workflow) + 2 (session stage 0+3) + 6 (fill) = 16. ✓

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/linen/src/main.rs` | Reordered proof calls in `_start`: workflow/persist/schema before session proof | +15/-15 |

## Strategy
- **Approach**: `reorder_proof_calls` — move workflow proofs before table-filling proof
- **Safety**: No new code, no new data, no blocking waits.  Pure call reordering.
- **Impact**: Session proof still fills table to 16, but only AFTER workflow proofs have consumed their 3 slots.

## Build + Proof Result
- `entrypoint_build.sh` PASS (8s)
- `run_daily_driver_proof.sh` PASS: **36/36 gates, 0 SKIP, 0 faults**

### Before (V6 baseline)
```
linen_object_workflow    SKIP   no workflow proof markers
linen_object_persist     SKIP   no persist proof markers
linen_object_schema      SKIP   no schema proof markers
```

### After (stabilize V1)
```
linen_object_workflow    PASS   creates=3 searches=3
linen_object_persist     PASS   persist sends: 3
linen_object_schema      PASS   kinds=3 statuses=4
```

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No new code paths — pure reordering of existing calls
- ❌ No blocking waits added
- ✅ Session proof still runs to completion (fills remaining slots)
- ✅ All existing proof stages unchanged
- ✅ Persist proof now successfully sends 3 fire-and-forget CREATE_OWNER calls (prev: 0)
- ✅ Schema proof consistently emits taxonomy

## Known Limitations
- Metadata bridge proof and disk object proof still run after session proof (unchanged,
  not enabled in daily driver proof)
- If additional proofs are added that consume table slots, ordering must be re-audited
- Session proof still fills table to 16 — no slot reservation mechanism

## Future Follow-up
- Add slot reservation API: `SESSION.reserve(n)` to guarantee N slots for a proof
- Assertion in session proof that table count matches expected value
- CI gate that detects SKIP gates and fails if not explicitly classified as safe
