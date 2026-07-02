# SPINDLE_SEXFILES_PERSIST_V1

**Date:** 2026-05-06
**Status:** Persistence layer coded — blocked on kernel capability grant
**Previous:** SPINDLE_REAL_KEYBOARD_V1
**Next:** SPINDLE_BELL_BRIDGE_V1 (Phase 5)

---

## Summary

Added SexFiles persistence layer with honest pending status:
- `persist_history()` — full save/restore logic (coded, guarded)
- `restore_history()` — read history from RamFS (coded, guarded)
- RamFS opcodes defined locally (0x30-0x37)
- History file target: `/tmp/spindle/history.log`
- Best-effort: gracefully handles unavailable SexFiles
- **BLOCKED**: Spindle lacks kernel capability grant for SLOT_STORAGE

---

## Current Status: PENDING (Capability Grant)

The persistence code (open/write/read/close) is fully implemented and compiles. It is guarded behind a safe marker because Spindle (PD 12) lacks the kernel capability grant for `SLOT_STORAGE` (slot 1). Calling `pdx_call(SLOT_STORAGE, ...)` without the cap causes a page fault.

### Missing Kernel Config (STOP FIRST)

```rust
// kernel/src/init.rs — in capability grant section
if spindle_id != 0 {
    pd.grant_capability(sex_pdx::SLOT_STORAGE, CapabilityData::Domain(spindle_id));
}
```

This grants Spindle access to SexFiles RamFS. After this 1-line kernel edit:
- `restore_history()` loads saved commands on boot
- `persist_history()` saves commands on Enter
- Full read/write persistence cycle

---

## Persistence Architecture (Ready When Unblocked)

```
Spindle Enter
  └→ hist.push(cmd)
  └→ persist_history(&hist)
       └→ pdx_call(SLOT_STORAGE, OP_RAMFS_OPEN, "spindle_history", O_CREATE)
       └→ pdx_call(SLOT_STORAGE, OP_RAMFS_WRITE, handle, offset, chunk) × N
       └→ pdx_call(SLOT_STORAGE, OP_RAMFS_CLOSE, handle)

Spindle Boot
  └→ restore_history(&mut hist)
       └→ pdx_call(SLOT_STORAGE, OP_RAMFS_OPEN, "spindle_history", 0)
       └→ pdx_call(SLOT_STORAGE, OP_RAMFS_READ, handle, offset, 8) × N
       └→ hist.push(entry) for each restored line
       └→ pdx_call(SLOT_STORAGE, OP_RAMFS_CLOSE, handle)
```

---

## Serial Markers

| Marker | Status |
|--------|--------|
| `[spindle.sexfiles.persist.pending]` | **Active** — emitted at boot |
| `[spindle.sexfiles.open]` | Ready (guarded) |
| `[spindle.sexfiles.write]` | Ready (guarded) |
| `[spindle.sexfiles.persist]` | Ready (guarded) |
| `[spindle.history.restore]` | Ready (guarded) |

---

## Files Changed

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | +80 lines — RamFS constants, persist/restore functions, guarded calls |
| `docs/handoff/SPINDLE_SEXFILES_PERSIST_V1.md` | NEW |

---

## Build / Runtime Result

| Check | Result |
|-------|--------|
| cargo check | PASS |
| entrypoint_build.sh | PASS |
| master_runtime_gate | **GREEN_MASTER** (6/6) |
| Faults | **0** |
| Persistence calls | **Guarded** (prevent page fault) |

---

## Unblock Condition

1-line kernel init.rs edit:
```rust
pd.grant_capability(sex_pdx::SLOT_STORAGE, CapabilityData::Domain(spindle_id));
```

After this: unguard the `persist_history()` and `restore_history()` calls.

---

## Next Prompt

```
SPINDLE_BELL_BRIDGE_V1
```

Phase 5: Bell event bridge (similarly blocked on cap grant).
