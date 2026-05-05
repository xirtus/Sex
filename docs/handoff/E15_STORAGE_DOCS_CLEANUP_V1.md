# E15_STORAGE_DOCS_CLEANUP_V1

**Status:** Implemented. Docs changed only.

**Date:** 2026-05-05

**Review gate:** "Accept only if docs-only and the storage canon becomes unambiguous without implying sexstore is the full future object/package store."

---

## Summary

Corrected `docs/manual_servers.md` to match the storage namespace canon established in STORAGE_NAMESPACE_AUDIT_V1. No code changes. No renames. No kernel/sex-pdx edits.

**Files changed (1):**
- `docs/manual_servers.md` — corrected sexshop (§7) and sexstore (§14) sections, dependency map (§17)

**Files created (1):**
- `docs/handoff/E15_STORAGE_DOCS_CLEANUP_V1.md` — this handoff document

---

## 1. Canon Established

| Server | Role | Status | Built? | Spawned? | PDX Slot |
|--------|------|--------|--------|----------|----------|
| **sexstore** | Bounded system-settings K/V | ✅ Active (E4–E14) | ✅ `sexos_build_spec.toml` | ✅ Domain 8 | `SLOT_SEXSTORE=10` |
| **sexshop** | Future object/package store | ⬜ Placeholder — not built, not spawned | ❌ Not in spec | ❌ Not in init.rs | ❌ No slot |

Key rules:
- sexstore is the **active** storage server for shell scene settings, audio policy, and system configuration
- sexshop is a **future placeholder** — dead code with POSIX assumptions, no runtime presence
- **Do not implement or route storage work through sexshop until a separate sexshop design gate exists**
- E13 durable backend is a **RAM-backed BSS scaffold** — not real persistent storage
- sexstore is NOT the full future object/package store — that role belongs to sexshop (future)

---

## 2. Corrections Made

### 2.1 Section 7: sexshop (Object & Package Store)

| Before | After |
|--------|-------|
| **Phase:** Phase 20+ (replaces `sexstore`) | **Phase:** Placeholder — not built, not spawned, no PDX slot |
| Purpose describes active lock-free object store | Purpose states "Aspirational design for a future object and package store. **Not implemented.**" |
| No disclaimer about status | Added: "Do not implement or route storage work through sexshop until a separate sexshop design gate exists." |
| No reference to actual active server | Added pointer to sexstore (§14) for current storage needs |

### 2.2 Section 14: sexstore (Legacy Object Store → System Settings K/V)

| Before | After |
|--------|-------|
| **Title:** sexstore / sexstore-gui — Legacy Object Store | **Title:** sexstore — System Settings K/V |
| **Phase:** Deprecated (replaced by sexshop in Phase 20) | **Phase:** Active (E4–E14) |
| "Both are empty stub loops" | Full capabilities table (15 features across E4–E14) |
| "New code should use sexshop" | "sexstore is the active, built, spawned storage server" |
| No opcode documentation | Opcode table: OP_KV_GET (0xB0), OP_KV_PUT (0xB1), OP_KV_DEL (0xB2) |
| No durable documentation | E13 durable backend description with scaffold disclaimer |
| No historical context | Historical note about "n" naming in older docs |
| Reference to non-existent `sexstore-gui` | Removed — no such directory or server exists |

### 2.3 Section 17: Dependency Map

| Before | After |
|--------|-------|
| `sexstore ─────────→ (deprecated stub)` | `sexstore ─────────→ silk-shell (GET/PUT/DEL via PDX slot 10)` |
| `sexstore-gui ─────→ (deprecated stub)` | Removed (no such server) |

---

## 3. Content Preserved

The following content was **kept** (updated with placeholder prefix):

- **Store Protocol enum** (aspirational — shows intended design for future sexshop) — kept as reference
- **Storage Paths table** (aspirational VFS paths) — kept as reference
- **Transaction System** (aspirational WAL design) — kept as reference
- **Object Cache** (aspirational cache design) — kept as reference
- **Outbound PDX Calls** (aspirational sexfiles/sexnet integration) — kept as reference

These describe a FUTURE design that has not been implemented. They are preserved for when sexshop has a real implementation plan, with the new preamble making clear they are aspirational.

---

## 4. Non-Targets (intentionally not changed)

| Document | Reason |
|----------|--------|
| `PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` | Old "n" references are historical — not actively wrong |
| `THEREMIN_SYSTEM_SOUND_ENGINE_PLAN_V1.md` | Uses "sexstore K/V" correctly throughout |
| `SEXAUDIO_HARP_PHASE_PLAN_V1.md` | Uses "sexstore K/V" correctly throughout |
| `B_APP_LAUNCH_SESSION_RESTORE_PLAN_V1.md` | Refers to sexshop as "future, G gate" — correct |
| `LINEN_DOCUMENT_LIFECYCLE_PLAN_V1.md` | References are correct and aspirational |
| Any code file | **No code changed** — E15 is docs-only |

---

## 5. Build Result

```
[SEXOS ENTRYPOINT] success
Build complete: sexos-v1.0.0.iso
```

**Warnings:** 1 (pre-existing — unused import `SLOT_SEXSTORE` in sexstore source).
**New warnings from E15:** 0.
**Errors:** None.

**Code changed:** ❌ No — docs only (`docs/manual_servers.md`).

---

## 6. STOP FIRST Findings

| # | Condition | Check | Stop? |
|---|-----------|-------|-------|
| S1 | Changes code or renames servers | Only `docs/manual_servers.md` changed. No code. No renames. | ❌ Not triggered |
| S2 | Implies sexstore is the full future object/package store | Sexstore described as "bounded system-settings K/V". Sexshop described as "future object/package store". Clear separation. | ❌ Not triggered |
| S3 | Claims real disk persistence for E13 | E13 section states "RAM-backed scaffold" and "NOT real persistent storage". | ❌ Not triggered |
| S4 | Requires kernel, sex-pdx, or build changes | No kernel, sex-pdx, or build spec modified. | ❌ Not triggered |
| S5 | Removes historical context or aspirational design | Preserved sexshop design content (marked aspirational). Added historical note about "n" naming. | ❌ Not triggered |
| S6 | Build fails or introduces warnings | Build passes cleanly. No new warnings. | ❌ Not triggered |

**All STOP FIRST conditions pass.**

---

## Appendix A: Files Referenced

| File | Role |
|------|------|
| `docs/manual_servers.md` | Target file — corrected |
| `docs/handoff/STORAGE_NAMESPACE_AUDIT_V1.md` | Canon reference — sexstore vs sexshop |
| `docs/handoff/E14_DURABLE_BACKEND_NEGATIVE_TEST_AUDIT_V1.md` | E13 scaffold documentation |
| `servers/sexstore/src/main.rs` | Actual active server (confirmed by build) |

## Appendix B: Verification

```bash
# Verify canon in corrected file
rg 'sexshop|sexstore|deprecated|replaces' docs/manual_servers.md

# Verify build with no code changes
make 2>&1 | grep "ENTRYPOINT"
```

---

*End of E15_STORAGE_DOCS_CLEANUP_V1.md*