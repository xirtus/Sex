# C3: Audit Collar Policy Table V2

**Status:** Complete
**Date:** 2026-05-06
**Purpose:** Verify C2 shell-local Collar policy table is correct — grant table
integrity, caller identity, deny-before-mutation, no accidental regressions.
Docs only. No code changes.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║           PASS_C2_COLLAR_POLICY_TABLE                         ║
╠══════════════════════════════════════════════════════════════╣
║ Grant table:              PASS (static, 12 auto-grants)       ║
║ Caller identity:          PASS (FOCUSED_SURFACE_ID safe)      ║
║ Deny-before-mutation:     PASS (all mutation after gate)      ║
║ Existing paths:           PRESERVED (Linen + Mesh)             ║
║ Audit ring:               PASS (deterministic, bounded)       ║
║ Boundaries:               INTAKT                               ║
║ Build:                    PASS (1622 sectors)                  ║
╚══════════════════════════════════════════════════════════════╝
```

## Source Documents

| Document | Reference |
|----------|-----------|
| `docs/handoff/C2_COLLAR_POLICY_TABLE_V2.md` | Implementation handoff |
| `docs/handoff/C1_COLLAR_REAL_AUTHORITY_MODEL_DESIGN_V1.md` | Authority model design |
| `docs/handoff/N15_AUDIT_MESH_OPEN_LINKED_OBJECT_V1.md` | Prior audit (verification patterns) |
| `docs/handoff/J4_LINEN_OBJECT_TO_QUIL_BUFFER_LINK_V1.md` | J4 chain (caller path) |
| `docs/handoff/J5_COLLAR_GATED_OPERATION_STUBS_V1.md` | Pre-V2 stub baseline |
| `servers/silk-shell/src/main.rs` | Implementation (lines 1095-1351, 9144-9148) |

## 1. Grant Table Integrity

### Structure Verification

| Property | Implementation | Source Lines | Status |
|----------|---------------|-------------|--------|
| Static fixed-size | `[Option<CollarGrant>; COLLAR_GRANT_CAP]` | 1274 | ✅ PASS |
| Capacity | 32 | 1273 (`const COLLAR_GRANT_CAP: usize = 32`) | ✅ PASS |
| No heap allocation | `static mut` array | 1274 | ✅ PASS |
| No strings | All fields are u64 or enum repr(u8) | 1249-1257 | ✅ PASS |
| No pointers | No raw/smart pointer fields | 1249-1257 | ✅ PASS |
| Clone + Copy | `#[derive(Debug, Clone, Copy)]` | 1247-1248 | ✅ PASS |
| repr(C) | `#[repr(C)]` | 1248 | ✅ PASS |

### Grant Record Fields

| Field | Type | Purpose |
|-------|------|---------|
| `grant_id` | u64 | Monotonic grant identifier |
| `subject_id` | u64 | Surface ID of grant holder |
| `object_id` | u64 | Linen object ID the grant applies to |
| `operation_mask` | u64 | Bitmask of allowed CollarOperation values |
| `generation` | u64 | Monotonic counter for stale-grant detection |
| `state` | CollarGrantState | Active, Revoked, Expired, or Tombstoned |

**All fields are IDs (never pointers, never strings).**

### State Handling

| State | V2 Behavior | Correct? |
|-------|-------------|----------|
| `Active` | Allows operation if subject/object/operation_mask match | ✅ Correct |
| `Revoked` | Skipped by filter: `if grant.state != CollarGrantState::Active { continue; }` | ✅ Correct |
| `Expired` | Same — skipped by filter | ✅ Correct |
| `Tombstoned` | Same — skipped by filter | ✅ Correct |

**Only `Active` grants are considered. All non-Active states are implicitly denied.**

### Auto-Grant Count and Values

12 deterministic auto-grants created at boot via `collar_init_grants()` (line 1310):

