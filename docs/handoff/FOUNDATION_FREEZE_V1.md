# FOUNDATION_FREEZE_V1

**Status:** Foundation freeze complete. All 4 agents closed.
**Build:** `[SEXOS ENTRYPOINT] success` — ISO 1638 sectors
**Boot:** QEMU — 10/10 PDs, zero faults/panics
**Date:** 2026-05-05

---

## 1. Foundation Scoreboard

| Agent | Scope | Status | Confidence |
|-------|-------|--------|------------|
| **A** | Build/boot pipeline, ISO integrity, 10-PD spawn | **LOCKED PASS** | 97–98% |
| **B** | PD/PDX audit (slots, routes, identity, markers) | **PASS** | 93–95% |
| **C** | MPK/PKU/PKEY isolation (transitions, map, risks) | **PASS, docs complete** | 94–95% |
| **D** | WINDOWS[1] guard fix, silk-shell Vec panic removal | **PATCH VERIFIED** | 97% |

**Composite freeze confidence:** ~96%

---

## 2. What Is Frozen

### PD Spawn Table (10 domains, deterministic)

| Domain | Server | PKEY | Cap Grants |
|--------|--------|------|------------|
| 1 | sexdisplay | 1 | Primary GPU lease, framebuffer |
| 2 | sexdrive | 2 | — |
| 3 | silk-shell | 3 | SLOT_DISPLAY, SLOT_SHELL, SLOT_SILKBAR, SLOT_SEXSTORE, SLOT_QUIL, SLOT_BELL |
| 4 | sexinput | 4 | SLOT_INPUT (InputRing), SLOT_SHELL |
| 5 | sexusb | 5 | SLOT_USB_SEXINPUT→sexinput |
| 6 | silkbar | 6 | SLOT_DISPLAY |
| 7 | linen | 7 | SLOT_DISPLAY |
| 8 | sexstore | 8 | — |
| 9 | quil | 9 | — (receives shell→quil ping via SLOT_QUIL) |
| 10 | sexbell | 10 | SLOT_BELL (self-cap) |

### Slot Map (1–12, slot 9 kernel-local)

| Slot | Constant | Target |
|------|----------|--------|
| 1 | SLOT_STORAGE | sexfiles VFS |
| 2 | SLOT_SEXT | sext pager |
| 3 | SLOT_INPUT | sexinput (HID) |
| 4 | SLOT_AUDIO | audio (reserved) |
| 5 | SLOT_DISPLAY | sexdisplay |
| 6 | SLOT_SHELL | silk-shell |
| 7 | SLOT_SILKBAR | silkbar |
| 8 | SLOT_USB_HOST | XHCI lease |
| 9 | SLOT_USB_SEXINPUT | sexinput (kernel-local) |
| 10 | SLOT_SEXSTORE | sexstore |
| 11 | SLOT_QUIL | quil |
| 12 | SLOT_BELL | sexbell |

### PKEY Map (13 of 16 used)

| PKEY | Assignment | Type |
|------|-----------|------|
| 0 | Kernel | Static (USER_ACCESSIBLE=0) |
| 1–10 | sexdisplay–sexbell | Domain (1:1 with domain_id) |
| 11–13 | Free | — |
| 14 | SHARED | IPC buffers |
| 15 | UNTRUSTED | Syscall return buffer |

### Known-Good PDX Routes

| Route | Slot | Direction | Proof |
|-------|------|-----------|-------|
| silk-shell → sexdisplay | SLOT_DISPLAY=5 | pdx_call | Log: cursor draws, surface ops |
| silk-shell → silkbar | SLOT_SILKBAR=7 | pdx_call | Log: workspace/focus state |
| silk-shell → sexstore | SLOT_SEXSTORE=10 | pdx_call | Log: KV get/put |
| silk-shell → quil | SLOT_QUIL=11 | pdx_call | Log: [shell.quil.route.ping] |
| silk-shell → sexbell | SLOT_BELL=12 | pdx_call | Log: [bell.readcap.allow] |
| sexusb → sexinput | SLOT_USB_SEXINPUT=9 | pdx_call | Log: forward.mouse |
| sexbell (self) | SLOT_BELL=12 | listen | Log: bell.boot, bell.notify.* |

### Bell Phase 1–4 (Complete, Unwired)

