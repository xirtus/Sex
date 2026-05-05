# E9_PRE_DURABLE_STORAGE_REAUDIT_V1

**Status:** Report only. No code changed. No architecture redesign.

**Date:** 2026-05-05

**Commit range:** `68ba28d fix(storage): E* track — sexstore protocol + E9 blocker fixes`

**Previous audit:** `docs/handoff/E9_PRE_DURABLE_STORAGE_SAFETY_AUDIT_V1.md`

---

## Executive Summary

**Verdict: GO — no critical or high findings remaining.**

| Severity | Previous Count | Fixed | Remaining | E9 Gate |
|----------|---------------|-------|-----------|---------|
| CRITICAL | 0 | — | 0 | Block E9 |
| HIGH     | 1 | 1 | 0 | Block E9 |
| MEDIUM   | 3 | 1 | 2 | Document — does not block E9 |
| LOW      | 3 | 0 | 3 | Acknowledge — no action required |

**CRITICAL-1 (bit-63 collision):** FIXED — verified in sexstore and silk-shell code.

**HIGH-1 (no shell-side status client):** FIXED — verified with `STORE_REPLY_STATUS_BIT`, `store_reply_is_status()`, named status dispatch in `handle_sexstore_get_reply()`.

**MEDIUM-3 (Esc double-dispatch):** FIXED — verified `panel_consumed` flag in HID dispatch path.

**No new critical/high findings discovered.** The remaining MEDIUM and LOW findings from the initial audit are still present but do not block E9.

---

## Blocker Fix Verification Table

### CRITICAL-1: REPLY_STATUS_BIT bit-63 collision

| Requirement | Status | Evidence |
|-------------|--------|----------|
| `store_validate_value()` rejects value & REPLY_STATUS_BIT != 0 | ✅ | `sexstore/src/main.rs:133` — `if value & REPLY_STATUS_BIT != 0 { return false; }` |
| GET can never return valid stored data with status bit set | ✅ | Bit 63 guard prevents any value with bit 63 from being stored. Checksum capped to 7 bits. |
| pack/unpack checksum masked to 0x7F on both sides | ✅ | pack: `silk-shell/src/main.rs:1448` — `& 0x7F`. unpack: `line 1463` — `& 0x7F`. sexstore validate: `line 138` — `& 0x7F`. |

### HIGH-1: Silk-shell storage status protocol client

| Requirement | Status | Evidence |
|-------------|--------|----------|
| Shell has explicit helpers for REPLY_STATUS_BIT | ✅ | `store_reply_is_status()` at line 1480, `store_reply_status()` at line 1482, `store_reply_is_value()` at line 1484 |
| Shell GET distinguishes status from value | ✅ | `handle_sexstore_get_reply()` at line 1488: dispatches on `store_reply_is_status(value)` first |
| NOT_FOUND defaults safely | ✅ | Status path logs `[shell.store.default] reason=status_reply` and returns; defaults already applied at boot |
| DENIED/INVALID/etc reject/default safely | ✅ | Same status path — all status codes log named marker and return |
| Proof markers exist | ✅ | `[shell.store.reply.status]`, `[shell.store.reply.value]`, `[shell.store.default]` |
| No broad protocol redesign | ✅ | Wire format unchanged: bit 63 = status, 0 = value. Only the client dispatch was rewritten. |

### MEDIUM-3: Esc double-dispatch (panel close + AccessZoomToggle)

| Requirement | Status | Evidence |
|-------------|--------|----------|
| `panel_consumed` flag present | ✅ | `silk-shell/src/main.rs:8114` — `let mut panel_consumed = false;` |
| All 4 panel keys (0x01, 0x02, 0x03, 0x04) set `panel_consumed = true` | ✅ | Lines 8119, 8132, 8139, 8146 |
| Atlas dispatch gated behind `if panel_consumed` | ✅ | Line 8148: `if panel_consumed { } else if ATLAS_MODE_ENABLED { ... }` |
| Normal-mode Esc zoom behavior unchanged | ✅ | When `SCENE_SETTINGS_ACTIVE == false`, `panel_consumed` stays false → full dispatch path runs |
| Atlas Esc behavior unchanged | ✅ | Same — panel must be active for consumption |

---

## Regression Scan

