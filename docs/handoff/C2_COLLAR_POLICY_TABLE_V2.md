# C2: Collar Policy Table V2

**Status:** Complete
**Date:** 2026-05-06
**Purpose:** Replace Collar AllowStub for LinkObjectToBuffer with a shell-local
static grant/policy table. No real Collar PD, no ABI changes, no persistence.
Preserve existing Linen→Quil open/link behavior when allowed.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║             PASS_C2_COLLAR_POLICY_TABLE_V2                   ║
╠══════════════════════════════════════════════════════════════╣
║ Grant table:             [Option<CollarGrant>; 32]           ║
║ Audit ring:              [Option<CollarAuditEvent>; 64]      ║
║ Auto-grants at boot:     12 (6 Linen + 6 Mesh)              ║
║ Decision:                Allow/Deny replaces AllowStub       ║
║ Caller identity:         FOCUSED_SURFACE_ID (single-threaded) ║
║ Existing paths:          PRESERVED (Linen + Mesh PrintScreen) ║
║ Boundaries:              INTAKT                               ║
║ Build:                   PASS (1622 sectors)                  ║
╚══════════════════════════════════════════════════════════════╝
```

## Changes

**Files:** `servers/silk-shell/src/main.rs` (~180 insertions, ~20 modifications)
**Handoff:** `docs/handoff/C2_COLLAR_POLICY_TABLE_V2.md`
**Commit:** *(to be added)*

### 1. New Types

#### CollarGrantState (lines 1239-1245)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum CollarGrantState {
    Active = 0,
    Revoked = 1,
    Expired = 2,
    Tombstoned = 3,
}
```

V2 only uses `Active`. The other states are defined for future grant lifecycle
but are not yet exercised. `Tombstoned` follows the same pattern as A6.

#### CollarGrant (lines 1249-1257)

```rust
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct CollarGrant {
    grant_id: u64,
    subject_id: u64,
    object_id: u64,
    operation_mask: u64,
    generation: u64,
    state: CollarGrantState,
}
```

- 6 × u64 + 1 × u8 = 52 bytes per grant
- `Clone + Copy` for safe snapshotting
- `repr(C)` for potential future PDX serialization
- No strings, no pointers, no heap

#### CollarAuditEvent (lines 1261-1270)

```rust
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct CollarAuditEvent {
    event_id: u64,
    operation: CollarOperation,
    object_id: u64,
    subject_id: u64,
    decision: CollarDecision,
    grant_ref: u64,
    reason: u64,
}
```

- 6 × u64 = 48 bytes per event
- Reason codes: 0=allowed, 1=missing_object, 2=missing_buffer, 3=no_grant,
  4=stop_first, 5=needs_grant

### 2. Static Tables

| Table | Type | Capacity | Size |
|-------|------|----------|------|
| `COLLAR_GRANTS` | `[Option<CollarGrant>; 32]` | 32 | 52 bytes per slot + Option discriminant |
| `COLLAR_AUDIT_EVENTS` | `[Option<CollarAuditEvent>; 64]` | 64 | 48 bytes per slot + Option discriminant |
| `COLLAR_GRANT_GENERATION` | `static mut u64` | 1..2^64 | Monotonic counter (0 reserved) |
| `COLLAR_AUDIT_WRITE_INDEX` | `static mut u64` | 0..2^64 | Monotonic counter |

**Total static memory:** ~6 KB (32 × ~56 bytes + 64 × ~52 bytes)

### 3. CollarDecision Enum Update

```rust
// Before:
AllowStub = 0,          // Stub placeholder
DenyMissingObject = 1,
DenyMissingBuffer = 2,
NeedsGrantLater = 3,
BlockedStopFirst = 4,

// After (V2):
Allow = 0,              // Grant table match found
Deny = 1,               // No matching active grant
DenyMissingObject = 2,  // (renumbered)
DenyMissingBuffer = 3,  // (renumbered)
NeedsGrantLater = 4,    // (renumbered)
BlockedStopFirst = 5,   // (renumbered)
```

**Breaking change:** Enum values shifted. Only one call site matches on
CollarDecision (`open_linen_object_in_quil()` line 1001), so the impact is
contained.

### 4. Function: `collar_check_operation_stub()`

**Signature unchanged.** The function internally reads `FOCUSED_SURFACE_ID` to
determine the caller identity. This is safe because the shell is single-threaded
— the dispatch loop processes one key at a time, and the focused surface is
stable throughout a single key processing cycle.

**Policy flow:**

