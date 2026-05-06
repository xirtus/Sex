# SPINDLE_BELL_BRIDGE_V1

**Date:** 2026-05-06
**Status:** Local event ring active — Bell bridge pending kernel capability grant
**Previous:** SPINDLE_SEXFILES_PERSIST_V1
**Next:** SPINDLE_LINEN_SPN_V1 (Phase 6)

---

## Summary

Bell event bridge status update:
- Local EventRing remains active (32 entries, 4 event kinds)
- Messages updated from "not kernel-spawned" to "capability grant pending"
- Serial marker: `[spindle.bell.pending] reason=no_bell_cap`
- All Bell opcodes exist in sex-pdx (SLOT_BELL=12, OP_BELL_NOTIFY=0xC0)
- EventRing auto-records CmdOk/CmdUnknown on command dispatch

---

## Unblock Condition

```rust
// kernel/src/init.rs — 1 line
pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(spindle_id));
```

After this, unguard the `pdx_call(SLOT_BELL, OP_BELL_NOTIFY, ...)` call.

---

## Local Event Ring (Active)

| Parameter | Value |
|-----------|-------|
| Max events | 32 |
| Max line bytes | 80 |
| Kinds | CmdOk, CmdFail, CmdUnknown, Info |
| Auto-recording | Recognized → CmdOk, Unknown → CmdUnknown |
| Commands | `events`, `events clear` |

### Event Display

```
events
Event log (most recent first):
  [OK]    help
  [OK]    proof
  [????]  asdf
Bell bridge pending (capability grant pending).
```

---

## Files Changed

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | 3 lines — updated pending messages, Bell marker |
| `docs/handoff/SPINDLE_BELL_BRIDGE_V1.md` | NEW |

---

## Build / Runtime Result

| Check | Result |
|-------|--------|
| entrypoint_build.sh | PASS |
| master_runtime_gate | **GREEN_MASTER** (6/6) |
| Faults | **0** |

### Serial Log

```
[spindle.bell.pending] reason=no_bell_cap
```

---

## Next Prompt

```
SPINDLE_LINEN_SPN_V1
```

Phase 6: Linen .spn session object (similarly blocked).
