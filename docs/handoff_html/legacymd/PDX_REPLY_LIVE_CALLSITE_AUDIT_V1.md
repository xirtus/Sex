# PDX_REPLY_LIVE_CALLSITE_AUDIT_V1

**Status:** Complete (audit only — no code changes needed for live servers).

**Date:** 2026-05-06

---

## Summary

Comprehensive audit of all `pdx_reply()` callsites across the entire source tree,
after the shared helper fix (PDX_REPLY_HELPER_FIX_V1) and sexstore migration
(SEXSTORE_REPLY_HELPER_MIGRATION_V1).

**Live callsites (built servers):** 18 callsites across 2 servers (sexbell, sexstore).
**Dead/future callsites (not in workspace):** 22 callsites across 9 servers.
**Risky patterns found:** 5 pointer-to-stack replies — all in dead code.

**No code patches needed for live code.** All live callsites are safe.

---

## 1. Shared Helper (Definition)

| File | Line | Signature | Notes |
|------|------|-----------|-------|
| `crates/sex-pdx/src/lib.rs` | 294 | `pub fn pdx_reply(target_pd: u32, value: u64) -> u64` | Safe wrapper around syscall 29. `options(nostack)`. Returns kernel status (ignored at all callsites). |

---

## 2. Live Callsites (Built Servers)

### 2.1 sexbell — 2 callsites (LIVE, SAFE)

| Line | Context | Value | Risk |
|------|---------|-------|------|
| 578 | OP_BELL_LIST — read cap denied | `u64::MAX` (error sentinel) | ✅ Integer error, safe |
| 685 | OP_BELL_LIST — lane summary | `packed` (64-bit bitfield of lane counts + redact count) | ✅ Integer bitfield, safe |

- `caller_pd: u32` from `msg.caller_pd` — no cast needed, matches `pdx_reply` signature
- Both replies are synchronous: caller waits on `pdx_listen_raw(0)` after calling
- No `unsafe` wrapper needed (pdx_reply is safe); current `unsafe { }` blocks are for budget logging
- Comment at line 7-9 is accurate documentation of kernel convention — keep as-is

### 2.2 sexstore — 16 callsites (LIVE, SAFE, just migrated)

| Pattern | Count | Value |
|---------|-------|-------|
| `pdx_reply(caller as u32, REPLY_STATUS_BIT \| KV_*)` | 14 | Status replies (bit63 = 1 discriminator) |
| `pdx_reply(caller as u32, result)` | 1 | GET stored value (raw u64) |
| `pdx_reply(caller as u32, 0)` | 1 | Unknown opcode fallback |

- `caller: u64` from `msg.caller_pd as u64` → `caller as u32` cast (safe, originates as u32)
- All callsites inside `unsafe { }` block (needed for pointer/volatile ops, not for pdx_reply)
- See `SEXSTORE_REPLY_HELPER_MIGRATION_V1.md` for full migration details

---

## 3. Dead/Future Callsites (Not in Workspace)

### 3.1 crates/silkbar — 2 callsites (DEAD)

| Line | Value | Notes |
|------|-------|-------|
| 398 | `0` | OP_SILKBAR_REGISTER ack |
| 402 | `0` | OP_SILKBAR_UNREGISTER ack |

- `req.caller_pd` type unknown but likely `u32` from `PdxMessage`
- Both wrapped in `unsafe { }` (unnecessary — pdx_reply is safe)
- **Not in workspace** (workspace has `servers/silkbar`, which has no pdx_reply calls)

### 3.2 sexnode — 9 callsites (DEAD, 2 RISKY)

| Line | Value | Risk |
|------|-------|------|
| 53 | `res` (from pdx_call) | ✅ Integer |
| 59 | `0` | ✅ Integer |
| 63 | `0` | ✅ Integer |
| 69 | `0` | ✅ Integer |
| 74 | `0` | ✅ Integer |
| 76 | `u64::MAX` | ✅ Integer |
| **81** | **`&reply as *const _ as u64`** | 🔴 POINTER-TO-STACK — reply is `MessageType::TranslatorReply` on stack |
| **86** | **`&reply as *const _ as u64`** | 🔴 POINTER-TO-STACK — reply is `MessageType::DriverLoadReply` on stack |
| 89 | `u64::MAX` | ✅ Integer |

