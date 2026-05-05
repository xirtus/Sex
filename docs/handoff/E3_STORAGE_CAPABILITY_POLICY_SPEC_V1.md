# E3_STORAGE_CAPABILITY_POLICY_SPEC_V1

**Status:** Docs/spec only. No code changed.

**Date:** 2026-05-05

**Review gate:** "Do not fake caller identity. If sexstore cannot observe caller
PD today, E4 must stay topology-limited or STOP FIRST for identity/ABI."

---

## Summary

Specification of a storage capability policy for sexstore K/V access control.
Defines StoreCapability kinds, caller identity validation, range ownership
checks, enforcement model, Collar integration points, and capability deny
proof markers.

E3 is the authorization layer on top of E2's protocol spec. E2 specifies
*what operations exist*; E3 specifies *who may perform them*.

**Caller identity finding:** sexstore already observes `msg.caller_pd` natively
through PDX messages (`PdxMessage.caller_pd`, available in
`pdx_listen_raw(0)`). No identity faking is needed — sexstore can determine
the caller's domain ID for every request. E3 is not blocked by identity/ABI.

---

## 1. Current Enforcement (E2 baseline)

### 1.1 How sexstore receives caller identity today

```rust
// servers/sexstore/src/main.rs, line 68:
let caller = msg.caller_pd as u64;  // domain ID of caller
```

The `PdxMessage` struct (`crates/sex-pdx/src/lib.rs:52`) always includes
`caller_pd: u32` for every received message. The kernel populates this
field from the sender's PD ID at dispatch time. It is not caller-falsifiable
through normal PDX — the kernel writes `caller_pd` from its own cap table,
not from any user-supplied value.

### 1.2 Current usage

`caller_pd` is currently used **only** for `kv_reply()` routing (lines 68,
109, 133, 143). It is never checked against any allowlist or capability
table before serving a GET or PUT.

### 1.3 Current access topology

```
SLOT_SEXSTORE cap → silk-shell (domain 3) → sexstore (domain 8)
  ↑ Only cap grant
  ↑ No domain-ID allowlist in sexstore
  ↑ Any PDX message arriving at sexstore is served regardless of caller_pd
```

**Implication:** If kernel/src/init.rs were modified to grant `SLOT_SEXSTORE`
to another PD (e.g., SexAudio at domain 10), sexstore would serve that
caller without any additional gating. There is no defense-in-depth.

---

## 2. Caller Identity Model

### 2.1 How identity arrives

```
PDX message flow:
  Caller (domain N) → syscall 0 (pdx_call) on SLOT_SEXSTORE
    → kernel writes caller_pd = N into the message ring
    → sexstore reads pdx_listen_raw(0)
    → msg.caller_pd == N

This identity is KERNEL-AUTHORITATIVE:
  - The kernel populates caller_pd from its own cap table
  - Not derived from any user-supplied field
  - Not falsifiable by the caller through normal PDX
  - sexstore trusts msg.caller_pd because the kernel is the only writer
```

### 2.2 Identity model invariants

1. `caller_pd` is the sender's PDX domain ID, assigned at boot and immutable.
2. `caller_pd == 0` is the kernel (domain 0). Kernel calls are infrequent
   (boot-time cap genesis only) and are always trusted.
3. `caller_pd` corresponds 1:1 with the sender's `struct ProtectionDomain`
   entry in the kernel's `DOMAIN_REGISTRY`.
4. There is no PDX mechanism for a caller to spoof another PD's `caller_pd`.
   The kernel's message_ring enqueue writes `caller_pd` from the sender's
   registered ID, not from any message field.
5. For the current `pdx_listen_raw(0)` path, sexstore always receives
   the correct `caller_pd`. No ABI change is needed.

### 2.3 Review gate resolution

> ✅ sexstore can observe caller PD natively through `msg.caller_pd`.
> No identity faking needed. No ABI/kernel change required.
> E3 can proceed with topology-limited enforcement.

---

## 3. StoreCapability Model

### 3.1 Capability kinds

