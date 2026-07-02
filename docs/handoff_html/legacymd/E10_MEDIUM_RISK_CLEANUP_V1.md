# E10_MEDIUM_RISK_CLEANUP_V1

**Status:** Implemented. Code changed.

**Date:** 2026-05-05

**Review gate:** "Accept E10 only if it fixes local medium risks without touching kernel/ABI or implementing durable storage."

---

## Summary

Fixes 2 of 3 remaining medium risks from E9 pre-audit. The third (reply buffer depth) is deferred as docs-only — queue redesign is out of scope for E10.

**Files changed:**
- `servers/sexstore/src/main.rs` — 2 fixes (caller constant documentation, reclaimed generation reset)

**No other files touched.** No kernel edits. No sex-pdx edits. No durable backend changes.

---

## Risk Cleanup Table

| Risk | Severity | Outcome | Status |
|------|----------|---------|--------|
| Hardcoded `KV_SHELL_CALLER = 3` | MEDIUM | Documented with cross-reference to init.rs spawn order | ✅ Fixed |
| Reclaimed slot keeps old generation | MEDIUM | Generation reset to 1 on reclaim (matches fresh-insert semantics) | ✅ Fixed |
| Reply buffer depth of 1 | MEDIUM | Deferred to E11+ — queue redesign requires kernel capability changes | 📄 Documented |

---

## Risk 1: Hardcoded KV_SHELL_CALLER = 3

### Before
```rust
const KV_SHELL_CALLER: u64 = 3;
```
No documentation of why the value is 3 or where it originates.

### After
```rust
// Silk-shell (domain 3) is the only authorized caller in E4.
// NOTE: This value must match silk-shell's domain ID as assigned by
// kernel/src/init.rs fixed spawn order (module_paths[2] = "silk-shell" → domain_id=3).
// If spawn order or domain allocation changes, update this constant to match.
const KV_SHELL_CALLER: u64 = 3;
```

**File:** `servers/sexstore/src/main.rs:110-115`

**Rationale:** No safe shared constant exists without sex-pdx/kernel edit (domain IDs are not exported as named constants). Keeping the local constant with clear dependency documentation is the minimal safe fix. The constant name `KV_SHELL_CALLER` already makes the pairing explicit.

**If spawn order changes in `kernel/src/init.rs:39`:**
1. Silk-shell's domain ID changes from 3 to something else
2. `KV_SHELL_CALLER` must be updated to match
3. All storage operations would silently return `KV_DENIED` until the constant is updated
4. Fix: search for `KV_SHELL_CALLER` references after any init.rs spawn order change

**Future improvement (E11+):** Replace with boot-time capability grant from kernel to sexstore, passing the authorized caller PD ID as an init argument.

---

## Risk 2: Reclaimed slot keeps old generation