- **Not in workspace, not built** (Phase 15+ placeholder)
- **RISK:** Lines 81, 86 send a pointer to a stack-local `MessageType`. The pointer is only valid while the stack frame exists. If the caller reads it asynchronously, it's a use-after-free.

### 3.3 sexc — 4 callsites (DEAD, 2 RISKY)

| Line | Value | Risk |
|------|-------|------|
| 63 | `res` (from handle_posix_syscall) | ✅ Integer |
| **68** | **`&reply as *const _ as u64`** | 🔴 POINTER-TO-STACK — `MessageType::PipeReply` on stack |
| **73** | **`&reply as *const _ as u64`** | 🔴 POINTER-TO-STACK — `MessageType::ProcReply` on stack |
| 76 | `u64::MAX` | ✅ Integer |

- **Not in workspace, not built** (Phase 19+ placeholder)
- Same pointer-to-stack risk as sexnode
- Imports from `libsys::pdx` (stale path; live code uses `sex_pdx`)

### 3.4 sex-ld — 1 callsite (DEAD, RISKY)

| Line | Value | Risk |
|------|-------|------|
| **54** | **`&reply as *const _ as u64`** | 🔴 POINTER-TO-STACK — `PdxReply` on stack |

- **Not in workspace, not built** (Phase 21 placeholder)
- Uses `sex_pdx::ring::PdxReply` and `libsys::pdx::pdx_reply` (mixed imports)

### 3.5 sexshop — 1 callsite (DEAD, STALE API)

| Line | Value | Notes |
|------|-------|-------|
| 50 | `found as u64` | Discovery lookup reply |

- **In workspace but NOT in build spec** (placeholder per E15)
- Uses `event.num` and `event.caller_pd` — but `PdxMessage` has `type_id` not `num`. **Stale API — does not compile with current sex-pdx.**
- Wrapped in `unsafe { }` (unnecessary)

### 3.6 sext — 1 callsite (DEAD)

| Line | Value | Notes |
|------|-------|-------|
| 42 | `0` | Page fault ack |

- **Not in workspace, not built** (Phase 19+ placeholder)
- Uses `sex_pdx::{pdx_listen_raw, pdx_reply, Message, MessageType}`; also imports `Message::from_u64` which may not exist

### 3.7 sexgemini — 1 callsite (DEAD)

| Line | Value | Notes |
|------|-------|-------|
| 32 | `handover.pfn` | Compile request PFN handover |

- **Not in workspace, not built**
- Uses `safe_pdx_register` (may not exist in current sex-pdx)

### 3.8 sexnet — 1 callsite (DEAD)

| Line | Value | Notes |
|------|-------|-------|
| 132 | `result` | Network op result |

- **Not in workspace, not built**

### 3.9 sexfiles — 1 callsite via vfs_pdx_reply wrapper (DEAD, RISKY)

| File | Line | Value | Risk |
|------|------|-------|------|
| `servers/sexfiles/src/pdx.rs` | 5 | `msg as *const _ as u64` | 🔴 POINTER — depends on caller ensuring msg lifetime |

- `vfs_pdx_reply(caller: u32, msg: &MessageType)` — sends pointer to caller
- **Not in workspace, not built**
- Wrapper re-exports `pdx_reply` from sex-pdx alongside old `PdxRequest` type
- Risk depends on whether `msg` outlives the caller's read

### 3.10 servers/sexdrive — 1 callsite (DEAD)

| Line | Value | Notes |
|------|-------|-------|
| `servers/sexdrive/src/driver.rs:20` | `CONFIRM_SIG` (0xCAFE_BABE) | ABI handshake confirmation |

- **Not in workspace, not built** (workspace builds `apps/sexdrive` which has different source)
- Wrapped in `unsafe { }` (unnecessary)

---

## 4. Risk Summary