| Grant | Field Values | Source Verification |
|-------|-------------|-------------------|
| G1 | grant_id=1, subject=200, object=1, op=LinkObjectToBuffer, state=Active | Line 1318-1328 |
| G2 | grant_id=2, subject=202, object=1, op=LinkObjectToBuffer, state=Active | Line 1335-1345 |
| G3 | grant_id=3, subject=200, object=2, op=LinkObjectToBuffer, state=Active | Line 1318-1328 (seed 2) |
| G4 | grant_id=4, subject=202, object=2, op=LinkObjectToBuffer, state=Active | Line 1335-1345 (seed 2) |
| G5 | grant_id=5, subject=200, object=3, op=LinkObjectToBuffer, state=Active | Line 1318-1328 (seed 3) |
| G6 | grant_id=6, subject=202, object=3, op=LinkObjectToBuffer, state=Active | Line 1335-1345 (seed 3) |
| G7 | grant_id=7, subject=200, object=4, op=LinkObjectToBuffer, state=Active | Line 1318-1328 (seed 4) |
| G8 | grant_id=8, subject=202, object=4, op=LinkObjectToBuffer, state=Active | Line 1335-1345 (seed 4) |
| G9 | grant_id=9, subject=200, object=5, op=LinkObjectToBuffer, state=Active | Line 1318-1328 (seed 5) |
| G10 | grant_id=10, subject=202, object=5, op=LinkObjectToBuffer, state=Active | Line 1335-1345 (seed 5) |
| G11 | grant_id=11, subject=200, object=6, op=LinkObjectToBuffer, state=Active | Line 1318-1328 (seed 6) |
| G12 | grant_id=12, subject=202, object=6, op=LinkObjectToBuffer, state=Active | Line 1335-1345 (seed 6) |

**Verified:** 12 auto-grants exactly match C2 doc claim. Grant IDs 1-12, generation
counter reaches 13 after init. All 6 LINEN_OBJECTS seed objects get grants for
both SURFACE_ID_LINEN (200) and SURFACE_ID_MESH (202).

### Index Calculation Correctness

```
gen = COLLAR_GRANT_GENERATION;          // starts at 1
COLLAR_GRANT_GENERATION += 1;           // now 2
idx = (gen - 1) % COLLAR_GRANT_CAP;     // (1-1) % 32 = 0
// First grant written to index 0

gen2 = COLLAR_GRANT_GENERATION;         // now 2
COLLAR_GRANT_GENERATION += 1;           // now 3
idx2 = (gen2 - 1) % COLLAR_GRANT_CAP;   // (2-1) % 32 = 1
// Second grant written to index 1
```

**Index calculation: CORRECT** — Grants for seed objects occupy indices 0-11,
leaving indices 12-31 free for dynamic grants. No collision for 6 seed objects.

### Generation / Stale-Grant Risk

Generation IDs are set once at creation and never validated against a per-object
generation counter. The C1 design (section 3) specified per-object generation
counters to detect stale grants, but C2 does not implement this.

**Risk:** If a LinenObject is deleted and recreated with the same object_id, a
pre-existing grant would still match it. However, in V2:
- LINEN_OBJECTS is a static array — objects are never deleted (the table has
  fixed capacity 16, slots are only written at init)
- No object deletion path exists in the current codebase
- Generation mismatch would only matter if object deletion + recreation existed

**Acceptable for V2.** The generation field is present in the struct for future
use but is not yet validated in the grant lookup.

**Grant table: PASS** — All integrity checks pass. Static, no heap, 12
deterministic auto-grants match C2 doc. Generation validation is deferred.

## 2. Caller Identity

### Identity Source

The caller identity is derived from `FOCUSED_SURFACE_ID` (line 1154):

```rust
let caller_sid = FOCUSED_SURFACE_ID;
```

This global variable is read at the time `collar_check_operation_stub()` is
called. The function has no explicit `caller_sid` parameter.

### Caller Identity Table

| Caller Surface ID | Surface Name | Has Auto-Grant? | V2 Decision |
|-------------------|-------------|-----------------|-------------|
| 200 | Linen | ✅ G1-G6 (seed objects 1-6) | Allow |
| 202 | Mesh | ✅ G7-G12 (seed objects 1-6) | Allow |
| 201 | Quil | ❌ No grants for any object | Deny |
| 203 | Collar | ❌ No grants for any object | Deny |
| 204 | Bell | ❌ No grants for any object | Deny |
| 0x98 | Command Palette | ❌ No grants for any object | Deny |
| Any other | Unknown | ❌ No grants | Deny |

