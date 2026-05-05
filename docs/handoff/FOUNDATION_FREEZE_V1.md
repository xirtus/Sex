# FOUNDATION_FREEZE_V1 — Pre-0.2 Baseline

**Date:** 2026-05-05
**Status:** Frozen. Foundation scope only. No 0.2 feature work.
**Next action:** Resume Spindle manual USB HID input proof.

---

## 1. Foundation Scope

This freeze captures the pre-0.2 boot isolation and shell baseline. It proves:
- Build pipeline deterministic and sealed
- 10 protection domains spawn and run
- MPK/PKU/PKEY hardware isolation active
- Silk desktop reachable (surfaces, cursor, clock)
- Silk-shell WM authoritative and listening
- No panics, #PF, #GP, or fault.kill in boot
- WINDOWS[1] array access hardened against OOB panic

Not included: Spindle USB HID input, SilkBar live updates, Bell real sender, display compositor improvements, persistent storage, audio.

---

## 2. Build/Boot Proof

| Proof | Result | Details |
|-------|--------|---------|
| `entrypoint_build.sh` | PASS | `[SEXOS ENTRYPOINT] success`, ISO 1639 sectors |
| ABI guard | PASS 6/6 | Contract + ABI hash verified |
| QEMU boot | PASS | All 10 PDs spawned, Silk desktop reached |
| PKU/MPK | PASS | CR4.PKE enabled, God Mode on kernel entry, PKRU restored on exit |
| Zero fatal exceptions | CLEAN | 0 panic, 0 #PF, 0 #GP, 0 fault.kill in boot log |
| SilkBar clock | PASS | `timer.tick.enter` continuous, clock counting |
| Sexdisplay | PASS | Cursor draw, surface updates, tokens applied |
| Linen | PASS | Placeholder fill rect rendered |

---

## 3. PD/PDX Status (Agent B, parked)

**PASS at 93-95% for foundation.** True backlog (no edits now):
- sexinput lacks `caller_pd` + unknown-opcode markers
- `SURFACE_ID_SPINDLE=0x99` registry comment missing
- `frame_id 8` gap undocumented

Stale findings (false alarms, verified by entrypoint ABI hash):
- R3 sexbell import
- unknown-opcode markers in silk-shell/silkbar
- ABI hash already verified by entrypoint build

---

## 4. MPK/PKU Status (Agent C)

**PASS at 91-93% for foundation.** See chunk docs:
- `FOUNDATION_MPK_PKU_CHUNK1_BASELINE.md` — 11 verified items
- `FOUNDATION_MPK_PKU_CHUNK2_PKEY_MAP.md` — 13 of 16 PKEYs used
- `FOUNDATION_MPK_PKU_CHUNK3_TRANSITIONS.md` — 12-entry transition matrix
- `FOUNDATION_MPK_PKU_CHUNK4_RISKS.md` — 7 dev-only risks

**Key invariant:** PKU is enforcement accelerator for sex-pdx authority, not an authority layer itself.

**Future tasks (before production):**
- `PKU_WARDEN_DIAGNOSTIC_FIX_V1`
- `ACTIVATE_MEMORY_CAP_WRPKRU_FIX_V1`
- `USER_GPF_DOMAIN_KILL_PLAN_V1`
- `PKU_VIOLATION_DOMAIN_KILL_PLAN_V1`
- `PKEY_ALLOCATION_BEYOND_16_PLAN_V1`

---

## 5. WINDOWS[1] Hardening (Agent D)

**FREEZE-CLOSED.**
- 29 direct `WINDOWS[1]` indexing lines removed from `servers/silk-shell/src/main.rs`
- Zero executable `WINDOWS[1]` accesses remaining
- All 6 access sites converted to `.get(1)` / `.get_mut(1)` with safe defaults
- Build: PASS. Boot: PASS. Fault scan: CLEAN (0 panic, 0 fault.kill, 0 #PF, 0 #GP, 0 WINDOWS regression)
- Touched only `servers/silk-shell/src/main.rs`

---

## 6. Parked Backlog

| Item | Priority | Notes |
|------|----------|-------|
| sexinput caller_pd + unknown-opcode markers | Low | No functional impact on current boot |
| SURFACE_ID_SPINDLE=0x99 doc comment | Low | Missing registry entry, not functional |
| frame_id 8 gap | Low | Undocumented gap in frame ID space |
| All Agent B true backlog | Low | No sex-pdx/kernel/ABI edits until 0.2 |

---

## 7. Files in This Freeze

```
M Cargo.toml                         (build metadata)
M crates/sex-pdx/src/lib.rs          (BELL constants, no functional change)
M kernel/src/init.rs                 (unchanged logic, backup artifacts removed)
M servers/silk-shell/src/main.rs     (WINDOWS[1] guard hardening only)
A docs/handoff/FOUNDATION_FREEZE_V1.md
A docs/handoff/FOUNDATION_MPK_PKU_CHUNK1_BASELINE.md
A docs/handoff/FOUNDATION_MPK_PKU_CHUNK2_PKEY_MAP.md
A docs/handoff/FOUNDATION_MPK_PKU_CHUNK3_TRANSITIONS.md
A docs/handoff/FOUNDATION_MPK_PKU_CHUNK4_RISKS.md
?? docs/handoff/BELL_*.md            (Bell Phase docs, unrelated)
?? docs/handoff/FIX_TOGGLE_SPINDLE_BUILD_V1.md
?? docs/handoff/N13_MESH_*.md
?? docs/handoff/USB_CURSOR_ROUTE_PROOF_V1.md
?? patches/
?? bx.sh qemuX.sh
?? kernel/src/init.rs.*.bak
?? servers/sexstore/src/main.rs.e6bak
```

No sex-pdx ABI changes. No kernel isolation weakening. No scheduler edits. No sexdisplay protocol changes.

---

## 8. Next Action

**Resume:** Spindle manual USB HID input proof.

Required context:
- `docs/handoff/USB_CURSOR_ROUTE_PROOF_V1.md`
- `claude-references/USB_STATUS.md`
- `servers/sexusb/src/main.rs`
- `servers/sexinput/src/main.rs`

---

*End of FOUNDATION_FREEZE_V1.md*
