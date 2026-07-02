# BELL_NAMESPACE_COLLISION_AUDIT_V1

**Status:** Complete — docs-only audit. No code changed.
**Build:** N/A (no code changes).
**Date:** 2026-05-05
**Depends on:** `BELL_SERVER_STUB_PLAN_V1.md` (contains the INVALID proposals corrected herein)

---

## Summary

Audit of Bell's proposed domain/PKEY/slot/opcode assignments against current SexOS namespace. **Collision CONFIRMED** — Bell's proposed domain 9, PKEY 9, and SLOT_BELL=11 all collide with Quil's existing assignments.

### Collision Findings

| Bell Proposal | Current Owner | Collision? | Severity |
|---------------|---------------|------------|----------|
| Domain 9 | Quil (`kernel/src/init.rs:76-78`) | **YES** | CRITICAL — must fix |
| PKEY 9 | Quil (1:1 domain→PKEY mapping) | **YES** | CRITICAL — must fix |
| SLOT_BELL = 11 | SLOT_QUIL = 11 (`crates/sex-pdx/src/lib.rs:355`) | **YES** | CRITICAL — must fix |
| OP_BELL_NOTIFY = 0xC0 | 0xC0-0xCF range is free | **NO** — clean | PASS |
| Spawn "after sexstore, before silk-shell" | silk-shell=index 2, sexstore=index 7 | **IMPOSSIBLE** — sexstore spawns after silk-shell | Must correct |

---

## Current Namespace Tables

### Domains (1:1 with PKEYs)

Source: `kernel/src/init.rs:38-88` — fixed spawn order in `module_paths` array.

| Domain | PKEY | Server | Init.rs Index | Established |
|--------|------|--------|---------------|-------------|
| 1 | 1 | sexdisplay | 0 | Phase 25 |
| 2 | 2 | sexdrive | 1 | Devmgr |
| 3 | 3 | silk-shell | 2 | Phase 25 |
| 4 | 4 | sexinput | 3 | HID |
| 5 | 5 | sexusb | 4 | USB |
| 6 | 6 | silkbar | 5 | Bar |
| 7 | 7 | linen | 6 | Linen |
| 8 | 8 | sexstore | 7 | E4+ |
| **9** | **9** | **quil** | **8** | **V1C** |

Domain 0 = kernel (Ring 0). MAX_DOMAINS = 1024 (`kernel/src/ipc.rs:68`).

### PDX Slots

Source: `crates/sex-pdx/src/lib.rs:346-355` + `kernel/src/init.rs:5` (SLOT_USB_SEXINPUT is kernel-local).

| Slot | Name | Server | Established |
|------|------|--------|-------------|
| 1 | SLOT_STORAGE | sexfiles VFS | Phase 25 |
| 2 | SLOT_SEXT | sext demand pager | Phase 25 |
| 3 | SLOT_INPUT | HID input | Phase 25 |
| 4 | SLOT_AUDIO | audio server | Phase 25 |
| 5 | SLOT_DISPLAY | SexDisplay | Phase 25 |
| 6 | SLOT_SHELL | silk-shell | Phase 25 |
| 7 | SLOT_SILKBAR | SilkBar | Phase 25 |
| 8 | SLOT_USB_HOST | USB host controller | USB |
| 9 | SLOT_USB_SEXINPUT | kernel-local const (init.rs:5, not in sex-pdx) | USB route |
| 10 | SLOT_SEXSTORE | sexstore K/V | E4+ |
| **11** | **SLOT_QUIL** | **Quil** | **V1C** |

### Opcodes in Use

Source: `crates/sex-pdx/src/lib.rs:85-107`.

| Opcode | Name | Direction | Server |
|--------|------|-----------|--------|
| 0xD0 | OP_QUIL_PING | shell → quil | Quil |
| 0xE4 | OP_WINDOW_CREATE | shell → sexdisplay | sexdisplay |
| 0xE5 | OP_WINDOW_SUBMIT | shell → sexdisplay | sexdisplay |
| 0xE6 | OP_WINDOW_VBLANK | sexdisplay → shell | sexdisplay |
| 0xE7 | OP_WINDOW_MAP | shell → sexdisplay | sexdisplay |
| 0xE8 | OP_WINDOW_WRITE | shell → sexdisplay | sexdisplay |
| 0xF0 | OP_SILKBAR_PING | shell → silkbar | SilkBar |
| 0xF1 | OP_SILKBAR_GET_ABI | shell → silkbar | SilkBar |
| 0xF2 | OP_SILKBAR_UPDATE | shell → silkbar | SilkBar |
| 0xF3 | OP_SILKBAR_WORKSPACE_ACTIVE | shell → silkbar | SilkBar |
| 0xF4 | OP_SILKBAR_FOCUS_STATE | shell → silkbar | SilkBar |
| 0xFC | OP_APPEARANCE_TOKENS | shell → sexdisplay | sexdisplay |
| 0xFD | OP_SURFACE_TAB_INFO | shell → sexdisplay | sexdisplay |

**Free opcode ranges:** 0xC0-0xCF, 0xD1-0xE3, 0xE9-0xEF, 0xF5-0xFB, 0xFE-0xFF.