### Caller Identity Path Verification

**Linen-focused PrintScreen:**
```
1. Dispatch: FOCUSED_SURFACE_ID == SURFACE_ID_LINEN (200)
2. → SurfaceAction::OpenObjectInQuil → FOCUSED_SURFACE_ID == SURFACE_ID_LINEN check passes
3. → open_linen_object_in_quil(obj_id)
4.   → collar_check_operation_stub(...)
5.     → caller_sid = FOCUSED_SURFACE_ID = 200
6.     → grant lookup: subject=200 → G1-G6 match → Allow
```

**Mesh-focused PrintScreen:**
```
1. Dispatch: FOCUSED_SURFACE_ID == SURFACE_ID_MESH (202)
2. → mesh_selected_fact_snapshot()
3. → open_linen_object_in_quil(fact.subject_id)
4.   → collar_check_operation_stub(...)
5.     → caller_sid = FOCUSED_SURFACE_ID = 202
6.     → grant lookup: subject=202 → G7-G12 match → Allow
```

**Command Palette OpenInQuil:**
```
1. Dispatch: FOCUSED_SURFACE_ID == SURFACE_ID_LINEN (200) [palette checks focus]
2. → palette_execute_selected() → OpenSelectedInQuil
3. → open_linen_object_in_quil(obj_id)
4.   → collar_check_operation_stub(...)
5.     → caller_sid = FOCUSED_SURFACE_ID = 200
6.     → grant lookup: subject=200 → G1-G6 match → Allow
```

### Focus-Spoofing Risk (Documented)

| Risk | Scenario | Impact | Mitigation |
|------|----------|--------|------------|
| Focus spoofing | An attacker PD sends fake keyboard input to change FOCUSED_SURFACE_ID | Would allow unauthorized operations under a different surface's identity | Shell is single-threaded; FOCUSED_SURFACE_ID is only changed by the dispatch loop via `try_set_focus()`. External PDs cannot mutate shell globals. |
| Race condition | FOCUSED_SURFACE_ID changes between gate call and buffer mutation | Would use wrong caller identity for the operation | Impossible in single-threaded dispatch — no preemption between `collar_check_operation_stub()` and buffer mutation at line 1000-1020. |

**Focus spoofing: DOCUMENTED as LOW risk.** The focused surface ID is a
shell-local global that only the dispatch loop can change. External actors
cannot mutate it. The real security boundary is the kernel's PDX slot isolation,
not the Collar grant table.

### What Would Be Required for Real Caller Identity

Real caller identity would require:
1. A real Collar PD with PDX endpoint
2. Caller identity derived from kernel-enforced PD slot number
3. PDX IPC that carries caller identity (kernel-enforced)
4. All operation paths routed through Collar PD, not shell-local function calls

**This is a STOP FIRST trigger** if attempted in current phase.

**Caller identity: PASS** — FOCUSED_SURFACE_ID is correct for single-threaded
shell dispatch. Focus-spoofing risk is documented as acceptable for V2.

## 3. Deny-Before-Mutation

### Critical Invariant

**All mutations occur AFTER the Collar gate check.** The `collar_check_operation_stub()`
call at line 1000 returns before any of these mutation paths:

```
Line 997-1004: Collar gate check
  → Deny → return false (line 1003)
  → Allow → continue to...
Line 1006-1012: Buffer kind mapping (local computation)
Line 1014-1066: Buffer slot find/create (writes QUIL_BUFFERS)
Line 1068-1073: linked_surface_id update (writes LINEN_OBJECTS)
Line 1075-1078: [linen.quil.buffer.linked] proof marker
Line 1081-1084: open_quil_in_active_scene() (creates/focuses Quil surface)
Line 1089: mesh_emit_linen_quil_links() (writes Mesh facts)
Line 1092: bell_emit_object_link_event() (writes Bell events)
Line 1097: quil_render_buffer_list() (sends 0xEC/0xEF to sexdisplay)
```

### Deny-Before-Mutation Table

