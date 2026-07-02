# DEAD_PDX_POINTER_REPLY_GUARD_V1

**Status:** Implemented (comments/guards only — no behavior change).

**Date:** 2026-05-06

---

## Summary

Added DO NOT ENABLE guard comments at all unsafe pointer-to-stack `pdx_reply`
callsites in dead/non-workspace servers. Added stale API notes for sexshop and sext.
No runtime behavior changes. Build passes.

---

## 1. Server Classification

| Server | In Workspace? | In Build Spec? | Status |
|--------|---------------|----------------|--------|
| sexbell | ✅ Yes | ✅ Yes | LIVE |
| sexstore | ✅ Yes | ✅ Yes | LIVE |
| sexshop | ✅ Yes | ❌ No | Placeholder (stale API — does not compile) |
| sexnode | ❌ No | ❌ No | Dead (Phase 15+, pointer-to-stack) |
| sexc | ❌ No | ❌ No | Dead (Phase 19+, pointer-to-stack) |
| sext | ❌ No | ❌ No | Dead (Phase 19+, stale API) |
| sex-ld | ❌ No | ❌ No | Dead (Phase 21, pointer-to-stack) |
| sexgemini | ❌ No | ❌ No | Dead (not in workspace) |
| sexnet | ❌ No | ❌ No | Dead (not in workspace) |
| sexfiles | ❌ No | ❌ No | Dead (not in workspace, pointer wrapper) |
| crates/silkbar | ❌ No | ❌ No | Dead (not in workspace) |
| servers/sexdrive | ❌ No | ❌ No | Dead (workspace builds apps/sexdrive) |

---

## 2. Dangerous Callsites Guarded

| File | Line | Pattern | Guard Added |
|------|------|---------|-------------|
| `servers/sexnode/src/main.rs` | 81 | `pdx_reply(..., &reply as *const _ as u64)` | ✅ `// DO NOT ENABLE: pointer reply lifetime invalid under current PDX model.` |
| `servers/sexnode/src/main.rs` | 86 | `pdx_reply(..., &reply as *const _ as u64)` | ✅ (same) |
| `servers/sexc/src/main.rs` | 68 | `pdx_reply(..., &reply as *const _ as u64)` | ✅ (same) |
| `servers/sexc/src/main.rs` | 73 | `pdx_reply(..., &reply as *const _ as u64)` | ✅ (same) |
| `servers/sex-ld/src/main.rs` | 54 | `pdx_reply(..., &reply as *const _ as u64)` | ✅ (same) |
| `servers/sexfiles/src/pdx.rs` | 5 | `pdx_reply(caller, msg as *const _ as u64)` | ✅ `// WARNING: DO NOT USE with stack-local MessageType — pointer lifetime invalid` |

---

## 3. Stale API Notes Added

| File | Issue | Guard Added |
|------|-------|-------------|
| `servers/sexshop/src/main.rs` | Uses `event.num` — `PdxMessage` has `.type_id` not `.num` | ✅ `// NOTE: Stale API — PdxMessage has .type_id not .num.` |
| `servers/sext/src/main.rs` | Uses `Message::from_u64()` and `.msg_type()` — not in sex-pdx | ✅ `// NOTE: Stale API — Message::from_u64 and Message::msg_type do not exist` |

---

## 4. Files Changed

| File | Change | Type |
|------|--------|------|
| `servers/sexnode/src/main.rs` | +2 guard comments | Comment only |
| `servers/sexc/src/main.rs` | +2 guard comments | Comment only |
| `servers/sex-ld/src/main.rs` | +1 guard comment, whitespace fix | Comment only |
| `servers/sexfiles/src/pdx.rs` | +3 line warning block on `vfs_pdx_reply` | Comment only |
| `servers/sexshop/src/main.rs` | +2 line stale API note | Comment only |
| `servers/sext/src/main.rs` | +2 line stale API note | Comment only |

**Total:** 6 files changed, 13 insertions, 1 deletion (whitespace). **No code behavior changes.**

---

## 5. Build Result

```
[SEXOS ENTRYPOINT] success
```

No new warnings (all patched files are outside workspace — workspace build unchanged).

---

## 6. Future Safe Rewrite Path

For any server that wants to use structured replies instead of raw integers:

### Option A: Integer Handle Reply (RECOMMENDED)
```
// Sender allocates from a per-caller handle table, returns handle.
let handle = self.handle_table.insert(reply_data);
pdx_reply(caller_pd, handle as u64);
// Caller sends subsequent PDX call with handle to retrieve data.
```

### Option B: Caller-Owned Buffer Contract
```
// Caller provides a buffer capability in the request message.
// Sender writes reply data into the caller's buffer via capability.
// Reply value = 0 (success) or error code.
// Requires: shared-memory capability grant between domains.
```

### Option C: Static/Owned Response Arena
```
// Pre-allocated static response ring (similar to message ring).
// Sender writes reply into the ring, returns offset/index in reply value.
// Caller reads from ring at that offset.
// Requires: pre-negotiated ring capacity, bounded entries.
```

### Option D: Zero-Copy PDX Capability Design (FUTURE)
```
// PDX protocol extension: reply value is a capability slot index.
// Kernel mediates access: sender grants temporary read cap on a data slot.
// Requires: kernel IPC capability model extension.
```

**Current codebase standard for live servers:** Raw integer/status replies (Options A-lite).
The 5 pointer-to-stack replies should be converted to Option A when those servers
are redesigned for workspace inclusion.

---

## Appendix A: Verification

```bash
rg 'pdx_reply.*&.*as \*const' --type rust
# Expected: 5 matches in dead servers (sexnode 2, sexc 2, sex-ld 1)
# Plus 1 match for vfs_pdx_reply in sexfiles
```

```bash
./scripts/entrypoint_build.sh
# Expected: success
```

---

*End of DEAD_PDX_POINTER_REPLY_GUARD_V1.md*