---

## Corrected Placeholder IDs for Bell

| Resource | Old (INVALID) | Corrected | Rationale |
|----------|---------------|-----------|-----------|
| Domain | 9 (Quil collision) | **10** | Next contiguous after Quil's domain 9 |
| PKEY | 9 (Quil collision) | **10** | 1:1 mapping with domain |
| SLOT_BELL | 11 (SLOT_QUIL collision) | **12** | Next after slot 11 (slot 9=kernel-local, 10=SEXSTORE, 11=QUIL) |
| Spawn order | "after sexstore, before silk-shell" (impossible) | **After quil (index 9)** | quil is currently last (index 8); Bell becomes last |
| Opcode range | 0xC0 (single, unverified) | **0xC0-0xCF** | Verified free; 0xD0=OP_QUIL_PING |

### Verification of Corrected IDs

| Check | Result | Evidence |
|-------|--------|----------|
| Slot 12 free | ✅ | Not assigned in sex-pdx or kernel |
| Domain 10 free | ✅ | Not used in init.rs spawn loop |
| PKEY 10 free | ✅ | No references in any .rs or .md |
| 0xC0-0xCF free | ✅ | Only 0xD0, 0xE4+, 0xF0+, 0xFC, 0xFD assigned |
| MAX_DOMAINS capacity | ✅ | MAX_DOMAINS=1024; domain 10 trivially within range |

### Collision Map (Before/After)

```
Before audit (INVALID):
  Domain 9 ── quil (exists)
  PKEY 9   ── quil (exists)
  SLOT 11  ── quil SLOT_QUIL=11 (exists)
  Bell proposed all three ── COLLISION

After audit (corrected):
  Domain 10 ── sexbell (future)
  PKEY 10   ── sexbell (future)
  SLOT 12   ── SLOT_BELL (future)
  Opcodes 0xC0-0xCF ── OP_BELL_* (future)
```

---

## Spawn Order Clarification

The original plan stated "Bell may provide event context to shell on boot" as rationale for spawning before silk-shell. This is **not possible** under the corrected placement:

- silk-shell is domain 3 (index 2 in `module_paths`)
- Bell would be domain 10 (index 9, after quil)

If boot-time event context to silk-shell is truly required, that would demand a fundamental spawn order redesign (Bell at domain 2 or earlier), shifting all subsequent server domains. That decision is **deferred**. For now, Bell spawns last and provides runtime-only event context.

---

## Corrective Actions

### Files to Update

| File | Action | Status |
|------|--------|--------|
| `docs/handoff/BELL_SERVER_STUB_PLAN_V1.md` | Mark domain 9/PKEY 9/SLOT_BELL=11 as INVALID; replace with domain 10/PKEY 10/SLOT_BELL=12/0xC0-0xCF | 🔧 This audit |

### Already Correct (No Change Needed)

| File | Reason |
|------|--------|
| `docs/handoff/BELL_PDX_PROTOCOL_SPEC_V1.md` | Uses TBD for all numeric values |
| `docs/handoff/BELL_EVENT_MODEL_DESIGN_GATE_V1.md` | No numeric assignments |
| `docs/handoff/BELL_CAPABILITY_POLICY_V1.md` | No numeric assignments |

---

## STOP FIRST Conditions

| # | Condition | Status |
|---|-----------|--------|
| S1 | Bell domain collides with existing domain | ✅ Fixed — domain 10 is free |
| S2 | Bell PKEY collides with existing PKEY | ✅ Fixed — PKEY 10 is free |
| S3 | Bell slot collides with existing slot | ✅ Fixed — slot 12 is free |
| S4 | Bell opcodes collide with existing opcodes | ✅ 0xC0-0xCF is free |
| S5 | Bell spawn order is possible | ✅ After quil (index 9) is feasible |
| S6 | MAX_DOMAINS supports Bell | ✅ 1024 >> 10 |
| S7 | sex-pdx needs change to confirm slot | ❌ Not yet — this audit is docs-only |
| S8 | Kernel init.rs needs change to confirm domain | ❌ Not yet — this audit is docs-only |

**All collision-related STOP FIRST conditions pass.** S7/S8 are gated for the implementation phase (BELL_SLOT_OPCODE_ASSIGNMENT_V1).

---

## References

- `BELL_SERVER_STUB_PLAN_V1.md` — contains the now-INVALID proposals being corrected
- `kernel/src/init.rs` — spawn order (line 38), domain assignments (lines 47-79)
- `crates/sex-pdx/src/lib.rs` — slot constants (lines 346-355), opcode constants (lines 85-107)
- `QUIL_PROTOCOL_ASSIGN_V1C.md` — Quil's original slot/opcode assignment
- `QUIL_STUB_CONSOLIDATION_AUDIT_V1.md` — Quil namespace confirmation
- `SEXSTORE_KERNEL_ENABLE_PLAN_V1.md` — slot 9 (SLOT_USB_SEXINPUT) documentation

---

*End of BELL_NAMESPACE_COLLISION_AUDIT_V1.md*