| Mutation | Lines | After Gate? | Denied Path Reaches? |
|----------|-------|-------------|---------------------|
| Buffer kind mapping | 1006-1012 | ✅ After gate (line 1004) | ❌ No — early return on Deny |
| QUIL_BUFFERS write (create/reuse) | 1014-1066 | ✅ After gate | ❌ No — early return on Deny |
| LINEN_OBJECTS linked_surface_id update | 1068-1073 | ✅ After gate | ❌ No — early return on Deny |
| [linen.quil.buffer.linked] proof marker | 1075-1078 | ✅ After gate | ❌ No — early return on Deny |
| open_quil_in_active_scene() | 1081-1084 | ✅ After gate | ❌ No — early return on Deny |
| mesh_emit_linen_quil_links() | 1089 | ✅ After gate | ❌ No — early return on Deny |
| bell_emit_object_link_event() | 1092 | ✅ After gate | ❌ No — early return on Deny |
| quil_render_buffer_list() | 1097 | ✅ After gate | ❌ No — early return on Deny |

### Deny Code Path (Exact)

```
open_linen_object_in_quil()
  → collar_check_operation_stub(LinkObjectToBuffer, object_id, 0)
    → grant lookup: no match → Deny
    → record_collar_audit(Deny) ← ONLY side effect on deny path
    → return CollarDecision::Deny
  → decision != Allow → true
  → [linen.quil.open.reject.collar] decision=1
  → return false  ← EARLY RETURN, no mutations below
```

**Deny-before-mutation: PASS** — All mutations are strictly after the gate.
Deny path returns at line 1003 before any buffer/object/Mesh/Bell/display
side effects. The only side effect on deny is the audit event recording inside
`collar_check_operation_stub()`.

## 4. Existing Path Preservation

### Path 1: Linen PrintScreen (0x59) → OpenInQuil

| Step | Before (J5) | After (C2) | Change? |
|------|-------------|------------|---------|
| Dispatch gate | `FOCUSED_SURFACE_ID == SURFACE_ID_LINEN` | Same | ❌ No change |
| `linen_selected_object_id()` | Returns selected ID | Same | ❌ No change |
| `open_linen_object_in_quil(obj_id)` | Called | Same | ❌ No change |
| Object validation (step 1) | Rejects missing | Same | ❌ No change |
| Grant ref check (step 2) | `[linen.quil.open.no_grant]` | Same | ❌ No change |
| **Collar gate (step 2.5)** | **AllowStub** | **Grant lookup → Allow** | **✅ Behavior preserved** |
| Buffer kind mapping (step 3) | Same | Same | ❌ No change |
| Buffer create/reuse (step 4) | Same | Same | ❌ No change |
| linked_surface_id (step 5) | Same | Same | ❌ No change |
| Mesh facts (step 8) | Same | Same | ❌ No change |
| Bell event (step 9) | Same | Same | ❌ No change |
| Quil render (step 10) | Same | Same | ❌ No change |

**Result: PRESERVED** — Linen PrintScreen still succeeds for seed objects.

### Path 2: Mesh PrintScreen (0x59) → OpenInQuil

| Step | Before (J5) | After (C2) | Change? |
|------|-------------|------------|---------|
| Dispatch gate | Mesh focused | Same | ❌ No change |
| `mesh_selected_fact_snapshot()` | Returns fact | Same | ❌ No change |
| `open_linen_object_in_quil(fact.subject_id)` | Called | Same | ❌ No change |
| Object validation (step 1) | Rejects missing | Same | ❌ No change |
| **Collar gate (step 2.5)** | **AllowStub** | **Grant lookup → Allow** | **✅ Behavior preserved** |
| Buffer create/reuse (step 4) | Same | Same | ❌ No change |
| Mesh facts (step 8) | Same | Same | ❌ No change |
| Bell event (step 9) | Same | Same | ❌ No change |
| Quil render (step 10) | Same | Same | ❌ No change |

**Result: PRESERVED** — Mesh PrintScreen still succeeds for seed objects.

### Path 3: Enter from Mesh → FocusLinen

This path does NOT call `open_linen_object_in_quil()`. It calls
`mesh_focus_linen_at_selected_fact()` which only calls `open_linen_in_active_scene()`
+ `linen_render_object_list()`. **No Collar gate involved. No behavior change.**

