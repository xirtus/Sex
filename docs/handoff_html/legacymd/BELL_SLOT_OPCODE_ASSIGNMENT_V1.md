# BELL_SLOT_OPCODE_ASSIGNMENT_V1

**Status:** Complete — sex-pdx constants assigned. No spawn, no server, no cap grants.
**Build:** `[SEXOS ENTRYPOINT] success` — ISO produced.
**Date:** 2026-05-05
**Depends on:** `BELL_NAMESPACE_COLLISION_AUDIT_V1.md` (audited placeholder IDs)

---

## Summary

Assigned Bell PDX namespace constants in `crates/sex-pdx/src/lib.rs`. Constants only — no Bell server, no spawn, no cap grants, no kernel edits, no behavior.

### Constants Added

#### Slot

```rust
pub const SLOT_BELL: u64 = 12;
```

#### Opcodes (range 0xC0-0xC7)

| Constant | Value | Direction | Description |
|----------|-------|-----------|-------------|
| `OP_BELL_NOTIFY` | 0xC0 | App → Bell | Request to create a BellEvent |
| `OP_BELL_CLOSE` | 0xC1 | App/Shell → Bell | Dismiss/close an existing event by ID |
| `OP_BELL_ACTION` | 0xC2 | App/Shell → Bell | Execute an action callback on an event |
| `OP_BELL_LIST` | 0xC3 | Shell → Bell | List current events (summary, no private content) |
| `OP_BELL_CLEAR` | 0xC4 | Shell → Bell | Clear events in a lane or all lanes |
| `OP_BELL_SUBSCRIBE` | 0xC5 | SilkBar → Bell | Subscribe to lane-summary updates (future) |
| `OP_BELL_SET_POLICY` | 0xC6 | Shell → Bell | Set per-app user policy override (future) |
| `OP_BELL_MUTE_SENDER` | 0xC7 | Shell → Bell | Mute a sender PD (future) |

#### Reserved expansion (0xC8-0xCF)

8 slots reserved for future Bell opcodes. Verified free.

---

## Files Changed

| File | Change | Type |
|------|--------|------|
| `crates/sex-pdx/src/lib.rs` | Added `SLOT_BELL=12`, 8 `OP_BELL_*` constants | Code |
| `sexos_build_spec.toml` | Updated `abi_version_hash` to match new sex-pdx | Config |
| `docs/handoff/BELL_SLOT_OPCODE_ASSIGNMENT_V1.md` | New handoff doc | Doc |

---

## Validation

### Collision Checks

| Check | Result | Evidence |
|-------|--------|----------|
| SLOT_QUIL=11 unchanged | ✅ | Still `pub const SLOT_QUIL: u64 = 11;` |
| OP_QUIL_PING=0xD0 unchanged | ✅ | Still `pub const OP_QUIL_PING: u64 = 0xD0;` |
| SLOT_BELL=12 does not collide | ✅ | Slot 12 was free (verified by namespace audit) |
| 0xC0-0xC7 not in use | ✅ | No existing OP constants in this range |
| 0xC8-0xCF free | ✅ | Reserved, no assignments |
| No existing SLOT_BELL or OP_BELL_* | ✅ | These are the first definitions |

### Build

```
[SEXOS ENTRYPOINT] success — ISO produced
```

### Scope Confirmation

| Area | Touched? | Evidence |
|------|----------|----------|
| sex-pdx constants | ✅ | SLOT_BELL=12, OP_BELL_*=0xC0-0xC7 |
| sexos_build_spec.toml hash | ✅ | Updated to match new sex-pdx hash |
| kernel init.rs spawn | ❌ | Not edited |
| kernel cap grants | ❌ | Not edited |
| servers/sexbell crate | ❌ | Not created |
| workspace Cargo.toml | ❌ | Not edited |
| sexdisplay | ❌ | Not touched |
| silk-shell | ❌ | Not touched |
| SilkBar | ❌ | Not touched |
| sexstore | ❌ | Not touched |
| Bell message structs | ❌ | Not added |
| Bell queues/ring buffers | ❌ | Not implemented |
| Any behavior | ❌ | Constants only |

---

## STOP FIRST Findings

| # | Condition | Check | Stop? |
|---|-----------|-------|-------|
| S1 | SLOT_BELL=12 already exists | ✅ Did not exist before this change | ❌ Not triggered |
| S2 | Any OP 0xC0-0xC7 already exists | ✅ All free | ❌ Not triggered |
| S3 | sex-pdx namespace convention conflicts | ✅ Consistent with SLOT_QUIL/SLOT_SEXSTORE pattern; 0xC0-0xC7 contiguous range | ❌ Not triggered |
| S4 | Constants require kernel/build/spawn edits | ✅ Constants-only; no other edits needed beyond hash update | ❌ Not triggered |
| S5 | Adding constants causes build failure | ✅ Build succeeds | ❌ Not triggered |
| S6 | OP_BELL_* names already exist with different values | ✅ First definition | ❌ Not triggered |

**All STOP FIRST conditions pass.**

---

## Sex-pdx Insertion Points

The Bell constants were added at the following locations in `crates/sex-pdx/src/lib.rs`:

```
Lines 103-113: OP_BELL_* opcodes (after OP_QUIL_PING block)
Line 368:     SLOT_BELL = 12 (after SLOT_QUIL in slot constants block)
```

---

## Next Phase Recommendation

**BELL_BOOT_SPAWN_V1** — Add sexbell to kernel init.rs spawn sequence (domain 10, PKEY 10) and grant SLOT_BELL capability. This is the next phase that crosses the kernel-edit threshold and requires explicit approval.

---

## References

- `BELL_NAMESPACE_COLLISION_AUDIT_V1.md` — namespace audit confirming placeholder IDs
- `BELL_SERVER_STUB_PLAN_V1.md` — corrected implementation plan (domain 10, PKEY 10, SLOT_BELL=12)
- `BELL_PDX_PROTOCOL_SPEC_V1.md` — protocol opcode definitions, message shapes
- `crates/sex-pdx/src/lib.rs` — assigned constants
- `sexos_build_spec.toml` — updated ABI version hash

---

*End of BELL_SLOT_OPCODE_ASSIGNMENT_V1.md*
