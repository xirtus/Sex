# BELL_READER_CAP_CLEANUP_V1

**Status:** Cleanup complete. All temporary proof scaffolds removed.
**Build:** `[SEXOS ENTRYPOINT] success`
**Date:** 2026-05-05

---

## 1. Files Changed

| File | Change | Type |
|------|--------|------|
| `kernel/src/init.rs` | Removed temporary seed/positive/negative proof scaffolds (~52 lines) | Cleanup |
| `docs/handoff/BELL_READER_CAP_CLEANUP_V1.md` | This document | Handoff |

**Not changed:** sexbell, sex-pdx, silk-shell, SilkBar, sexdisplay, storage, limine.cfg, sexos_build_spec.toml

---

## 2. Scaffolds Removed

Three temporary kernel enqueues removed from `kernel/src/init.rs` (lines 184-235):

| # | Type | caller_pd | Marker | Removed? |
|---|------|-----------|--------|----------|
| 1 | OP_BELL_NOTIFY (seed) | 0 (kernel) | `[kernel.sexbell.cap.seed]` | ✅ |
| 2 | OP_BELL_LIST (positive) | 3 (silk-shell) | `[kernel.sexbell.cap.positive]` | ✅ |
| 3 | OP_BELL_LIST (negative) | 2 (sexdrive) | `[kernel.sexbell.cap.negative]` | ✅ |

All associated `MessageType::IpcCall` construction, arg0 packing, and `use` imports removed.

---

## 3. Preserved: silk-shell SLOT_BELL Routing Cap

```rust
// Line 107-112 (unchanged)
if sexbell_id != 0 {
    pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(sexbell_id));
    serial_println!("[kernel.sexbell.cap.shell] shell→bell slot=12");
}
```

Silk-shell retains SLOT_BELL = 12 routing cap to sexbell's message ring. This is a permanent grant, not a scaffold.

---

## 4. Preserved: Sexbell Self-Cap

```rust
// Lines 178-180 (unchanged)
pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(sexbell_id));
serial_println!("[kernel.sexbell.cap] self slot=...");
```

---

## 5. Preserved: Sexbell OP_BELL_LIST Allowlist

```rust
const BELL_LIST_ALLOWLIST: &[u32] = &[
    3,  // silk-shell (domain 3, policy owner)
];
```

With `[bell.readcap.allow]` and `[bell.readcap.deny]` markers intact.

---

## 6. Denied Callers Still Absent from Cap Grants

| Server | Domain | SLOT_BELL granted? | Status |
|--------|--------|-------------------|--------|
| sexdisplay | 1 | ❌ | Not granted |
| sexdrive | 2 | ❌ | Not granted |
| silk-shell | 3 | ✅ | Granted (reader) |
| sexinput | 4 | ❌ | Not granted |
| sexusb | 5 | ❌ | Not granted |
| silkbar | 6 | ❌ | Not granted |
| linen | 7 | ❌ | Not granted |
| sexstore | 8 | ❌ | Not granted |
| quil | 9 | ❌ | Not granted |
| sexbell | 10 | ✅ Self-cap only | Granted (self) |

---

## 7. Verification: No Temporary Reader-Cap Enqueue Remains

```bash
$ rg -n "kernel\.sexbell\." kernel/src/init.rs
111: [kernel.sexbell.cap.shell] shell→bell slot=12
180: [kernel.sexbell.cap] self slot=12
# Only permanent cap grants. No seed/positive/negative test enqueues.
```

---

## 8. Build Result

```
[SEXOS ENTRYPOINT] success
```

---

## 9. Runtime Proof Convention

All future QEMU proofs use **`./qemuX.sh`** — patched QEMU with XHCI/HID fixes, `-M q35,i8042=off`, USB-only input, `-display sdl`.

---

## 10. Next Phase

**BELL_READER_CAP_FREEZE_V1** — Docs/audit freeze of Bell Phase 4 (reader cap). Lock the allowlist design, SLOT_BELL routing grant, and read-cap enforcement contract. No further changes to reader authority before next planned phase.

---

*End of BELL_READER_CAP_CLEANUP_V1.md*