### Path 4: Stale Object Rejection

| Step | Before (J5) | After (C2) | Change? |
|------|-------------|------------|---------|
| `open_linen_object_in_quil(bad_id)` | Called | Same | ❌ No change |
| Object validation (step 1) | `[linen.quil.open.reject.missing]` | Same | ❌ No change |
| **Collar gate (step 2.5)** | Never reached | Never reached | ❌ No change |
| Early return | ✅ No mutations | ✅ No mutations | ❌ No change |

**Result: PRESERVED** — Stale/invalid objects rejected at step 1 before Collar gate.

### Path 5: No Grant (Non-Linen/Mesh Caller)

This is a NEW behavior — previously, any caller got AllowStub. Now non-Linen/Mesh
callers get Deny:

| Scenario | Before (J5) | After (C2) | Correct? |
|----------|-------------|------------|----------|
| Quil-focused PrintScreen | AllowStub → creates buffer | Deny → no buffer | ✅ Correct — Quil should not open Linen objects |
| Collar-focused PrintScreen | AllowStub → creates buffer | Deny → no buffer | ✅ Correct — Collar should not open Linen objects |
| Bell-focused PrintScreen | AllowStub → creates buffer | Deny → no buffer | ✅ Correct — Bell should not open Linen objects |

**This is a security improvement.** Previously, any surface could create Linen→Quil
links. Now only Linen and Mesh (the two surfaces with auto-grants) can.

**Existing path preservation: PASS** — All previously working paths preserved.
Non-Linen/Mesh paths that were previously incorrectly allowed are now correctly
denied.

## 5. Audit Ring

### Structure Verification

| Property | Implementation | Source | Status |
|----------|---------------|--------|--------|
| Static fixed-size | `[Option<CollarAuditEvent>; COLLAR_AUDIT_CAP]` | Line 1277 | ✅ PASS |
| Capacity | 64 | Line 1276 (`const COLLAR_AUDIT_CAP: usize = 64`) | ✅ PASS |
| Overwrite behavior | Overwrite oldest at `idx = write_index % 64` | Line 1289 | ✅ PASS |
| Overflow marker | `[collar.audit.overwrite]` | Line 1299-1300 | ✅ PASS |
| Write marker | `[collar.audit.write]` on every record | Line 1301-1303 | ✅ PASS |

### Every Decision Writes an Audit Event

`record_collar_audit()` is called from every return path in `collar_check_operation_stub()`:

| Return Path | Line | Reason Code | Audit Written? |
|-------------|------|-------------|----------------|
| DenyMissingObject | 1172 | 1 | ✅ Yes |
| DenyMissingBuffer | 1183 | 2 | ✅ Yes |
| Allow (grant match) | 1204 | 0 | ✅ Yes |
| Deny (no grant) | 1213 | 3 | ✅ Yes |
| BlockedStopFirst | 1219 | 4 | ✅ Yes |
| NeedsGrantLater | 1225 | 5 | ✅ Yes |

**Reason codes are deterministic:**
- 0 = allowed
- 1 = missing_object
- 2 = missing_buffer
- 3 = no_grant
- 4 = stop_first
- 5 = needs_grant

### No Panic/Full Path

The ring uses modulo indexing, so there is no "full" state — it always
overwrites the oldest entry. The `wrapping_add` in `collar_init_grants()`
also ensures no panic on overflow.

### Audit Event Record

```rust
struct CollarAuditEvent {
    event_id: u64,       // Monotonic write index
    operation: u8,       // CollarOperation
    object_id: u64,      // Target resource
    subject_id: u64,     // Caller identity
    decision: u8,        // CollarDecision
    grant_ref: u64,      // Grant that authorized (0 if denied)
    reason: u64,         // Policy reason code
}
```

**Audit ring: PASS** — Deterministic, bounded, no panic path, every decision
recorded.

## 6. Boundary Check

