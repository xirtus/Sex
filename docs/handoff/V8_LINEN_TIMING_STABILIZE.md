# V8_LINEN_TIMING_STABILIZE — Handoff

## Goal
Restore full 43/43 daily-driver V8 proof after 3 Linen timing-SKIP gates recurred.

## Root Cause
```
linen_object_workflow    SKIP
linen_object_persist     SKIP
linen_object_schema      SKIP
```

**Cause**: The V8 `_start` proof ordering had the DiskFS slot proof
(`run_linen_diskfs_slot_proof`) running BEFORE the workflow/persist/schema
proofs.  The diskfs slot proof calls `pdx_storage_sync()` which blocks
waiting for storage replies via `storage_sync_reply()`.  If the SexFiles
storage server hasn't entered its message loop yet, this blocks indefinitely,
starving all subsequent proofs.

The V6 stabilize fix (LINEN_TIMING_SKIP_STABILIZE_V1) only reordered
workflow proofs before the session proof, but didn't account for the diskfs
slot proof which also runs before the workflow proofs and can block.

**Ordering that caused the skip**:
```
_start → diskfs slot proof [BLOCKS on pdx_storage_sync]
       → workflow proof [NEVER REACHED]
       → persist proof [NEVER REACHED]
       → schema proof [NEVER REACHED]
       → session proof [NEVER REACHED]
```

## Fix
Move workflow/persist/schema proofs to the TOP of `_start`, before ALL
storage-dependent proofs (diskfs direct, diskfs slot, session init).

```
_start → workflow proof [3 creates, no storage deps ✓]
       → persist proof [3 fire-and-forget sends, local only ✓]
       → schema proof [taxonomy markers, local only ✓]
       → timing stabilize marker
       → diskfs direct proof [may block — workflow already done]
       → diskfs slot proof [may block — workflow already done]
       → session init [may block — workflow already done]
       → session proof [fills table — workflow objects already created]
```

**Key insight**: Workflow/persist/schema proofs use only local `SESSION.create()`
and `pdx_call()` fire-and-forget — no `pdx_storage_sync()` blocking.  They can
complete before storage is ready.

### Added Marker
```
[linen.timing.stabilize] strategy=v8_move_workflow_before_diskfs ok=1
[linen.timing.stabilize.done] ok=1
```

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/linen/src/main.rs` | Moved workflow/persist/schema before diskfs; added timing marker | +22/-17 |

## Build + Proof Result
- `entrypoint_build.sh` PASS (9s)
- `run_daily_driver_proof.sh` PASS: **43/43 gates, 0 SKIP, 0 faults**

### Before
```
linen_object_workflow    SKIP   no workflow proof markers
linen_object_persist     SKIP   no persist proof markers
linen_object_schema      SKIP   no schema proof markers
```

### After
```
linen_object_workflow    PASS   creates=3 searches=3
linen_object_persist     PASS   persist sends: 3
linen_object_schema      PASS   kinds=3 statuses=4
```

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No new code — pure call reordering + 1 marker
- ✅ Workflow proofs use only local SESSION (no storage deps)
- ✅ Diskfs and session proofs unchanged — still run, after workflow
- ✅ Persist proof now achieves 3 fire-and-forget CREATE_OWNER sends

## Lessons Learned
1. **V6 fix was partial**: it only moved workflow before session_proof, not
   before diskfs proofs.
2. **Diskfs slot proof blocks**: `run_linen_diskfs_slot_proof()` calls
   `storage_sync_reply()` which is `pdx_listen_raw(0)` in a spin loop —
   blocks until storage replies.
3. **Defense in depth**: all non-storage proofs should run before any
   storage-blocking proof.  Principle: "local first, remote later."

## Future Follow-up
- Add `pdx_try_listen_raw()` timeout variant to prevent indefinite block
- Add assertion in `_start` that workflow proof completed before diskfs
- CI gate: detect SKIP gates in daily driver and fail build