| Risk Level | Count | Description |
|------------|-------|-------------|
| ✅ Safe | 34 | Integer/status replies to caller_pd |
| 🔴 Pointer-to-stack | 5 | sexnode (2), sexc (2), sex-ld (1) — all dead code |
| 🔴 Stale API | 2 | sexshop uses `.num` (dead), sext uses `Message::from_u64` (dead) |

All 5 risky patterns are in **dead code not in workspace** — they are not built, not spawned, not reachable at runtime.

---

## 5. Live Code Health

| Server | Callsites | Import Source | `caller_pd` type | Cast needed? | Notes |
|--------|-----------|---------------|-------------------|--------------|-------|
| sexbell | 2 | `sex_pdx::pdx_reply` | `u32` (from `msg.caller_pd`) | No | ✅ Clean |
| sexstore | 16 | `sex_pdx::pdx_reply` | `u64` (widened from `msg.caller_pd`) | `caller as u32` | ✅ Clean |

---

## 6. STOP FIRST Items

| # | Condition | Check | Stop? |
|---|-----------|-------|-------|
| S1 | ABI/protocol change required | All live callsites use integer u64 values — no protocol change needed | ❌ Not triggered |
| S2 | Pointer reply needs ownership redesign | 5 pointer-to-stack callsites exist but all are dead code | ❌ Not triggered (flag for future) |
| S3 | Caller wait semantics unclear | sexbell/sexstore: caller waits on pdx_listen after pdx_call — standard synchronous pattern | ❌ Not triggered |
| S4 | Kernel/cap grant edits needed | No — all live servers already have their PDX slots | ❌ Not triggered |

---

## 7. Recommended Next Prompt

```
SEXBELL_BELL_REPLY_REMOVAL_V1
MISSION: Replace sexbell's remaining unsafe { pdx_reply(...) } wrappers
with bare pdx_reply(...) calls. Remove unnecessary unsafe blocks.
No behavior change. Build passes.
```

---

## Appendix: All Callsites (Full Table)