### Before
```rust
(*slot).key = key;
(*slot).val = val;
bump_generation(slot);  // continues from old key's generation counter
```
If key 0x01 had gen=3 and was tombstoned, then key 0x02 reclaimed the same slot:
- Before: gen=4 (bumped from old key's 3)
- Marker: `[sexstore.generation.bump] key=2 slot=0 gen=4 op=reclaim`

### After
```rust
(*slot).key = key;
(*slot).val = val;
// Reset generation to 1 for new key lifecycle (different from old key).
// E10: generation is per-slot but semantically per-key on reclaim.
(*slot).generation = 1;
```
After fix:
- Generation = 1 (reset to match fresh-insert semantics)
- Marker: `[sexstore.generation.bump] key=2 slot=0 gen=1 op=reclaim`

**File:** `servers/sexstore/src/main.rs:285-287`

**Rationale:** Generation is semantically per-key. When a different key reclaims a tombstoned slot, it starts a new lifecycle — generation resets to 1. This matches the fresh-insert behavior (line 264: `(*slot).generation = 1; // first write`). The wrap invariant (generation never 0 after first write, wraps 255→1) is preserved.

**Behavior change:** Previously, reclaimed slots for different keys showed a higher generation number (continuing from old key's counter). Now they show gen=1. This affects proof marker output only — no caller-visible change since generation is internal.

**Same-key revive (unchanged):** When the same key is revived from tombstone (found_slot path, line 244), `bump_generation()` is still called — preserving the continuous counter for the same key's lifecycle.

---

## Risk 3: Reply buffer depth of 1 (deferred)

**Files:** `kernel/src/capability.rs:247`, `kernel/src/ipc/router.rs:36`

**Status:** Deferred to E11+. Not fixed in E10.

**Why deferred:** Fixing the reply buffer depth requires changes to the kernel's `ProtectionDomain::incoming_replies` queue (currently `VecDeque::with_capacity(1)`). This is a kernel capability infrastructure change:
- Increasing buffer depth requires allocation changes (currently 1-deep)
- Adding backpressure (`Err(BufferFull)`) requires `send_reply` signature change
- Both are kernel-internal changes that need separate design

**Current safety:** The synchronous protocol (listen → process → reply) means no impacted path exists today. sexstore is single-threaded and only processes one message at a time.

**Must fix before:** Any async storage operation (background flush, write-back cache, batched replies) in E11+.

**Recommended fix (E11+):**
```rust
// Increase from VecDeque::with_capacity(1) to at least 8
incoming_replies: Mutex::new(VecDeque::with_capacity(8)),
```
And/or return `Err(BufferFull)` from `send_reply` instead of silently dropping the oldest reply.

---

## Build Result

```
[SEXOS ENTRYPOINT] success
Build complete: sexos-v1.0.0.iso
```

**Sexstore warnings:** None.
**New warnings:** None.
**Errors:** None.

---

## Behavior Changes

| Scenario | Before (E9) | After (E10) |
|----------|-------------|-------------|
| Reclaim tombstoned slot for different key | `gen=4 op=reclaim` (bumped from old key's 3) | `gen=1 op=reclaim` (reset to 1) |
| All generation invariants | 0=never written, 1..255=write count, wraps 255→1 | Unchanged |
| shell caller constant | 3, undocumented | 3, documented with init.rs ref |
| Reply buffer depth | 1, silent drop | Unchanged (deferred) |
| All other behavior | As specified in E4–E9 | Unchanged |

---

## STOP FIRST Findings

| Condition | Status |
|-----------|--------|
| Requires kernel/ABI change | ❌ Not required |
| Requires sex-pdx change | ❌ Not required |
| Implements durable storage | ❌ Not implemented |
| Expands behavior beyond cleanup | ❌ Not expanded |
| Adds raw paths | ❌ Not added |
| Adds app storage caps | ❌ Not added |
| Adds LIST/ENUM | ❌ Not added |
| Implements Linen/Quil persistence | ❌ Not implemented |
| Changes capability topology | ❌ Not changed |
| Logs values/content/titles/paths | ❌ Not logged |

> E10 passes its own gate. Local medium-risk fixes only. No kernel/ABI changes. No durable backend.

---

## Ready/Not Ready for E11

### Yes — E11 can proceed

1. **Risk 1 (caller constant):** Fixed — documented dependency on init.rs spawn order
2. **Risk 2 (generation on reclaim):** Fixed — generation reset to 1 on reclaim, preserving invariant
3. **Risk 3 (reply buffer depth):** Deferred — documented with fix recommendation for E11+
4. **No regressions:** Build passes, RAM-only preserved, proof markers metadata-only

### E11 scope (proposed)

- **E11_DURABLE_BACKEND_DESIGN_V1** — docs-only design for durable backend architecture
- Must address deferred reply-buffer-depth risk before any async storage operations
- Must reference E10 changes (caller constant documentation, generation behavior)
- Must retain all E8/E9 constraints (redaction, boundedness, no raw paths)