| Kind | Symbol | Allows | Scope |
|------|--------|--------|-------|
| **StoreRead** | `SCO_STORE_READ` | GET on matching keys | Key range |
| **StoreWrite** | `SCO_STORE_WRITE` | PUT on matching keys | Key range |
| **StoreDelete** | `SCO_STORE_DELETE` | Future DELETE on matching keys | Key range |
| **StoreAdmin** | `SCO_STORE_ADMIN` | All ops, schema migration, repair, compaction | Full table |

### 3.2 Capability-to-operation mapping

| Operation | Required capability |
|-----------|-------------------|
| GET key `K` | `StoreRead(range)` where `range` contains `K` |
| PUT key `K` | `StoreWrite(range)` where `range` contains `K` |
| DELETE key `K` | `StoreDelete(range)` where `range` contains `K` (E6+) |
| Schema migration | `StoreAdmin` |
| Table repair/compact | `StoreAdmin` |
| Full table scan | `StoreAdmin` |

### 3.3 Capability structure (spec)

```rust
/// Key range: inclusive start, inclusive end.
/// Example: Range { start: 0x10, end: 0x1F } covers Theremin settings.
struct KeyRange {
    start: u32,
    end: u32,
}

/// A granted storage capability.
/// E3: static/immutable at compile time (no runtime grant table yet).
struct StoreCapability {
    /// Which PD holds this capability.
    holder_pd: u32,
    /// What operations are allowed.
    kind: StoreCapKind,  // Read | Write | Delete | Admin
    /// Which keys this capability covers.
    /// StoreAdmin may have range = FULL_TABLE sentinel.
    key_range: KeyRange,
}
```

### 3.4 Static capability table

E3 specifies capabilities as a **compile-time static table** in sexstore.
No runtime grant or revoke in E3. The table is an array of
`StoreCapability` entries that sexstore checks on every request.

```rust
/// Static capability table — compiled into sexstore.
/// E3 only. E4+ may add runtime grant/revoke.
static STORE_CAPS: [StoreCapability; 2] = [
    // silk-shell (domain 3): full access to shell legacy range
    StoreCapability {
        holder_pd: 3,
        kind: StoreCapKind::Read,
        key_range: KeyRange { start: 0x01, end: 0x0F },
    },
    StoreCapability {
        holder_pd: 3,
        kind: StoreCapKind::Write,
        key_range: KeyRange { start: 0x01, end: 0x0F },
    },
    // kernel (domain 0): StoreAdmin — full table access
    StoreCapability {
        holder_pd: 0,
        kind: StoreCapKind::Admin,
        key_range: KeyRange { start: 0x00, end: 0xFF },  // FULL_TABLE
    },
];
```

### 3.5 Enforcement flow

```
sexstore receives message:
  ┌─ Extract caller_pd, opcode, key, value
  ├─ If opcode is unknown → reply(KV_UNSUPPORTED_OP); [sexstore.reply]
  ├─ If key == 0 → reply(KV_INVALID_KEY); [sexstore.key.invalid]
  ├─ Lookup caller_pd in STORE_CAPS:
  │   ├─ No matching entry → reply(KV_DENIED); [sexstore.put.reject] or [sexstore.get.reject]
  │   ├─ Matching entry, wrong kind (e.g., Read but op=PUT) → reply(KV_DENIED)
  │   ├─ Matching entry, key outside range → reply(KV_DENIED)
  │   └─ Matching entry, valid kind + in range → proceed to operation
  └─ Execute operation → reply(KV_OK or error)
```

### 3.6 Who gets what in E3

| PD | Domain | StoreCap | Key range | Rationale |
|----|--------|----------|-----------|-----------|
| silk-shell | 3 | Read, Write | `0x01–0x0F` | Scene appearance, input config, audio policy flags |
| silk-shell | 3 | Admin | `0x70–0x7F` | Admin/debug keys (future E6+: compaction, repair) |
| kernel | 0 | Admin | Full table | Boot-time cap genesis, emergency repair |
| All others | — | None | — | No storage caps in E3 |

### 3.7 What E3 does NOT do