| # | File | Line | Server | Value | Live? | Risk |
|---|------|------|--------|-------|-------|------|
| 1 | crates/sex-pdx/src/lib.rs | 294 | — | definition | ✅ | — |
| 2 | servers/sexbell/src/main.rs | 578 | sexbell | `u64::MAX` | ✅ Live | ✅ Safe |
| 3 | servers/sexbell/src/main.rs | 685 | sexbell | `packed` (bitfield) | ✅ Live | ✅ Safe |
| 4 | servers/sexstore/src/main.rs | 517 | sexstore | status \| KV_INVALID_KEY | ✅ Live | ✅ Safe |
| 5 | servers/sexstore/src/main.rs | 519 | sexstore | status \| KV_DENIED | ✅ Live | ✅ Safe |
| 6 | servers/sexstore/src/main.rs | 538 | sexstore | status \| KV_INVALID_VALUE | ✅ Live | ✅ Safe |
| 7 | servers/sexstore/src/main.rs | 584 | sexstore | status \| KV_OK | ✅ Live | ✅ Safe |
| 8 | servers/sexstore/src/main.rs | 648 | sexstore | status \| status | ✅ Live | ✅ Safe |
| 9 | servers/sexstore/src/main.rs | 684 | sexstore | status \| KV_INVALID_KEY | ✅ Live | ✅ Safe |
| 10 | servers/sexstore/src/main.rs | 686 | sexstore | status \| KV_DENIED | ✅ Live | ✅ Safe |
| 11 | servers/sexstore/src/main.rs | 721 | sexstore | `result` (u64) | ✅ Live | ✅ Safe |
| 12 | servers/sexstore/src/main.rs | 733 | sexstore | status \| KV_NOT_FOUND | ✅ Live | ✅ Safe |
| 13 | servers/sexstore/src/main.rs | 741 | sexstore | status \| KV_NOT_FOUND | ✅ Live | ✅ Safe |
| 14 | servers/sexstore/src/main.rs | 781 | sexstore | status \| KV_INVALID_KEY | ✅ Live | ✅ Safe |
| 15 | servers/sexstore/src/main.rs | 783 | sexstore | status \| KV_DENIED | ✅ Live | ✅ Safe |
| 16 | servers/sexstore/src/main.rs | 830 | sexstore | status \| KV_OK | ✅ Live | ✅ Safe |
| 17 | servers/sexstore/src/main.rs | 845 | sexstore | status \| KV_OK | ✅ Live | ✅ Safe |
| 18 | servers/sexstore/src/main.rs | 853 | sexstore | status \| KV_NOT_FOUND | ✅ Live | ✅ Safe |
| 19 | servers/sexstore/src/main.rs | 864 | sexstore | `0` | ✅ Live | ✅ Safe |
| 20 | crates/silkbar/src/main.rs | 398 | silkbar | `0` | ❌ Dead | ✅ Safe |
| 21 | crates/silkbar/src/main.rs | 402 | silkbar | `0` | ❌ Dead | ✅ Safe |
| 22 | servers/sexnode/src/main.rs | 53 | sexnode | `res` | ❌ Dead | ✅ Safe |
| 23 | servers/sexnode/src/main.rs | 59 | sexnode | `0` | ❌ Dead | ✅ Safe |
| 24 | servers/sexnode/src/main.rs | 63 | sexnode | `0` | ❌ Dead | ✅ Safe |
| 25 | servers/sexnode/src/main.rs | 69 | sexnode | `0` | ❌ Dead | ✅ Safe |
| 26 | servers/sexnode/src/main.rs | 74 | sexnode | `0` | ❌ Dead | ✅ Safe |
| 27 | servers/sexnode/src/main.rs | 76 | sexnode | `u64::MAX` | ❌ Dead | ✅ Safe |
| 28 | servers/sexnode/src/main.rs | 81 | sexnode | `&reply as *const _ as u64` | ❌ Dead | 🔴 Ptr-to-stack |
| 29 | servers/sexnode/src/main.rs | 86 | sexnode | `&reply as *const _ as u64` | ❌ Dead | 🔴 Ptr-to-stack |
| 30 | servers/sexnode/src/main.rs | 89 | sexnode | `u64::MAX` | ❌ Dead | ✅ Safe |
| 31 | servers/sexc/src/main.rs | 63 | sexc | `res` | ❌ Dead | ✅ Safe |
| 32 | servers/sexc/src/main.rs | 68 | sexc | `&reply as *const _ as u64` | ❌ Dead | 🔴 Ptr-to-stack |
| 33 | servers/sexc/src/main.rs | 73 | sexc | `&reply as *const _ as u64` | ❌ Dead | 🔴 Ptr-to-stack |
| 34 | servers/sexc/src/main.rs | 76 | sexc | `u64::MAX` | ❌ Dead | ✅ Safe |
| 35 | servers/sext/src/main.rs | 42 | sext | `0` | ❌ Dead | ✅ Safe |
| 36 | servers/sex-ld/src/main.rs | 54 | sex-ld | `&reply as *const _ as u64` | ❌ Dead | 🔴 Ptr-to-stack |
| 37 | servers/sexshop/src/main.rs | 50 | sexshop | `found as u64` | ❌ Dead | ✅ Safe (stale API) |
| 38 | servers/sexgemini/src/main.rs | 32 | sexgemini | `handover.pfn` | ❌ Dead | ✅ Safe |
| 39 | servers/sexnet/src/main.rs | 132 | sexnet | `result` | ❌ Dead | ✅ Safe |
| 40 | servers/sexfiles/src/pdx.rs | 5 | sexfiles | `msg as *const _ as u64` | ❌ Dead | 🔴 Ptr (wrapper) |
| 41 | servers/sexdrive/src/driver.rs | 20 | sexdrive | `CONFIRM_SIG` | ❌ Dead | ✅ Safe (built as apps/sexdrive, different source) |

**Total: 41 callsites** (1 definition + 18 live + 22 dead/future)

---

*End of PDX_REPLY_LIVE_CALLSITE_AUDIT_V1.md*