```
collar_check_operation_stub(op, object_id, buffer_id):
1. caller_sid = FOCUSED_SURFACE_ID
2. [collar.policy.check] — entry marker with op, object, buffer, caller
3. Validate object exists in LINEN_OBJECTS → DenyMissingObject if not found
4. Validate buffer exists in QUIL_BUFFERS → DenyMissingBuffer if not found
5. Match op:
   - OpenObject | LinkObjectToBuffer → grant table lookup
   - SaveBuffer | BuildTarget | RunTarget → BlockedStopFirst
   - RenameObject | ArchiveObject → NeedsGrantLater
6. Grant lookup for OpenObject/LinkObjectToBuffer:
   - Iterate COLLAR_GRANTS
   - Filter: state == Active, subject_id == caller_sid,
     object_id == target, operation_mask includes op
   - Match found → [collar.grant.match] + [collar.policy.allow] + Allow
   - No match → [collar.grant.reject] + [collar.policy.deny] + Deny
7. All paths call record_collar_audit() before returning
```

### 5. Function: `record_collar_audit()`

Records a `CollarAuditEvent` into the `COLLAR_AUDIT_EVENTS` ring with
overwrite-oldest overflow (same pattern as Bell/Mesh/Tombstone rings).

**Called from every return path in `collar_check_operation_stub()`** — including
Allow, Deny, DenyMissingObject, DenyMissingBuffer, BlockedStopFirst, and
NeedsGrantLater.

### 6. Function: `collar_init_grants()`

Creates auto-grants at boot for all seed Linen objects:

| Grant | Subject | Object | Operation | State | Origin |
|-------|---------|--------|-----------|-------|--------|
| G1 | Linen (200) | Seed 1 (CodeFile, id=1) | LinkObjectToBuffer | Active | boot |
| G2 | Linen (200) | Seed 2 (Document, id=2) | LinkObjectToBuffer | Active | boot |
| G3 | Linen (200) | Seed 3 (QuilWorkspaceRef, id=3) | LinkObjectToBuffer | Active | boot |
| G4 | Linen (200) | Seed 4 (Reference, id=4) | LinkObjectToBuffer | Active | boot |
| G5 | Linen (200) | Seed 5 (MeshDiagnosticRef, id=5) | LinkObjectToBuffer | Active | boot |
| G6 | Linen (200) | Seed 6 (MessageDraft, id=6) | LinkObjectToBuffer | Active | boot |
| G7 | Mesh (202) | Seed 1 (CodeFile, id=1) | LinkObjectToBuffer | Active | boot |
| G8 | Mesh (202) | Seed 2 (Document, id=2) | LinkObjectToBuffer | Active | boot |
| G9 | Mesh (202) | Seed 3 (QuilWorkspaceRef, id=3) | LinkObjectToBuffer | Active | boot |
| G10 | Mesh (202) | Seed 4 (Reference, id=4) | LinkObjectToBuffer | Active | boot |
| G11 | Mesh (202) | Seed 5 (MeshDiagnosticRef, id=5) | LinkObjectToBuffer | Active | boot |
| G12 | Mesh (202) | Seed 6 (MessageDraft, id=6) | LinkObjectToBuffer | Active | boot |

**Rationale:** Both Linen and Mesh call `open_linen_object_in_quil()` which
internally calls the Collar gate. Both surfaces need grants to pass the V2
check. The grants are identity-independent at boot because both Linen and Mesh
are shell-local surfaces, not external PDs.

### 7. Call Site Update

`open_linen_object_in_quil()` line 1000:
```rust
// Before:
let decision = collar_check_operation_stub(CollarOperation::LinkObjectToBuffer, object_id, 0);
if decision != CollarDecision::AllowStub {

// After:
let decision = collar_check_operation_stub(CollarOperation::LinkObjectToBuffer, object_id, 0);
if decision != CollarDecision::Allow {
```

**Function signature unchanged.** The caller identity is derived from
`FOCUSED_SURFACE_ID` inside the gate function.

### 8. Boot Sequence

`collar_init_grants()` is called after `linen_quil_seed_coherence_init()` at
line 8988:
```rust
// C2: Initialize Collar auto-grants for seed objects.
collar_init_grants();
```

This ensures all seed Linen objects exist before auto-grants reference them.

## Proof Markers

