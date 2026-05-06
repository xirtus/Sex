# SEXSTORE_REPLY_HELPER_MIGRATION_V1

**Status:** Implemented.

**Date:** 2026-05-06

---

## Summary

Replaced sexstore's local `kv_reply()` / `kv_reply_status()` helpers (syscall 29 asm wrappers)
with the shared `sex_pdx::pdx_reply(target_pd: u32, value: u64) -> u64` from the sex-pdx crate.

No behavior change. Reply encoding (bit63 status discriminator, value layout) preserved exactly.

---

## 1. Changes

**Files changed:**

| File | Change |
|------|--------|
| `servers/sexstore/src/main.rs` | Removed local `kv_reply` / `kv_reply_status`; replaced callsites with `sex_pdx::pdx_reply` |
| `docs/handoff/SEXSTORE_REPLY_HELPER_MIGRATION_V1.md` | This handoff document |

---

## 2. Callsite Migration

**Old helper:** `unsafe fn kv_reply(target_pd: u64, val: u64)` — local inline asm syscall 29.

**New helper:** `pub fn pdx_reply(target_pd: u32, value: u64) -> u64` — shared, safe, from sex-pdx.

| Pattern | Count | Replacement |
|---------|-------|-------------|
| `kv_reply_status(caller, KV_INVALID_KEY)` | 3 | `pdx_reply(caller as u32, REPLY_STATUS_BIT \| KV_INVALID_KEY)` |
| `kv_reply_status(caller, KV_DENIED)` | 3 | `pdx_reply(caller as u32, REPLY_STATUS_BIT \| KV_DENIED)` |
| `kv_reply_status(caller, KV_OK)` | 3 | `pdx_reply(caller as u32, REPLY_STATUS_BIT \| KV_OK)` |
| `kv_reply_status(caller, KV_NOT_FOUND)` | 3 | `pdx_reply(caller as u32, REPLY_STATUS_BIT \| KV_NOT_FOUND)` |
| `kv_reply_status(caller, KV_INVALID_VALUE)` | 1 | `pdx_reply(caller as u32, REPLY_STATUS_BIT \| KV_INVALID_VALUE)` |
| `kv_reply_status(caller, status)` | 1 | `pdx_reply(caller as u32, REPLY_STATUS_BIT \| status)` |
| `kv_reply(caller, result)` | 1 | `pdx_reply(caller as u32, result)` |
| `kv_reply(caller, 0)` | 1 | `pdx_reply(caller as u32, 0)` |
| **Total** | **16** | |

---

## 3. Type Casts Needed

- `caller` is `u64` (line 503: `let caller = msg.caller_pd as u64`)
- `pdx_reply` expects `target_pd: u32`
- All callsites use `caller as u32` — safe because `msg.caller_pd` originates as `u32` from `PdxMessage.caller_pd: u32`

---

## 4. Behavior Equivalence Proof

| Aspect | Before (kv_reply) | After (pdx_reply) | Equivalent? |
|--------|-------------------|-------------------|-------------|
| Syscall number | 29 (SYSCALL_PDX_REPLY) | 29 (SYSCALL_PDX_REPLY) | ✅ Same |
| target_pd register | `rdi = target_pd (u64)` | `rdi = target_pd as u64 (u32→u64)` | ✅ Same |
| value register | `rsi = val (u64)` | `rsi = value (u64)` | ✅ Same |
| Return | none (void) | `u64` status (ignored) | ✅ No behavior change |
| Status discriminator | `REPLY_STATUS_BIT \| code` | `REPLY_STATUS_BIT \| code` | ✅ Same |
| Value reply | raw u64 | raw u64 | ✅ Same |
| Safety | `unsafe fn` | `pub fn` (safe, asm inside) | ✅ Same effect |
| Stack | `options(nostack)` | `options(nostack)` | ✅ Same |

---

## 5. Build Result

```
[SEXOS ENTRYPOINT] success
Build complete: sexos-v1.0.0.iso
```

**Warnings:** 1 (pre-existing — unused import `SLOT_SEXSTORE` in sexstore source — documented in E15).
**New warnings from this change:** 0.
**Errors:** None.

---

## 6. Remaining Duplicate Reply Helpers

| Server | Helper | Status |
|--------|--------|--------|
| `sexbell` | `bell_reply` (unsafe, syscall 29) | **Still present** — not in scope of this patch |
| `sexstore` | ~~`kv_reply` / `kv_reply_status`~~ | ✅ Removed |

The `bell_reply` in `servers/sexbell/src/main.rs:12` is a structurally identical local helper
(`unsafe fn bell_reply(target_pd: u32, val: u64)`). It should be migrated in a follow-up.

---

## 7. STOP FIRST Conditions

| # | Condition | Check | Stop? |
|---|-----------|-------|-------|
| S1 | Non-u32 target type ambiguity | `caller` is `u64` → `as u32` safe (originates as u32 from PdxMessage) | ❌ Not triggered |
| S2 | Shared helper return/status handling changes | Return value ignored (was void before, void now) | ❌ Not triggered |
| S3 | Callsites depend on side effects of kv_reply | All callsites just send reply, no side effects | ❌ Not triggered |
| S4 | Build exposes unrelated storage issues | Build passes. Only pre-existing `SLOT_SEXSTORE` unused warning. | ❌ Not triggered |

---

## Appendix A: Verification Commands

```bash
rg 'kv_reply' servers/sexstore/src/main.rs
# Expected: no output (all removed)

rg 'pdx_reply' servers/sexstore/src/main.rs
# Expected: 16 callsites + 1 use import

rg 'bell_reply' servers/sexbell/src/main.rs
# Remaining duplicate (not in scope)
```
