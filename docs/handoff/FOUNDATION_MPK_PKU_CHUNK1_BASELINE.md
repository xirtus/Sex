# FOUNDATION MPK/PKU — Chunk 1: Baseline Verification

**Audit:** Agent C — MPK/PKU/PKEY structural soundness.
**Date:** 2026-05-05
**Confidence:** 91–93% (pre-0.2 acceptable; not production 1.0).

## Verified Items (all PASS)

| # | Check | Source |
|---|-------|--------|
| 1 | CR4.PKE set before user PDs | `pku.rs:28-34` in `X86Hal::init()` — step 1 of `kernel_init()` |
| 2 | PKU support CPUID-gated | `pku.rs:17-25` via `raw_cpuid::has_pku()` |
| 3 | PKRU instructions gated by `PKU_ENABLED` | All 4 stubs + `wrpkru()`/`rdpkru()` check the atomic |
| 4 | 10 domains map 1:1 to PKEYs 1–10 | domain_id == pkey; `for_domain(pkey)` derives PKRU |
| 5 | PKEY 14 = SHARED IPC | RW for sexdrive, RO for sexdisplay, No Access for others |
| 6 | PKEY 15 = UNTRUSTED syscall buffer | Kernel-owned page, USER_ACCESSIBLE but PKRU-denied to user |
| 7 | `switch_to` restores `TaskContext.pkru` | `mov eax, [rsi+0x80]` — offset verified vs struct layout |
| 8 | Syscall/IRQ/#PF/#GP stubs enter God Mode | `xor eax,ecx,edx; wrpkru` after `PKU_ENABLED` check |
| 9 | Return paths restore user PKRU | Syscall: pop saved → wrpkru; switch_to: load pkru → iretq |
| 10 | No PKEY collisions | 13 of 16 PKEYs used; no overlaps |
| 11 | No isolation weakening | Kernel pages: USER_ACCESSIBLE=0; PKEY 0 open in user PKRU is harmless |

**Architecture invariant confirmed:** PKU is an enforcement accelerator for sex-pdx authority, not an authority layer itself.
