# C1: Collar Real Authority Model Design

**Status:** Complete (Design only — no code changes)
**Date:** 2026-05-06
**Purpose:** Design the real Collar authority model that will eventually replace
AllowStub decisions. Define authority objects, grant/capability state, policy
dimensions, integration map, and minimal safe V2 implementation. Docs only.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║    SAFE_SHELL_LOCAL_COLLAR_POLICY_V2_DESIGN                  ║
║    BLOCKED_REAL_COLLAR_PD                                    ║
║    BLOCKED_STOP_FIRST                                        ║
╠══════════════════════════════════════════════════════════════╣
║ Current Collar:          J5 shell-local stubs only           ║
║ Real Collar PD:          BLOCKED — requires STOP FIRST       ║
║ V2 policy table:         SAFE — shell-local, no new ABI      ║
║ No code changes:         CONFIRMED (design only)              ║
╚══════════════════════════════════════════════════════════════╝
```

## 1. Current Collar Reality

### Where Collar Exists Today

| Location | What | Type |
|----------|------|------|
| `servers/silk-shell/src/main.rs` (line 1108) | `CollarOperation` enum — 7 variants | Stub |
| `servers/silk-shell/src/main.rs` (line 1122) | `CollarDecision` enum — 5 variants | Stub |
| `servers/silk-shell/src/main.rs` (line 1143) | `collar_check_operation_stub()` | Stub |
| `servers/silk-shell/src/main.rs` (line 998) | Called in `open_linen_object_in_quil()` | Single wire point |
| `servers/silk-shell/src/main.rs` (line 233) | `LinenObject::grant_ref` field | Always 0 |
| `servers/silk-shell/src/main.rs` (line 698) | `QuilBuffer::grant_ref` field | Always 0 |
| `servers/silk-shell/src/main.rs` (line 203) | Collar placeholder surface ID | Surface only |
| `servers/silk-shell/src/main.rs` (line 6019) | Collar surface open/toggle/frame helpers | Navigation only |
| `docs/handoff/F2_COLLAR_AUTHORITY_MAP_V1.md` | Full authority object model spec | Docs only |
| `docs/handoff/J5_COLLAR_GATED_OPERATION_STUBS_V1.md` | Stub implementation handoff | Code |
| `docs/handoff/I2_COLLAR_PLACEHOLDER_SURFACE_V1.md` | Collar surface in scene/frame/tab | Code |

### What Is Stubbed

- **All Collar decisions** — `AllowStub` for `OpenObject` and `LinkObjectToBuffer`
- **All grant_ref values** — Every object and buffer has `grant_ref: 0` (no grant)
- **Collar PD** — No real Collar server process exists
- **Collar grant UI** — Collar surface is a teal fill rect placeholder only
- **Collar audit ring** — No audit event recording
- **Collar-Bell integration** — No action token approval path
- **Collar-Mesh integration** — No authority graph edges from Collar state

### What Operations Already Call Collar

| Operation | Call Site | Decision | Status |
|-----------|-----------|----------|--------|
| `LinkObjectToBuffer` (op=6) | `open_linen_object_in_quil()` line 998 | `AllowStub` | Wired |
| `OpenObject` (op=0) | Not yet called | `AllowStub` | Unwired |
| `RenameObject` (op=1) | Not yet called | `NeedsGrantLater` | Unwired |
| `ArchiveObject` (op=2) | Not yet called | `NeedsGrantLater` | Unwired |
| `SaveBuffer` (op=3) | Not yet called | `BlockedStopFirst` | Unwired |
| `BuildTarget` (op=4) | Not yet called | `BlockedStopFirst` | Unwired |
| `RunTarget` (op=5) | Not yet called | `BlockedStopFirst` | Unwired |

### What Authority Decisions Are Fake / AllowStub

| Decision | Current Behavior | Real Behavior Needed |
|----------|-----------------|---------------------|
| `AllowStub` | Always returned for OpenObject / LinkObjectToBuffer | Must check caller identity, object kind, focus context |
| `DenyMissingObject` | Validates object exists in LINEN_OBJECTS | Keep — this is a real integrity check |
| `DenyMissingBuffer` | Validates buffer exists in QUIL_BUFFERS | Keep — this is a real integrity check |
| `NeedsGrantLater` | Returned for RenameObject / ArchiveObject | Must check user-approved grant state |
| `BlockedStopFirst` | Returned for SaveBuffer / BuildTarget / RunTarget | Keep — these require STOP FIRST review |

### What Must NOT Be Treated as Real Security Yet

1. **AllowStub for LinkObjectToBuffer** — Not real authority. An object in the
   shell-local LINEN_OBJECTS table is NOT authenticated. Collar has NOT verified
   the caller's identity.
2. **grant_ref == 0** — Not a denied state. It's an uninitialized placeholder.
3. **Collar surface** — A fill-rect placeholder, not an authority wallet UI.
4. **Shell-local dispatch** — No real Collar PD means no PDX-level authority
   enforcement. The kernel's capability system is the real boundary.
5. **Mesh topology facts** — Recorded from shell-local state, not from Collar
   audit. Mesh shows what links exist, not what authority governs them.

## 2. Real Authority Objects

### Subject (Who Asks)

| Subject | Identifier | Example |
|---------|-----------|---------|
| Shell action | `sid: u64` (surface_id) | `SURFACE_ID_LINEN` (200) |
| PD identity | `pd_slot: u8` | Domain 3 (silk-shell) |
| User action | `action_id: u64` | Keyboard shortcut, mouse click |
| Command palette | `cmd_index: u8` | "Open in Quil" command |
| App manifest | `manifest_hash: u64` | Verified at launch |

### Object (What Is Accessed)

| Object | Identifier | Creator | Example |
|--------|-----------|---------|---------|
| Linen object | `object_id: u64` | Shell-local seed or future | Seed objects 1-6 |
| Quil buffer | `buffer_id: u64` | J4 dynamic creation | 1000+N |
| Mesh fact | `fact_id: u64` | J6 automatic | Ring index |
| Surface | `surface_id: u64` | Boot init | 200 (Linen), 202 (Mesh) |
| Device | `device_id: u64` | Kernel/hardware | USB HID device |
| PD route | `slot_id: u8` | Kernel init | Domain 3 → Domain 8 |
| File/store object | `store_handle: u64` | sexstore | Future |

### Operation (What Is Done)

| Operation | Code | Collar Gate | Status |
|-----------|------|-------------|--------|
| `OpenObject` | 0 | `AllowStub` | Safe for V2 |
| `RenameObject` | 1 | `NeedsGrantLater` | Requires grant |
| `ArchiveObject` | 2 | `NeedsGrantLater` | Requires grant |
| `SaveBuffer` | 3 | `BlockedStopFirst` | STOP FIRST |
| `BuildTarget` | 4 | `BlockedStopFirst` | STOP FIRST |
| `RunTarget` | 5 | `BlockedStopFirst` | STOP FIRST |
| `LinkObjectToBuffer` | 6 | `AllowStub` | Safe for V2 |
| `FocusLinenObject` | 7 | (new) | Read-only — no gate needed |
| `ReadBufferContent` | 8 | (new) | Requires grant |
| `ExecuteBuildOutput` | 9 | (new) | STOP FIRST |
| `GrantCapability` | 10 | (new) | Admin only |
| `RevokeCapability` | 11 | (new) | Admin only |

### Context (Under What Conditions)

| Context | Values | Description |
|---------|--------|-------------|
| Active Scene | `scene_id: u8` | 0-3 (main, project, debug, monitor) |
| Focus Owner | `surface_id: u64` | Surface currently receiving keyboard input |
| Trust Lane | `system`, `local`, `known`, `untrusted` | Derived from caller identity |
| Session State | `unlocked`, `locked`, `privacy_mode` | Current session trust level |
| Origin | `keyboard`, `palette`, `api`, `mesh` | How the operation was triggered |
| Mesh Topology | `has_link: bool` | Whether subject ↔ object has a recorded link |

### Decision (What Collar Returns)

| Decision | Description | V2 Available? |
|----------|-------------|---------------|
| `Allow` | Operation permitted (replaces AllowStub) | ✅ Yes |
| `Deny` | Operation denied (policy or grant missing) | ✅ Yes |
| `AllowStub` | Safe for development — not real authority | ⏳ Transitional |
| `DenyMissingObject` | Referenced object not found | ✅ Keep |
| `DenyMissingBuffer` | Referenced buffer not found | ✅ Keep |
| `NeedsGrantLater` | Would require user-approved grant | ✅ Keep |
| `BlockedStopFirst` | STOP FIRST trigger | ✅ Keep |

## 3. Grant / Capability State Design

### Core Constraint: Static Tables Only, No Heap

All Collar grant state must be stored in fixed-size arrays with no heap
allocation. This is consistent with the existing shell-local pattern
(LINEN_OBJECTS[16], QUIL_BUFFERS[16], MESH_FACTS[32], TOMBSTONE_RING[16]).

### Grant Table

```rust
/// A single Collar grant record. No heap, no strings, no pointers.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct CollarGrant {
    grant_id: u64,              // Monotonic grant identifier
    subject_id: u64,            // Who holds the grant (PD slot, surface_id)
    object_id: u64,             // What resource the grant applies to
    operation_mask: u64,        // Bitmask of allowed CollarOperation values
    generation: u64,            // Monotonic generation for stale-grant detection
    state: CollarGrantState,    // Active, Revoked, Expired, Tombstoned
    origin: u64,                // What created this grant (user action, boot manifest)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum CollarGrantState {
    Active = 0,
    Revoked = 1,
    Expired = 2,
    Tombstoned = 3,
}
```

### Grant Table Size

| Constant | Value | Rationale |
|----------|-------|-----------|
| `COLLAR_GRANT_CAP` | 32 | One grant per seed object + dynamic buffers + system grants |

### Generation IDs

| Generation Counter | Initial | Purpose |
|-------------------|---------|---------|
| `COLLAR_GRANT_GENERATION` | 1 | Monotonic counter for grant creation (0 reserved) |
| `PER_OBJECT_GRANT_GEN` | 1 | Stored in LinenObject/QuilBuffer alongside grant_ref |

### Stale Grant Detection

A grant is stale if:
1. The grant's `generation` does not match the current per-object generation
2. The grant's `state` is `Revoked`, `Expired`, or `Tombstoned`
3. The referenced object no longer exists (validated at check time)

### Grant Lifecycle (Simplified for V2)

```
                 ┌──────────────┐
                 │   Requested  │
                 └──────┬───────┘
                        │
              ┌─────────┼──────────┐
              │         │          │
         ┌────▼──┐ ┌───▼────┐ ┌───▼──────┐
         │Active │ │ Denied │ │ Expired  │
         └───┬───┘ └────────┘ └───┬──────┘
             │                    │
         ┌───▼──────┐        (terminal)
         │ Revoked  │
         └───┬──────┘
             │
         ┌───▼──────────┐
         │ Tombstoned   │ ← terminal after full audit
         └──────────────┘
