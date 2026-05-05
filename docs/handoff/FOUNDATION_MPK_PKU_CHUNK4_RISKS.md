# FOUNDATION MPK/PKU — Chunk 4: Development-Only Risks

Acceptable for pre-0.2 foundation. Must fix before production/security-beta.

## Risk 1 — pku_warden wrong PKRU diagnostic
- **File:** `kernel/src/pku.rs:90`
- **Problem:** `rdpkru()` called from God Mode → always prints 0x00000000
- **Fix:** Read `current_pkru_mask` from current PD instead
- **Task:** `PKU_WARDEN_DIAGNOSTIC_FIX_V1`

## Risk 2 — activate_memory_cap doesn't wrpkru
- **File:** `kernel/src/capability.rs:301-312`
- **Problem:** Updates `current_pkru_mask` but no `wrpkru` call → HW/SW desync
- **Current:** **No callers exist.** Dead code.
- **Fix:** Add `wrpkru()` after store
- **Task:** `ACTIVATE_MEMORY_CAP_WRPKRU_FIX_V1`

## Risk 3 — PKU violation panics kernel
- **File:** `kernel/src/interrupts.rs:468-474`
- **Problem:** `panic!("PKU SECURITY VIOLATION")` — kills entire system for any user violation
- **Acceptable:** Yes, Phase 31 "prove it works" behavior
- **Fix:** Domain-kill path (like user null-jump handler)
- **Task:** `PKU_VIOLATION_DOMAIN_KILL_PLAN_V1`

## Risk 4 — #GP panics kernel
- **File:** `kernel/src/interrupts.rs:648-653`
- **Problem:** User-mode GPF kills whole kernel
- **Fix:** Check CPL==3, kill domain
- **Task:** `USER_GPF_DOMAIN_KILL_PLAN_V1`

## Risk 5 — validate_core_state not wired
- **File:** `kernel/src/core_local.rs:107-113`
- **Problem:** PKRU desync check exists but has no callers
- **Priority:** Low — desync can't happen without `activate_memory_cap` calls

## Risk 6 — Framebuffer PKEY double-tagging
- **Files:** `init.rs:200-208` and `graphics/handoff.rs:24-28`
- **Problem:** Redundant paths; currently both use PKEY 1

## Risk 7 — MAX_DOMAINS (1024) > PKEY count (16)
- **File:** `capability.rs:186-217`
- **Problem:** `for_domain` guards `pkey < 16`; domains >= 16 get no self-access
- **Current:** No domains > 10 exist
- **Task:** `PKEY_ALLOCATION_BEYOND_16_PLAN_V1`