| Phase | Scope | Status |
|-------|-------|--------|
| 1 | Boot spawn + OP_BELL_NOTIFY handler | ✅ Frozen |
| 2 | 16-entry RAM queue | ✅ Frozen |
| 3 | OP_BELL_LIST summary API (marker-only) | ✅ Frozen |
| 4 | Read-cap allowlist (silk-shell only) | ✅ Frozen |

### WINDOWS[1] Guard Fix

- **File:** `servers/silk-shell/src/main.rs`
- **Fix:** Replaced `Vec::new()` in WINDOWS hot path with static array + index guard
- **Risk:** Latent Vec panic removed (out-of-bounds would previously panic)

---

## 3. Known Backlog (Post-Freeze)

| # | Item | Severity | Owner |
|---|------|----------|-------|
| B1 | sexinput lacks `caller_pd` identity markers | Low | Future hardening |
| B2 | sexinput lacks `[pdx.opcode.unknown]` fallthrough | Low | Future hardening |
| B3 | Surface registry comment missing 0x99 Spindle | Low | Next Spindle work |
| B4 | Frame_id 8 gap undocumented | Low | Next Spindle work |
| C1 | PKU warden diagnostic prints wrong value (God Mode) | Low | PKU_WARDEN_DIAGNOSTIC_FIX |
| C2 | `activate_memory_cap` doesn't wrpkru (dead code) | Low | ACTIVATE_MEMORY_CAP_FIX |
| C3 | PKU violation panics kernel (domain-kill needed) | Medium | PKU_VIOLATION_DOMAIN_KILL |
| C4 | #GP panics kernel (user-mode domain-kill needed) | Medium | USER_GPF_DOMAIN_KILL |
| C5 | MAX_DOMAINS(1024) > PKEY count(16) | Low | PKEY_ALLOCATION_PLAN |

---

## 4. File Manifest

### Code Changes (8 files)

| File | Change |
|------|--------|
| `Cargo.toml` | Added `servers/sexbell` workspace member |
| `crates/sex-pdx/src/lib.rs` | Added SLOT_BELL=12, OP_BELL_* 0xC0-0xC7 |
| `kernel/src/init.rs` | sexbell spawn (domain 10), SLOT_BELL grants (self + silk-shell) |
| `limine.cfg` | Added quil + sexbell MODULE_PATH entries |
| `servers/silk-shell/src/main.rs` | WINDOWS[1] Vec guard fix, Spindle terminal stub |
| `sexos_build_spec.toml` | sexbell build stage, updated ABI hash |
| `tools/qemu` | Submodule commit update (patched QEMU) |
| `servers/sexbell/src/main.rs` | Bell server (notify handler, queue, list, reader cap) |

### Handoff Docs (40+ files)

All `docs/handoff/BELL_*.md`, `docs/handoff/FOUNDATION_*`, plus this document.

---

## 5. Next Steps (Post-Commit)

1. Spindle terminal (silk-shell-local, no kernel changes)
2. Bell sender cap path (wire real OP_BELL_NOTIFY sender)
3. SilkBar Bell presence (lane-summary indicator)
4. PKU violation domain-kill path
5. Storage persistence for Bell events

---

## References

- `BELL_NAMESPACE_COLLISION_AUDIT_V1.md` — namespace audit (domain 10, PKEY 10)
- `BELL_SLOT_OPCODE_ASSIGNMENT_V1.md` — SLOT_BELL=12, OP_BELL_* 0xC0-0xC7
- `BELL_PHASE1_FREEZE_V1.md` — Phase 1 freeze checkpoint
- `BELL_RAM_QUEUE_FREEZE_V1.md` — Phase 2 freeze checkpoint
- `BELL_LIST_SUMMARY_FREEZE_V1.md` — Phase 3 freeze checkpoint
- `BELL_READER_CAP_CLEANUP_V1.md` — Phase 4 cleanup (pre-freeze)
- `FOUNDATION_MPK_PKU_CHUNK1_BASELINE.md` — MPK/PKU baseline
- `FOUNDATION_MPK_PKU_CHUNK2_PKEY_MAP.md` — PKEY assignment map
- `FOUNDATION_MPK_PKU_CHUNK3_TRANSITIONS.md` — PKRU transitions
- `FOUNDATION_MPK_PKU_CHUNK4_RISKS.md` — Development-only risks
- `kernel/src/init.rs` — spawn table, cap grants
- `servers/sexbell/src/main.rs` — Bell handler
- `crates/sex-pdx/src/lib.rs` — slot/opcode constants

---

*End of FOUNDATION_FREEZE_V1.md*