| Area | Status | Evidence |
|------|--------|----------|
| `kernel/` | ✅ CLEAN | No kernel changes in C2 commit |
| `crates/sex-pdx/` | ✅ CLEAN | No ABI/opcode changes |
| `servers/sexdisplay/` | ✅ CLEAN | No sexdisplay changes |
| `servers/bell/` | ✅ CLEAN | No Bell code changes |
| `servers/mesh/` | ✅ CLEAN | No Mesh code changes |
| `servers/linen/` | ✅ CLEAN | No linen server changes |
| `servers/quil/` | ✅ CLEAN | No quil server changes |
| PDX ABI/opcodes | ✅ CLEAN | No opcode additions |
| Real Collar PD | ✅ NOT TRIGGERED | Shell-local tables only |
| Persistent storage | ✅ NOT TRIGGERED | Memory-only grants |
| MPK permission mutation | ✅ NOT TRIGGERED | Advisory only |
| Grant UI | ✅ NOT TRIGGERED | No prompts or UI |

### STOP FIRST Triggers

| Trigger | Status |
|---------|--------|
| Real Collar PD/server | ✅ NOT TRIGGERED |
| New PDX ABI/opcodes | ✅ NOT TRIGGERED |
| Kernel edits | ✅ NOT TRIGGERED |
| sex-pdx ABI edits | ✅ NOT TRIGGERED |
| sexdisplay changes | ✅ NOT TRIGGERED |
| Persistent grant storage | ✅ NOT TRIGGERED |
| Cross-PD grant propagation | ✅ NOT TRIGGERED |
| Hardware MPK permission mutation | ✅ NOT TRIGGERED |
| User prompt UI with security meaning | ✅ NOT TRIGGERED |
| Renderer authority decisions | ✅ NOT TRIGGERED |
| Broad rewrite of operation paths | ✅ NOT TRIGGERED |

**STOP FIRST: NOT TRIGGERED** — All forbidden areas clean.

### git diff Verification

The C2 commit (`bf1fc7a`) modifies only:

```
servers/silk-shell/src/main.rs                     | 210 ++++++++++--
docs/handoff/C2_COLLAR_POLICY_TABLE_V2.md          | 383 +++++++++++++++++++++
```

Only `servers/silk-shell/src/main.rs` and the handoff doc were modified.
No kernel, sex-pdx, sexdisplay, bell, mesh, linen, or quil server changes.

**Boundaries: INTAKT** — All boundary areas confirmed clean.

## 7. Remaining Risks

| Risk | Severity | Status | Mitigation |
|------|----------|--------|------------|
| No dynamic grants for linked buffers | LOW | Accepted | Auto-grants cover seed objects; LinkObjectToBuffer only applies to Linen objects (not buffers) |
| Caller identity from FOCUSED_SURFACE_ID is implicit | LOW | Documented | Safe in single-threaded dispatch; real security requires kernel-enforced PD identity |
| No per-object generation validation | LOW | Accepted | Objects are never deleted/recreated in V2 — no stale-grant scenario exists |
| No revocation support | LOW | Accepted | Grant lifecycle states defined but no path sets them yet |
| Only LinkObjectToBuffer wired | LOW | Accepted | OpenObject follows same code path but has no auto-grants — would Deny if called |
| V2 is still shell-local, not real security | INFO | Documented | Kernel's PDX/MPK is the real security boundary; Collar is advisory |
| Non-Linen/Mesh callers now denied (behavior change from J5) | INFO | Improvement | Previously all callers got AllowStub; now unauthorized surfaces correctly denied |

## 8. Next Safest Step

**C4: Denied-path visual/audit surfacing design** — Docs-only design for surfacing
Collar denied-path information on the Collar surface and/or Mesh surface:

1. **Collar surface grant list** — Display active grants on Collar placeholder
   surface (currently a teal fill rect). Read-only list of auto-grants.
2. **Collar audit ring visualization** — Display recent `[collar.audit.write]`
   events on Collar surface. Mirrors Bell event list pattern.
3. **Mesh Collar-audit fact kind** — Add `CollarAudit` fact kind to Mesh ring
   so denied operations appear in Mesh fact list.
4. **No new authority decisions** — C4 is visualization only. No Collar policy
   changes, no new grant paths.

**After C4:** Evaluate Bell→Collar audit integration (Mirror audit events as
Bell events), or dynamic grants at link time.

---

*End of C3 audit. Verdict: PASS_C2_COLLAR_POLICY_TABLE — no corrections needed.*