| Constraint | Status | Notes |
|------------|--------|-------|
| RAM-only storage preserved | ✅ | No disk/durable code added |
| No raw paths | ✅ | No path strings in sexstore or shell storage code |
| No values/content logged | ✅ | Proof markers log only metadata (caller, key, status, state, gen, reason) |
| No app storage caps | ✅ | Only silk-shell (domain 3) has SLOT_SEXSTORE cap |
| No kernel/sex-pdx changes | ✅ | commit 68ba28d touches only `servers/sexstore/` and `servers/silk-shell/` |
| OP_KV_DEL remains local | ✅ | `0xB2` defined only in `servers/sexstore/src/main.rs:27` — not in `crates/sex-pdx/src/lib.rs` |
| Build succeeds | ✅ | `[SEXOS ENTRYPOINT] success` |

### git status summary

```
 M tools/qemu          ← submodule pointer change (unrelated, not staged)
?? bx.sh               ← untracked script (unrelated)
?? qemuX.sh            ← untracked script (unrelated)
?? patches/            ← untracked directory (unrelated)
?? sexstore/*.e6bak    ← backup files (unrelated, can be removed)
?? sexstore/*.e7bak    ← backup files (unrelated, can be removed)
```

**Remaining findings from initial audit (unchanged):**

| Finding | Severity | Status |
|---------|----------|--------|
| Reply buffer depth of 1 (capability.rs:247, router.rs:36) | MEDIUM | Documented — does not block E9 |
| Hardcoded KV_SHELL_CALLER = 3 (sexstore/main.rs:111) | MEDIUM | Documented — does not block E9 |
| Reclaimed tombstoned slot keeps old generation (sexstore/main.rs:278) | MEDIUM | Documented — does not block E9 |
| PKU page table walk via HHDM without validation (pku.rs:118-244) | LOW | Acknowledge |
| caller_pd widened u32→u64→u32 (sexstore/main.rs:147) | LOW | Acknowledge |
| Raw pointer access to KV table (sexstore/main.rs:209,350,447) | LOW | Acknowledge |

---

## Ready for E9 Durable Backend Gate

### Yes — E9 can proceed

| Requirement | Status |
|-------------|--------|
| All critical/high blockers fixed | ✅ Yes |
| CRITICAL-1 (bit-63 collision) | ✅ Fixed |
| HIGH-1 (shell storage client) | ✅ Fixed |
| MEDIUM-3 (Esc double-dispatch) | ✅ Fixed |
| No new critical/high findings | ✅ Confirmed |
| Regression scan clean | ✅ No regressions |
| Build succeeds | ✅ `[SEXOS ENTRYPOINT] success` |

### E9 scope (reminder)

E9 must remain **docs/spec/gate only** — not a durable backend implementation.

1. Define persistent backend gate: requirements for adding disk-backed or session-persistent storage
2. Reference E8 redaction classes: no persistent log may store unredacted StructuralMeta+
3. Verify marker classification before any marker is persisted
4. Address MEDIUM findings (reply buffer depth, KV_SHELL_CALLER configuration, generation semantics) before implementation

### E9 gate condition

```
GO — no critical/high findings. E9 durable backend gate spec may proceed as docs-only.
```

## Files Inspected in Re-Audit

| File | Focus |
|------|-------|
| `servers/sexstore/src/main.rs` | CRITICAL-1: store_validate_value bit-63 guard, checksum masking; OP_KV_DEL local; proof markers |
| `servers/silk-shell/src/main.rs` | HIGH-1: STORE_REPLY_STATUS_BIT helpers, handle_sexstore_get_reply dispatch, pack/unpack checksum masking; MEDIUM-3: panel_consumed |
| `crates/sex-pdx/src/lib.rs` | Verify no OP_KV_DEL or sexstore protocol constants leaked |
| `docs/handoff/E9_BLOCKER_FIX_CRITICAL_MEDIUM_V1.md` | Reference for fix scope |
| `docs/handoff/E9_BLOCKER_FIX_HIGH1_SHELL_STORAGE_CLIENT_V1.md` | Reference for fix scope |
| `docs/handoff/E9_PRE_DURABLE_STORAGE_SAFETY_AUDIT_V1.md` | Baseline audit findings |
| `docs/handoff/E8_STORAGE_REDACTION_POLICY_V1.md` | Redaction classes reference |