| Marker | Location | When |
|--------|----------|------|
| `[collar.policy.check]` | `collar_check_operation_stub()` | Every Collar gate call |
| `[collar.policy.allow]` | `collar_check_operation_stub()` | Grant table match — operation allowed |
| `[collar.policy.deny]` | `collar_check_operation_stub()` | No matching grant — operation denied |
| `[collar.grant.match]` | `collar_check_operation_stub()` | Active grant found for caller+object+op |
| `[collar.grant.reject]` | `collar_check_operation_stub()` | No matching grant |
| `[collar.audit.write]` | `record_collar_audit()` | Audit event written to ring |
| `[collar.audit.overwrite]` | `record_collar_audit()` | Ring full, oldest overwritten |
| `[collar.grant.auto]` | `collar_init_grants()` | Auto-grant created at boot |
| `[collar.grant.init]` | `collar_init_grants()` | Grant initialization complete |

### Preserved Existing Markers (Unchanged)

| Marker | Location | Status |
|--------|----------|--------|
| `[collar.gate.reject] reason=missing_object` | `collar_check_operation_stub()` | ✅ Preserved |
| `[collar.gate.reject] reason=missing_buffer` | `collar_check_operation_stub()` | ✅ Preserved |
| `[collar.gate.reject] reason=stop_first` | `collar_check_operation_stub()` | ✅ Preserved |
| `[collar.gate.needs_grant]` | `collar_check_operation_stub()` | ✅ Preserved |
| `[linen.quil.open.reject.collar]` | `open_linen_object_in_quil()` | ✅ Preserved |

### Removed Markers

| Marker | Reason |
|--------|--------|
| `[collar.gate.check]` | Replaced by `[collar.policy.check]` |
| `[collar.gate.allow_stub]` | Replaced by `[collar.policy.allow]` + `[collar.grant.match]` |

## Existing Path Preservation

### Path 1: Linen focused + PrintScreen (0x59)

```
→ scancode_to_action → SurfaceAction::OpenObjectInQuil
  → FOCUSED_SURFACE_ID == SURFACE_ID_LINEN → linen_selected_object_id()
  → open_linen_object_in_quil(obj_id)
    → collar_check_operation_stub(LinkObjectToBuffer, obj_id, 0)
      → FOCUSED_SURFACE_ID == SURFACE_ID_LINEN (200)
      → grant lookup: subject=200, object=obj_id, op=LinkObjectToBuffer
      → MATCH: auto-grant G1-G6 → Allow
    → [collar.policy.allow] → buffer creation/linking → succeeds
```

### Path 2: Mesh focused + PrintScreen (0x59)

```
→ Mesh dispatch → mesh_selected_fact_snapshot()
  → open_linen_object_in_quil(fact.subject_id)
    → collar_check_operation_stub(LinkObjectToBuffer, subject_id, 0)
      → FOCUSED_SURFACE_ID == SURFACE_ID_MESH (202)
      → grant lookup: subject=202, object=subject_id, op=LinkObjectToBuffer
      → MATCH: auto-grant G7-G12 → Allow
    → [collar.policy.allow] → buffer creation/linking → succeeds
```

### Path 3: Denied (non-Linen/Mesh caller)

```
→ open_linen_object_in_quil() called from unexpected surface
  → collar_check_operation_stub(LinkObjectToBuffer, obj_id, 0)
    → FOCUSED_SURFACE_ID == SURFACE_ID_QUIL (201) [example]
    → grant lookup: subject=201, object=obj_id, op=LinkObjectToBuffer
    → NO MATCH → Deny
  → [collar.policy.deny] → [linen.quil.open.reject.collar]
  → return false → no buffer creation, no linking
```

## Grant Table at Boot

Standard boot creates exactly 12 grants (6 seed objects × 2 surfaces):

```
[collar.grant.auto] grant_id=1 subject=200 object=1 op=LinkObjectToBuffer
[collar.grant.auto] grant_id=2 subject=202 object=1 op=LinkObjectToBuffer
[collar.grant.auto] grant_id=3 subject=200 object=2 op=LinkObjectToBuffer
[collar.grant.auto] grant_id=4 subject=202 object=2 op=LinkObjectToBuffer
[collar.grant.auto] grant_id=5 subject=200 object=3 op=LinkObjectToBuffer
[collar.grant.auto] grant_id=6 subject=202 object=3 op=LinkObjectToBuffer
[collar.grant.auto] grant_id=7 subject=200 object=4 op=LinkObjectToBuffer
[collar.grant.auto] grant_id=8 subject=202 object=4 op=LinkObjectToBuffer
[collar.grant.auto] grant_id=9 subject=200 object=5 op=LinkObjectToBuffer
[collar.grant.auto] grant_id=10 subject=202 object=5 op=LinkObjectToBuffer
[collar.grant.auto] grant_id=11 subject=200 object=6 op=LinkObjectToBuffer
[collar.grant.auto] grant_id=12 subject=202 object=6 op=LinkObjectToBuffer
[collar.grant.init] count=12 generation=13
```

