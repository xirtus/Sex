# SPINDLE_SEXFILES_HISTORY_V1

**Date:** 2026-05-06
**Status:** In-memory history proven — SexFiles persistence pending kernel spawn
**Contract:** docs/handoff/SPINDLE_APP_CONTRACT_V1.md
**Previous:** SPINDLE_NATIVE_COMMAND_DISPATCH_V1
**Next:** SPINDLE_BELL_EVENTS_V1

---

## Summary

Added in-memory command history ring with honest persistence status:
- Fixed 128-entry ring buffer (128 × 256 bytes = 32 KiB BSS)
- Commands pushed to history on Enter
- `history` command displays recent commands (newest first)
- `history clear` command resets the ring
- SexFiles persistence is **pending** — honest status reported

---

## Persistence Status: PENDING

**No safe VFS client path exists.** Spindle is not kernel-spawned and cannot make PDX calls to SexFiles.

When `history` is invoked, the output includes:
```
history persistence pending SexFiles client bridge.
Spindle not kernel-spawned -- no PDX call to sexfiles.
```

### Exact Missing Bridge

| Component | What's Needed |
|-----------|--------------|
| Kernel spawn | Add `apps/spindle` to `kernel/src/init.rs` module_paths (STOP FIRST) |
| PDX slot | Add `SLOT_SPINDLE` to `crates/sex-pdx/src/lib.rs` (STOP FIRST) |
| VFS call | `pdx_call(SLOT_STORAGE, OP_RAMFS_OPEN, ...)` for history file |
| History file | `/tmp/spindle/history.log` via RamFS create_with_owner |
| Read on boot | `pdx_call(SLOT_STORAGE, OP_RAMFS_READ, ...)` to restore history |

All four are STOP FIRST. Until approved, Spindle operates in memory-only mode with no data loss risk (writes never reach disk).

---

## History Ring

```
ring: [[u8; 256]; 128]  ← 32 KiB BSS
write_pos: wraps 0..127
total: monotonic u32
```

| Parameter | Value |
|-----------|-------|
| Max entries | 128 |
| Max line bytes | 256 (matches CMD_MAX) |
| Storage | Static BSS — no heap allocation |
| Overflow | Wraps; oldest entry overwritten |
| Clear | Resets ring + counters |

---

## New Commands

| Command | Output |
|---------|--------|
| `history` | Lists recent commands (newest first), then honest persistence status |
| `history clear` | Resets history ring to empty |

---

## Files Changed

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | +100 lines — History struct, history commands, proof stages 18-20 |
| `docs/handoff/SPINDLE_SEXFILES_HISTORY_V1.md` | NEW |

## Files NOT Changed

| File | Reason |
|------|--------|
| `kernel/src/init.rs` | STOP FIRST |
| `crates/sex-pdx/` | STOP FIRST |
| `servers/sexfiles/` | No protocol changes |
| `servers/silk-shell/` | No routing changes |

---

## Proof Gate (Extended to 20 Stages)

### New Stages (18-20)

| Stage | Operation | Assertion | Marker |
|-------|-----------|-----------|--------|
| 18 | Push "ver" to history, run `history` | Recognized, entry count correct | `[spindle.history.show]` |
| 19 | Run `history clear` | Ring reset, total=0 | `[spindle.history.clear]` |
| 20 | Persistence status | Always passes (honest pending) | `[spindle.history.persistence]` |

### Updated Stage 5 (Enter)

Now pushes command line to `hist` before dispatching:
```rust
hist.push(line.as_bytes());
sb.push(line.as_bytes());
let recognized = dispatch(line.as_bytes(), sb, hist);
```

---

## Build / Runtime Result

| Check | Result |
|-------|--------|
| cargo check | PASS (4 warnings) |
| entrypoint_build.sh | PASS |
| master_runtime_gate | **GREEN_MASTER** (6/6) |
| Faults | **0** |

---

## Next Prompt

```
SPINDLE_BELL_EVENTS_V1
```

---

## Contract Boundaries Preserved

- **No fake persistence** — honest pending status, no mock SexFiles calls
- **No kernel edits** — in-memory only
- **No sex-pdx ABI edits**
- **No sexfiles protocol changes**
- **No unbounded heap** — 32 KiB history ring is static BSS
- **No panic on unavailable path** — commands handle empty history gracefully
- **No raw cross-PD pointers**