```

V2 supports 4 states (vs. F2's 7-state model). Prompt, Faulted, and Audited
states are deferred as they require UI and real PD infrastructure.

### Audit Event Ring

```rust
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct CollarAuditEvent {
    event_id: u64,              // Monotonic audit identifier
    timestamp_seq: u64,         // Sequence number (no wall clock)
    operation: CollarOperation, // What was attempted
    object_id: u64,             // What resource was accessed
    subject_id: u64,            // Who attempted the operation
    decision: CollarDecision,   // Allow, Deny, etc.
    grant_ref: u64,             // Grant that authorized (or 0 if denied)
    reason: u64,                // Policy reason code
}
```

| Constant | Value | Rationale |
|----------|-------|-----------|
| `COLLAR_AUDIT_CAP` | 64 | Events fire frequently; ring must hold enough for diagnostic |

Ring overflow: overwrite oldest (same as Bell/Mesh/Tombstone ring pattern).

## 4. Decision Enum Design (V2)

```rust
/// Decision from the Collar operation gate.
/// V2 replaces AllowStub with real Allow/Deny for wired operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum CollarDecision {
    /// Operation permitted — caller identity, object, and context validated.
    Allow = 0,
    /// Operation denied — policy reject or grant missing.
    Deny = 1,
    /// Referenced Linen object not found.
    DenyMissingObject = 2,
    /// Referenced Quil buffer not found.
    DenyMissingBuffer = 3,
    /// Operation would require a user-approved grant.
    NeedsGrantLater = 4,
    /// Operation blocked by STOP FIRST policy.
    BlockedStopFirst = 5,
}
```

### Changes from J5

| J5 Decision | V2 Decision | Change |
|-------------|-------------|--------|
| `AllowStub` (0) | Removed | Replaced by real `Allow` (0) |
| — | `Allow` (0) | New — real authority check passed |
| — | `Deny` (1) | New — operation denied by policy |
| `DenyMissingObject` (1) | `DenyMissingObject` (2) | Re-numbered |
| `DenyMissingBuffer` (2) | `DenyMissingBuffer` (3) | Re-numbered |
| `NeedsGrantLater` (3) | `NeedsGrantLater` (4) | Re-numbered |
| `BlockedStopFirst` (4) | `BlockedStopFirst` (5) | Re-numbered |

**Breaking change:** The enum values shift. All call sites that match on
`CollarDecision` must be updated. This is acceptable for V2 because only
one call site exists (`open_linen_object_in_quil()` line 999).

## 5. Policy Dimensions

| # | Dimension | Values | V2 Priority |
|---|-----------|--------|-------------|
| 1 | **Caller identity** | Surface ID, PD slot, action origin | HIGH — LinkObjectToBuffer must verify caller owns the object |
| 2 | **Object kind** | LinenObjectKind, QuilBufferKind | HIGH — CodeFile vs. Document vs. BuildArtifact |
| 3 | **Operation kind** | CollarOperation variant | HIGH — Different operations have different policies |
| 4 | **Active scene** | scene_id, focus owner | MEDIUM — Only allow operations on objects in active scene |
| 5 | **Grant present** | CollarGrant exists with matching subject/object/operation | HIGH — Core of V2 policy |
| 6 | **Grant state** | Active, Revoked, Expired, Tombstoned | HIGH — Stale grants must be rejected |
| 7 | **Generation match** | Grant generation vs. per-object generation | MEDIUM — Prevent grant reuse after object recreation |
| 8 | **Focus context** | Is the calling surface focused? | MEDIUM — Input focus as implicit authority |
| 9 | **Mesh topology** | Does a Mesh fact link subject↔object? | LOW — Diagnostic only in V2 |
| 10 | **Bell severity** | Would denial trigger a Bell event? | LOW — Deferred to Bell integration |

### V2 Policy Decision Tree

```
collar_check_operation(op, subject_id, object_id, buffer_id) → CollarDecision:

1. Validate existence (same as J5)
   → object_id != 0 and not in LINEN_OBJECTS → DenyMissingObject
   → buffer_id != 0 and not in QUIL_BUFFERS → DenyMissingBuffer

2. Check STOP FIRST operations (same as J5)
   → SaveBuffer, BuildTarget, RunTarget → BlockedStopFirst

3. Check NeedsGrantLater operations (same as J5)
   → RenameObject, ArchiveObject → NeedsGrantLater

4. Check Grant Table (NEW — replaces AllowStub for wired ops)
   → Search COLLAR_GRANTS for:
       subject_id == caller_id
       && object_id == target_object
       && operation_mask includes op
       && state == Active
       && generation matches per-object generation
   → Found → Allow  (emit [collar.gate.allow] with grant_id)
   → Not found → Deny  (emit [collar.gate.deny] with reason)
```

### V2 Initial Policy Table

| Operation | Subject | Object | Grant Required? | V2 Decision |
|-----------|---------|--------|----------------|-------------|
| `LinkObjectToBuffer` | Linen surface (200) | Any seed object (1-6) | YES — auto-granted at boot | `Allow` |
| `LinkObjectToBuffer` | Mesh surface (202) | Any linked object | YES — auto-granted at boot | `Allow` |
| `LinkObjectToBuffer` | Any other surface | Any object | YES — no grant exists | `Deny` |
| `OpenObject` | Any surface | Any object | YES — auto-granted at boot for seeds | `Allow` |
| `OpenObject` | Any surface | Dynamic buffer | YES — grant created at link time | `Allow` |

### Auto-Grants at Boot

For V2, boot creates auto-grants for all seed objects to known surfaces:

| Grant | Subject | Object | Operations | Origin |
|-------|---------|--------|------------|--------|
| G1 | Linen surface (200) | Seed 1 (CodeFile) | OpenObject, LinkObjectToBuffer | boot |
| G2 | Linen surface (200) | Seed 2 (Document) | OpenObject, LinkObjectToBuffer | boot |
| G3 | Linen surface (200) | Seed 3 (QuilWorkspaceRef) | OpenObject, LinkObjectToBuffer | boot |
| G4 | Linen surface (200) | Seed 4 (Reference) | OpenObject, LinkObjectToBuffer | boot |
| G5 | Linen surface (200) | Seed 5 (MeshDiagnosticRef) | OpenObject, LinkObjectToBuffer | boot |
| G6 | Linen surface (200) | Seed 6 (MessageDraft) | OpenObject, LinkObjectToBuffer | boot |
| G7 | Linen surface (200) | Dynamic buffer (1000+N) | OpenObject, LinkObjectToBuffer | link_time |
| G8 | Mesh surface (202) | Dynamic buffer (1000+N) | LinkObjectToBuffer | link_time |

## 6. Integration Map

### Linen ↔ Collar

| Linen Action | Current | V2 Collar |
|-------------|---------|-----------|
| Open object in Quil (PrintScreen) | `AllowStub` | Check COLLAR_GRANTS for subject=Linen, object=target, op=LinkObjectToBuffer |
| Select object (J/K) | No gate needed | No gate needed (read-only) |
| Focus Linen (F8/Enter from Mesh) | No gate needed | No gate needed (navigation) |
| Render object list | No gate needed | No gate needed (display) |

### Quil ↔ Collar

| Quil Action | Current | V2 Collar |
|------------|---------|-----------|
| Create buffer (J4 dynamic) | `AllowStub` | Check grant exists for subject=Linen, object=target |
| Reuse existing buffer | `AllowStub` | Check grant same as create |
| Render buffer list | No gate needed | No gate needed (display) |

### Mesh ↔ Collar

| Mesh Action | Current | V2 Collar |
|------------|---------|-----------|
| Record fact (J6) | No gate needed | No gate needed (passive observation) |
| Render fact list | No gate needed | No gate needed (display) |
| Focus Linen at fact (Enter) | No gate needed | No gate needed (navigation) |
| Open in Quil from fact (PrintScreen) | Via `open_linen_object_in_quil()` | Same path — Collar gate inside callee |
| Display Collar audit state | Not implemented | Future — Mesh reads COLLAR_AUDIT ring |

### Bell ↔ Collar

| Bell Action | Current | V2 Collar |
|------------|---------|-----------|
| Record event (object link) | No Collar gate | No Collar gate (passive recording) |
| Emit audit event for Collar decision | Not implemented | Record `CollarAuditEvent` on Allow/Deny |
| Action token approval | Not implemented | Deferred — requires Bell PD |

### Silk Shell ↔ Collar

| Shell Action | Current | V2 Collar |
|-------------|---------|-----------|
| Keyboard dispatch | Asks `open_linen_object_in_quil()` which calls Collar | Same — Collar is in the callee |
| Command palette execute | Routes to existing handlers | Same — Collar in callee |
| Surface lifecycle | No Collar gate | No — lifecycle is shell-local |

### sexdisplay ↔ Collar

**sexdisplay never owns authority.** The renderer renders pixels; it does not
decide what is trusted. Collar decisions are made in silk-shell, and sexdisplay
is told what to draw via existing 0xEC/0xEF/0xEE primitives.

### PDX / MPK ↔ Collar

**Collar never bypasses kernel capability enforcement.** Collar decisions are
advisory to the kernel's existing PDX slot isolation and MPK/PKEY memory
protection. Real security comes from the kernel, not from Collar stubs.

## 7. Minimal Safe V2 Implementation Plan

### V2 Scope

| Category | In Scope | Not In Scope |
|----------|----------|-------------|
| Collar grant table | ✅ Static `[Option<CollarGrant>; 32]` | ❌ Real Collar PD |
| Collar audit ring | ✅ Static `[Option<CollarAuditEvent>; 64]` | ❌ Persistent storage |
| Policy decision replacement | ✅ Replace `AllowStub` with `Allow`/`Deny` for wired ops | ❌ New ABI/opcodes |
| Auto-grant at boot | ✅ Seed object grants | ❌ Grant UI |
| Generation validation | ✅ Per-object generation counters | ❌ Cross-PD grants |
| Proof markers | ✅ `[collar.gate.allow]`, `[collar.gate.deny]` | ❌ Bell integration |
| CollarOperation enum | ✅ Add 5 new variants (FocusLinenObject, ReadBufferContent, etc.) | ❌ New Collar PD |

### V2 Changes to `servers/silk-shell/src/main.rs`

#### 1. Add CollarGrant and CollarAuditEvent structs

```rust
// After existing CollarOperation/CollarDecision enums.
// Add new types before collar_check_operation().

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct CollarGrant {
    grant_id: u64,
    subject_id: u64,
    object_id: u64,
    operation_mask: u64,
    generation: u64,
    state: CollarGrantState,
    origin: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum CollarGrantState {
    Active = 0,
    Revoked = 1,
    Expired = 2,
    Tombstoned = 3,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct CollarAuditEvent {
    event_id: u64,
    timestamp_seq: u64,
    operation: CollarOperation,
    object_id: u64,
    subject_id: u64,
    decision: CollarDecision,
    grant_ref: u64,
    reason: u64,
}
```

#### 2. Add static tables

```rust
const COLLAR_GRANT_CAP: usize = 32;
static mut COLLAR_GRANTS: [Option<CollarGrant>; COLLAR_GRANT_CAP] = [None; COLLAR_GRANT_CAP];
static mut COLLAR_GRANT_GENERATION: u64 = 1;  // 0 reserved

const COLLAR_AUDIT_CAP: usize = 64;
static mut COLLAR_AUDIT_EVENTS: [Option<CollarAuditEvent>; COLLAR_AUDIT_CAP] = [None; COLLAR_AUDIT_CAP];
static mut COLLAR_AUDIT_WRITE_INDEX: u64 = 0;
```

#### 3. Update `collar_check_operation_stub()` → `collar_check_operation()`

Replace the `AllowStub` case with a real grant table lookup:

```rust
fn collar_check_operation(
    op: CollarOperation,
    caller_sid: u64,
    object_id: u64,
    buffer_id: u64,
) -> CollarDecision {
    // 1. Validate existence (unchanged from J5)
    // ...

    // 2. STOP FIRST operations (unchanged from J5)
    // ...

    // 3. NeedsGrantLater operations (unchanged from J5)
    // ...

    // 4. Grant table lookup (NEW — replaces AllowStub)
    // Determine the target object_id for grant lookup.
    let target_id = if object_id != 0 { object_id } else { buffer_id };
    if target_id == 0 {
        return CollarDecision::Deny;
    }

    for slot in COLLAR_GRANTS.iter() {
        if let Some(grant) = slot {
            if grant.state != CollarGrantState::Active { continue; }
            if grant.subject_id != caller_sid { continue; }
            if grant.object_id != target_id { continue; }
            if (grant.operation_mask & (1 << (op as u64))) == 0 { continue; }
            // Generation match prevents stale grants.
            // (Per-object generation stored alongside grant_ref.)
            serial_println!("[collar.gate.allow] op={} subject={} object={} grant={}",
                op as u8, caller_sid, target_id, grant.grant_id);
            record_collar_audit(op, target_id, caller_sid, CollarDecision::Allow, grant.grant_id, 0);
            return CollarDecision::Allow;
        }
    }

    // No matching active grant found.
    serial_println!("[collar.gate.deny] op={} subject={} object={} reason=no_grant",
        op as u8, caller_sid, target_id);
    record_collar_audit(op, target_id, caller_sid, CollarDecision::Deny, 0, 1);
    CollarDecision::Deny
}
```

#### 4. Add auto-grant boot initialization

```rust
unsafe fn collar_init_grants() {
    // Auto-grant: Linen surface can LinkObjectToBuffer for each seed object.
    for obj in LINEN_OBJECTS.iter() {
        if let Some(o) = obj {
            let idx = (COLLAR_GRANT_GENERATION as usize - 1) % COLLAR_GRANT_CAP;
            let gen = COLLAR_GRANT_GENERATION;
            COLLAR_GRANT_GENERATION += 1;
            COLLAR_GRANTS[idx] = Some(CollarGrant {
                grant_id: gen,
                subject_id: SURFACE_ID_LINEN,
                object_id: o.object_id,
                operation_mask: (1 << CollarOperation::OpenObject as u64)
                              | (1 << CollarOperation::LinkObjectToBuffer as u64),
                generation: gen,
                state: CollarGrantState::Active,
                origin: 0, // boot
            });
            serial_println!("[collar.grant.auto] grant_id={} subject={} object={} op_mask={:#x}",
                gen, SURFACE_ID_LINEN, o.object_id,
                (1 << CollarOperation::OpenObject as u64) | (1 << CollarOperation::LinkObjectToBuffer as u64));
        }
    }
    // Auto-grant: Mesh surface can LinkObjectToBuffer for linked objects.
    // (Deferred — Mesh auto-grants created when facts record links.)
    serial_println!("[collar.grant.init] count={} generation={}", COLLAR_GRANT_CAP, COLLAR_GRANT_GENERATION);
}
```

#### 5. Add audit recording

```rust
unsafe fn record_collar_audit(
    op: CollarOperation,
    object_id: u64,
    subject_id: u64,
    decision: CollarDecision,
    grant_ref: u64,
    reason: u64,
) {
    let idx = (COLLAR_AUDIT_WRITE_INDEX as usize) % COLLAR_AUDIT_CAP;
    COLLAR_AUDIT_EVENTS[idx] = Some(CollarAuditEvent {
        event_id: COLLAR_AUDIT_WRITE_INDEX,
        timestamp_seq: COLLAR_AUDIT_WRITE_INDEX, // monotonic, no wall clock
        operation: op,
        object_id,
        subject_id,
        decision,
        grant_ref,
        reason,
    });
    COLLAR_AUDIT_WRITE_INDEX += 1;
    serial_println!("[collar.audit.record] event_id={} op={} object={} subject={} decision={} grant={} reason={}",
        idx as u64, op as u8, object_id, subject_id, decision as u8, grant_ref, reason);
}
```

#### 6. Wire collar_init_grants() into boot sequence

Add `collar_init_grants()` call after `quil_buffer_table_init()` in boot.

#### 7. Update call site

`open_linen_object_in_quil()` passes `SURFACE_ID_LINEN` as caller_sid:

```rust
// Line 998 currently:
let decision = collar_check_operation_stub(CollarOperation::LinkObjectToBuffer, object_id, 0);

// V2:
let decision = collar_check_operation(CollarOperation::LinkObjectToBuffer, SURFACE_ID_LINEN, object_id, 0);
```

### V2 Build Test

- Build must pass with no new warnings
- Existing Linen→Quil open path must still work (auto-grants cover seed objects)
- Mesh PrintScreen path must still work (auto-grants or dynamic grants)
- No kernel/ABI/sexdisplay changes

### What V2 Does NOT Do

- Does NOT create a real Collar PD
- Does NOT add new PDX ABI/opcodes
- Does NOT change sexdisplay rendering
- Does NOT grant/revoke kernel capabilities
- Does NOT add persistent storage
- Does NOT add Bell integration (Collar→Bell audit events deferred)
- Does NOT add Mesh Collar-audit display
- Does NOT add user prompt UI
- Does NOT add grant revocation UI

## 8. STOP FIRST Table

| Trigger | C1 Status | V2 Would Trigger? |
|---------|-----------|-------------------|
| Real Collar PD/server | ✅ BLOCKED — design only | ❌ V2 is shell-local |
| New PDX ABI/opcodes | ✅ NOT TRIGGERED | ❌ V2 uses existing ops |
| Kernel edits | ✅ NOT TRIGGERED | ❌ V2 is shell-local |
| sex-pdx ABI edits | ✅ NOT TRIGGERED | ❌ V2 uses existing constants |
| sexdisplay changes | ✅ NOT TRIGGERED | ❌ V2 uses existing 0xEC/0xEF/0xEE |
| Persistent grant storage | ✅ NOT TRIGGERED | ❌ V2 is memory-only |
| Cross-PD grant propagation | ✅ NOT TRIGGERED | ❌ V2 is shell-local |
| Hardware MPK permission mutation | ✅ NOT TRIGGERED | ❌ V2 is advisory only |
| User prompt UI with security meaning | ✅ NOT TRIGGERED | ❌ V2 deferred |
| Renderer authority decisions | ✅ NOT TRIGGERED | ❌ sexdisplay never owns policy |
| Broad rewrite of operation paths | ✅ NOT TRIGGERED | ❌ V2 replaces only AllowStub |
| New CollarOperation kinds | ✅ NOT TRIGGERED | ⚠️ V2 adds FocusLinenObject etc. — safe additive |
| Bell integration (action tokens) | ✅ NOT TRIGGERED | ❌ V2 deferred |
| Mesh authority graph from Collar | ✅ NOT TRIGGERED | ❌ V2 deferred |

**C1: STOP FIRST NOT TRIGGERED** — Design only, no code changes.
**V2: STOP FIRST NOT TRIGGERED** — Shell-local, no new ABI, no kernel changes.

## 9. Relationship to Other Subsystems

### Mesh (N1-N15 Complete)

Mesh is the **authority graph visualizer**, not the authority owner. V2 Collar
grants can be displayed as Mesh fact kinds (e.g., `GrantCreated`, `GrantDenied`)
but Collar never reads Mesh state to make policy decisions.

### Bell (M1-M8 Complete)

Bell is the **attention firewall / event router**. V2 Collar:
- Records audit events to `COLLAR_AUDIT_EVENTS` ring (shell-local, same pattern)
- Does NOT route through Bell's ring — Bell integration deferred
- Future: Collar audit events are mirrored as Bell events with `ActionTokenRequested`

### Linen (J1-J7 Complete)

Linen objects are the primary resources gated by Collar. V2 Collar:
- Validates all LinkObjectToBuffer operations against grant table
- Auto-grants cover seed objects at boot
- Dynamic grants created when buffers are linked
- Rejects operations from non-Linen surfaces without grants

### Quil (J3-J4 Complete)

Quil buffers are secondary resources linked to Linen objects. V2 Collar:
- Inherits Linen object grants via buffer creation path
- Does not independently gate buffer read/edit (deferred)

### Surface Lifecycle (A3-A7 Complete)

Surface lifecycle (Visible, Minimized, Closing, Tombstoned, etc.) is
shell-local policy. Collar does NOT gate lifecycle transitions.

## 10. Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Static tables vs. dynamic allocation | Static | Consistent with all existing shell-local patterns (LINEN_OBJECTS, QUIL_BUFFERS, MESH_FACTS) |
| Grant table size | 32 | Covers 6 seed objects + 16 dynamic buffers + 10 system grants |
| Audit ring size | 64 | Twice the size of other rings because Collar decisions fire frequently |
| Decision enum renumbering | Breaking change | Only one call site exists — acceptable for V2 |
| Caller identity | Surface ID (u64) | Consistent with existing shell architecture. Future: PD slot number |
| Auto-grant at boot | Seed objects only | Dynamic grants created at link time |
| No Bell integration in V2 | Deferred | Keep V2 minimal. Bell integration requires Bell PD or extended ring |

## 11. Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| AllowStub replacement may break existing paths if grants misconfigured | MEDIUM | V2 must preserve all current allowed paths |
| Collar surface is still a teal fill rect — no grant visualization | LOW | Deferred — not needed for V2 policy enforcement |
| No revocation UI — user cannot revoke grants | LOW | Deferred — grants are auto-created; revocation requires UI |
| No persistence — grants lost on reboot | LOW | Acceptable — auto-grants recreated at boot |
| Only LinkObjectToBuffer is wired; other operations unwired | LOW | V2 wires only LinkObjectToBuffer; others deferred |
| Caller identity spoofed by shell dispatch | MEDIUM | Shell dispatch is single-threaded — caller_sid is trusted |
| grant_ref field in LinenObject/QuilBuffer still 0 | LOW | V2 uses grant table lookup instead of grant_ref |

## 12. C2 Implementation Prompt Summary

**C2: Implement V2 Collar policy table** — Replace AllowStub with real grant
table lookup for `LinkObjectToBuffer`. Changes to `servers/silk-shell/src/main.rs`:

1. Add `CollarGrant` struct + `COLLAR_GRANTS[32]` static table
2. Add `CollarAuditEvent` struct + `COLLAR_AUDIT_EVENTS[64]` ring
3. Add `CollarGrantState` enum (Active, Revoked, Expired, Tombstoned)
4. Rename `collar_check_operation_stub()` → `collar_check_operation()`
5. Update signature: add `caller_sid: u64` parameter
6. Replace `AllowStub` branch with grant table lookup
7. Add `collar_init_grants()` — auto-grants for seed objects at boot
8. Add `record_collar_audit()` — audit event on Allow/Deny
9. Add `[collar.gate.allow]`, `[collar.gate.deny]`, `[collar.audit.record]`, `[collar.grant.auto]`, `[collar.grant.init]` proof markers
10. Wire `collar_init_grants()` after `quil_buffer_table_init()` in boot
11. Update `open_linen_object_in_quil()` call site to pass `SURFACE_ID_LINEN`
12. Call `record_collar_audit()` on Deny paths too
13. Build and verify existing Linen→Quil paths still work

**Do NOT:**
- Create real Collar PD
- Add new PDX/ABI/opcodes
- Change sexdisplay or kernel
- Add grant revocation/UI
- Add Bell integration
- Add Mesh Collar-audit display
- Add persistence
- Wire operations beyond LinkObjectToBuffer

---

*End of C1 Collar Real Authority Model Design. C2 implements V2 policy table.*