- ❌ No runtime grant/revoke (cap table is compile-time static)
- ❌ No per-key granularity (range-based only)
- ❌ No Delete capability (opcode doesn't exist yet — E6)
- ❌ No Collar integration (E3 defines the model; Collar bindings are future)
- ❌ No app PD caps (Linen, Quil, SexAudio, Theremin still denied)

---

## 4. Range Ownership Validation

### 4.1 Ownership table

E3 adopts the key range allocation from E2 as the authoritative ownership map.
Sexstore uses this table to validate that the caller's cap range is a subset
of the owning PD's assigned range.

| Range | Owner PD | Owner name | Notes |
|-------|----------|------------|-------|
| `0x00` | (nobody) | Reserved | Always denied |
| `0x01–0x0F` | 3 | silk-shell | Shell legacy |
| `0x10–0x1F` | (future) | Theremin | Reserved, not yet granted |
| `0x20–0x2F` | (future) | SexAudio | Reserved, not yet granted |
| `0x30–0x3F` | (future) | Shell appearance | Reserved for E4+ migration |
| `0x40–0x4F` | (future) | Shell input | Reserved |
| `0x50–0x5F` | (future) | Linen | Reserved for F-track |
| `0x60–0x6F` | (future) | App storage | Reserved |
| `0x70–0x7F` | 3 | Admin | Shell-only admin |
| `0x80–0xFF` | (nobody) | Unallocated | Denied until allocated |

### 4.2 Validation rules

1. A StoreCapability's key_range must be a subset of the range owned by
   that PD in the ownership table. If not → spec error, capability not granted.
2. `StoreAdmin` cap may override range ownership (full table access).
3. Overlapping ranges across different PDs are a spec error — caught at
   plan/compile time, not runtime.
4. Key `0x00` is never ownable and never accessible.

---

## 5. Collar Integration Model

### 5.1 Current Collar state

F2_COLLAR_AUTHORITY_MAP_V1.md defines Collar as an authority mapping layer
for capability grants. It does not yet define `StoreCapability` types.
Collar's `CapabilityData` enum in `kernel/src/capability.rs` uses variants
like `Domain(u32)`, `InputRing`, and `MemLend(...)` — there is no
`StorageCap` variant yet.

### 5.2 E3 Collar spec

E3 does **not** modify Collar or kernel capability types. Instead, E3
specifies that **when** Collar gains `StoreCapability` support (future),
the mapping should be:

```
Collar CapabilityData → StoreCapability mapping:

CapabilityData::StoreRead(range)  → StoreCap { kind: Read,  range }
CapabilityData::StoreWrite(range) → StoreCap { kind: Write, range }
CapabilityData::StoreDelete(range)→ StoreCap { kind: Delete,range }
CapabilityData::StoreAdmin        → StoreCap { kind: Admin, range: FULL_TABLE }

Kernel grants these caps at boot time (kernel/src/init.rs):
  pd.grant_capability(SLOT_SEXSTORE_CAP_READ, CapabilityData::StoreRead(shell_range));
  pd.grant_capability(SLOT_SEXSTORE_CAP_WRITE, CapabilityData::StoreWrite(shell_range));
```

Until Collar has `StoreCapability` types, E3 enforcement uses the
static cap table inside sexstore (compile-time configured). The two
models are compatible: when Collar gains storage types, the static table
can be replaced with a runtime table populated from Collar grants.

### 5.3 Collar integration invariants

1. Collar never stores raw cross-PD pointers — all references are capability
   descriptors. StoreCapability follows this pattern (key range, not pointer).
2. StoreCapability grants flow through the same `grant_capability()` path
   as existing capabilities.
3. StoreCapability does not introduce new kernel ABI — it extends the
   `CapabilityData` enum and reuses PDX call dispatch.
4. `StoreAdmin` is never grantable to non-system PDs.

---

## 6. Proof Markers (spec for E3)

E3 does not implement markers (deferred to E7). But E3 specifies the
capability-specific markers that E7 will emit:

```
[store.capability.deny]   seq=N key=K caller=C reason=no_cap|wrong_kind|out_of_range
[store.capability.check]  seq=N key=K caller=C result=allow|deny
```

### 6.1 Deny reasons

| Reason | Meaning |
|--------|---------|
| `no_cap` | Caller PD has no StoreCapability entry at all |
| `wrong_kind` | Caller has a cap but wrong operation kind (e.g., Read for PUT) |
| `out_of_range` | Caller has a cap but key is outside granted range |
| `admin_only` | Operation requires StoreAdmin |

### 6.2 Allow marker refinement

Existing `[sexstore.kv.put]` / `[sexstore.kv.get]` will be extended with
capability status in E7:

```
[sexstore.put.allow]  seq=N key=K caller=C range_ok=1
[sexstore.get.allow]  seq=N key=K caller=C range_ok=1
```

---

## 7. E3 Enforcement — Implementation Model

### 7.1 What changes in sexstore (when E3 is implemented)

The current sexstore dispatch:

```rust
let caller = msg.caller_pd as u64;
match msg.type_id {
    OP_KV_PUT => { /* serve unconditionally */ }
    OP_KV_GET => { /* serve unconditionally */ }
    _ => { kv_reply(caller, 0); }
}
```

E3 version:

```rust
let caller = msg.caller_pd as u64;
if !cap_check(caller, required_kind, key) {
    kv_reply(caller, KV_DENIED);
    // [store.capability.deny] seq=N key=K caller=C reason=...
    return;
}
match msg.type_id {
    OP_KV_PUT => { /* serve with cap validated */ }
    OP_KV_GET => { /* serve with cap validated */ }
    _ => { kv_reply(caller, KV_UNSUPPORTED_OP); }
}
```

Where `cap_check()` walks `STORE_CAPS` and returns true if:
- `caller_pd` matches a cap entry
- Operation maps to allowed kind (GET→Read, PUT→Write)
- `key` is within the cap's `key_range`

### 7.2 Implementation order

1. Add `StoreCapability` type + `STORE_CAPS` static table to sexstore
2. Add `cap_check()` helper
3. Call `cap_check()` at top of dispatch, before any operation
4. Add budgeted `[store.capability.deny]` markers
5. Build + verify

### 7.3 E3 implementation is safe

- **No kernel changes** — capability table lives inside sexstore
- **No ABI changes** — `STORE_CAPS` is an internal sexstore static
- **No PDX changes** — `caller_pd` is already in every message
- **No sex-pdx changes** — no new constants needed
- **Minimal code** — ~30 lines: type defs + table + cap_check + deny markers
- **Reversible** — remove table + cap_check call to revert to cap-less mode

---

## 8. What E3 Does Not Cover

| Feature | Status | Reason |
|---------|--------|--------|
| Runtime grant/revoke | ❌ Not in E3 | Cap table is compile-time static. E4+ may add runtime. |
| Per-key granularity | ❌ Not in E3 | Range-based only. Per-key is overkill for current slot count (16). |
| StoreDelete capability | ❌ Deferred to E6 | DELETE opcode doesn't exist yet. |
| App PD caps | ❌ Denied | Linen/Quil/SexAudio/Theremin not granted caps in E3. |
| Capability promotion (StoreRead→StoreWrite) | ❌ Not defined | Each kind is separate. No implicit upgrade. |
| Capability delegation | ❌ Not defined | PD may not delegate its storage cap to another PD. |
| Collar `CapabilityData` integration | ❌ Spec-only | Collar does not yet have `StoreCapability` variants. Static table suffices. |

---

## 9. STOP FIRST Conditions

| Condition | Action |
|-----------|--------|
| Faking caller identity (using caller-supplied value instead of `msg.caller_pd`) | **STOP** — kernel-authoritative identity is the only acceptable source |
| Hardcoding caller_pd checks without using domain IDs from msg.caller_pd | **STOP** — must use kernel-provided identity |
| Adding Collar `StoreCapability` types before Collar model supports them | **STOP** — E3 uses static table; Collar integration is future |
| Granting storage caps to app PDs (Linen, Quil, SexAudio, Theremin) | **STOP** — E3 grants only shell + kernel |
| Using StoreAdmin as a granularity escape (granting full table to non-admin PD) | **STOP** — StoreAdmin is kernel + shell admin only |
| Runtime cap table mutation without proof marker | **STOP** — every grant/revoke must produce a proof marker (E7+) |
| Capability check bypass for any opcode | **STOP** — every opcode must go through cap_check |
| Kernel/ABI/syscall changes for storage capability | **STOP** — E3 requires no kernel changes |
| sex-pdx constant additions | **STOP** — E3 requires no sex-pdx changes |

### 9.1 Review gate status

> ✅ **Caller identity:** sexstore observes `msg.caller_pd` natively.
> No identity faking. No ABI/kernel change needed. E3 is not blocked.

> ✅ **E3 passes its own gate.** Docs/spec only. No code changed.

---

## 10. Files Referenced

| File | Relevance |
|------|-----------|
| `servers/sexstore/src/main.rs` | Current dispatch; `msg.caller_pd` at line 68 |
| `crates/sex-pdx/src/lib.rs` | `PdxMessage` struct with `caller_pd: u32` field |
| `kernel/src/init.rs` | Current cap grant (`SLOT_SEXSTORE` → silk-shell) |
| `kernel/src/capability.rs` | `CapabilityData` enum (no StoreCap variant yet) |
| `docs/PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` | StoreCapability kinds and ownership model (§10) |
| `docs/handoff/E2_STORAGE_PROTOCOL_SPEC_V1.md` | Key namespace, value envelope, operation model |
| `docs/handoff/E1_STORAGE_BOUNDARY_AUDIT_V1.md` | Current storage topology audit |
| `docs/handoff/F2_COLLAR_AUTHORITY_MAP_V1.md` | Collar authority model (no StoreCapability yet) |

---

## 11. Next Phase: E4_SEQUENCE_SCHEMA_VERSION_V1

This is the **first phase that may require sexstore code changes**.

Scope:
- StorageVersion (monotonic version counter in sexstore metadata)
- Sequence_id generation for proof markers (monotonic u32 per boot)
- Value envelope enforcement (validate type_class + version on PUT)
- Migration rules for schema changes
- Key range migration plan (move `0x01` → `0x30+`)

E4 depends on:
- E3 (capability policy) — capability table is prerequisite for schema version checks
- E2 (protocol spec) — value envelope, key namespace

---

## Appendix A: Caller Identity — Full Trace

```
1. silk-shell (domain 3) calls:
     pdx_call(SLOT_SEXSTORE, OP_KV_PUT, key=0x01, value=blob, 0)
   
2. Kernel syscall 0 handler:
   - Looks up SLOT_SEXSTORE in silk-shell's cap table
   - Finds CapabilityData::Domain(8) → sexstore is domain 8
   - Routes message to sexstore's message_ring
   - Writes caller_pd = 3 (silk-shell's domain ID) into PdxMessage
   
3. sexstore reads:
     let msg = pdx_listen_raw(0);
     let caller = msg.caller_pd;  // == 3 (kernel-written, not falsifiable)
   
4. E3 cap_check:
     cap_check(caller=3, kind=Write, key=0x01)
       → walks STORE_CAPS
       → finds entry: holder_pd=3, kind=Write, range=0x01-0x0F
       → 3 == 3 ✅, Write == Write ✅, 0x01 in [0x01, 0x0F] ✅
       → returns true (allow)
```

The caller PD ID is kernel-authoritative at every step. No caller-supplied
field influences it. The only way to change `caller_pd` is to be a different
PD (different domain ID) or have the kernel route differently.

## Appendix B: Static Cap Table vs Collar — Coexistence Model

```
E3 (static table inside sexstore):
  sexstore/src/main.rs:
    static STORE_CAPS: [StoreCapability; N] = [ ... ];
    fn cap_check(caller_pd, kind, key) → bool;

  Simple, no kernel changes, no ABI changes.
  Works immediately.

Future (Collar grants populate sexstore table):
  kernel/src/init.rs:
    pd.grant_capability(SLOT_STORE_READ, CapabilityData::StoreRead(range));
  
  sexstore receives grant notification → inserts into runtime cap table.
  sexstore's cap_check() reads runtime table instead of static table.

  Migration path:
    1. E3: static table only.
    2. E4+: add runtime table alongside static table.
    3. Future: remove static table, use Collar grants only.
  
  Both models use the same cap_check() interface — only the table source changes.
```