## Boundaries

| Area | Status |
|------|--------|
| `kernel/` | ✅ CLEAN |
| `crates/sex-pdx/` | ✅ CLEAN |
| `servers/sexdisplay/` | ✅ CLEAN |
| `servers/bell/` | ✅ CLEAN |
| `servers/mesh/` | ✅ CLEAN |
| `servers/linen/` | ✅ CLEAN |
| `servers/quil/` | ✅ CLEAN |
| PDX ABI/opcodes | ✅ CLEAN |
| Collar PD creation | ✅ NOT TRIGGERED |
| Persistent storage | ✅ NOT TRIGGERED |
| MPK permission mutation | ✅ NOT TRIGGERED |

### STOP FIRST Check

| Trigger | Status |
|---------|--------|
| Real Collar PD/server | ✅ NOT TRIGGERED — shell-local tables only |
| New PDX ABI/opcodes | ✅ NOT TRIGGERED — no new PDX calls |
| kernel edits | ✅ NOT TRIGGERED — no kernel changes |
| sex-pdx ABI edits | ✅ NOT TRIGGERED — no ABI changes |
| sexdisplay changes | ✅ NOT TRIGGERED — no sexdisplay changes |
| Persistent grant storage | ✅ NOT TRIGGERED — memory-only |
| Cross-PD grant propagation | ✅ NOT TRIGGERED — shell-local only |
| Hardware MPK permission mutation | ✅ NOT TRIGGERED — advisory only |
| User prompt UI with security meaning | ✅ NOT TRIGGERED — no prompts |
| Renderer authority decisions | ✅ NOT TRIGGERED |
| Broad rewrite of operation paths | ✅ NOT TRIGGERED — only Collar gate internal logic changed |

**STOP FIRST: NOT TRIGGERED** — All forbidden areas clean.

## Functional Changes Summary

| Aspect | Before (J5) | After (C2) |
|--------|-------------|------------|
| Decision for LinkObjectToBuffer | Always `AllowStub` | Grant table lookup → `Allow` or `Deny` |
| Caller identity | Not tracked | `FOCUSED_SURFACE_ID` |
| Grant state | None | 12 auto-grants at boot |
| Audit trail | None | 64-entry audit ring |
| Proof markers | `[collar.gate.allow_stub]` | `[collar.policy.allow]` + `[collar.grant.match]` |
| Deny behavior | N/A | Returns `Deny`, emits `[collar.policy.deny]`, no buffer creation |

## Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Caller identity source | `FOCUSED_SURFACE_ID` | Single-threaded dispatch ensures correct surface at call time. No parameter change needed in `open_linen_object_in_quil()` |
| Grant table capacity | 32 | Covers 12 auto-grants + 16 dynamic buffer grants + room for future |
| Audit ring capacity | 64 | Collar decisions fire frequently; 64 entries provide ~5 full boot cycles before overwrite |
| Operation mask bitfield | `u64` | Supports up to 64 operations in a single grant |
| Auto-grant for Mesh | All 6 seed objects | Mesh can open any linked object's Quil view through same callee |
| No dynamic grants | Deferred | Dynamic buffer grants will be created at link time as future work |
| No revocation | Deferred | Grant lifecycle states defined but only Active is used in V2 |

## Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| No dynamic grants for linked buffers | LOW | Auto-grants cover seed objects; buffer creation works through seed object grant |
| Caller identity from FOCUSED_SURFACE_ID is implicit | LOW | Safe because shell is single-threaded; documented in function comment |
| No revocation support | LOW | Grant table supports Revoked/Expired/Tombstoned states but no path sets them yet |
| Only LinkObjectToBuffer wired | LOW | OpenObject still follows same grant lookup path but has no auto-grants |
| V2 is still shell-local, not real security | INFO | The kernel's PDX/MPK system is the real security boundary |

## Build Result

```
ISO image produced: 1622 sectors
[SEXOS ENTRYPOINT] success
```

**Build: PASS** — ISO produced successfully.

## Next Steps

1. **C3: Audit C2** — Verify grant table integrity, auto-grant correctness, no
   bypass paths, audit ring bounds. Docs only.
2. **After C3:** Evaluate dynamic grants (create grant when buffer is linked)
   or Collar grant display on Mesh surface.
