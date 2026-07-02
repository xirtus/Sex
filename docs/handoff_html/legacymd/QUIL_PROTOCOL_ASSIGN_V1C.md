# QUIL_PROTOCOL_ASSIGN_V1C

**Status:** Docs/spec only. No code changed.
**Purpose:** Assign SLOT_QUIL and OP_QUIL_PING for future route-proof implementation (V1D).

---

## 1. Current Blocker Summary

Shell → Quil PDX routing is blocked by three missing pieces:

| Blocker | Current State | Required |
|---------|---------------|----------|
| Capability slot | No SLOT_QUIL exists | silk-shell needs a slot pointing to Quil PD |
| Proof opcode | No OP_QUIL_PING exists | A type_id value Quil can match on |
| Kernel cap grant | No grant from kernel to silk-shell | `pd.grant_capability(SLOT_QUIL, Domain(quil_id))` |

---

## 2. Slot Audit: SLOT_QUIL = 11

### Existing Slot Constants

| Constant | Value | Owner |
|----------|-------|-------|
| `SLOT_STORAGE` | 1 | sexfiles VFS |
| `SLOT_SEXT` | 2 | sext demand pager |
| `SLOT_INPUT` | 3 | HID input |
| `SLOT_AUDIO` | 4 | Audio server |
| `SLOT_DISPLAY` | 5 | SexDisplay compositor |
| `SLOT_SHELL` | 6 | silk-shell orchestration |
| `SLOT_SILKBAR` | 7 | SilkBar model authority |
| `SLOT_USB_HOST` | 8 | USB host controller lease |
| *(kernel-local)* | 9 | SLOT_USB_SEXINPUT (sexusb → sexinput route, not a pub constant) |
| `SLOT_SEXSTORE` | 10 | sexstore K/V service |
| **PROPOSED: SLOT_QUIL** | **11** | **Quil PD** |

### Collision Check
- Slot 11 is **unused** in `crates/sex-pdx/src/lib.rs`
- Slot 11 is **unused** in `kernel/src/init.rs` (cap grants loop up to domain IDs, not slot numbers)
- Slot 11 is **unused** across all `servers/` and `crates/` code

**Verdict:** `SLOT_QUIL = 11` is clean. No collision.

---

## 3. Opcode Audit: OP_QUIL_PING = 0xD0

### Opcode Map (sex-pdx constants + server usage)

| Range | Usage | Owner |
|-------|-------|-------|
| `0xD0` – `0xDC` | **FREE** | — |
| `0xDD` | Compositor Commit (legacy) | purple-scanout, kernel |
| `0xDE` | Window Create (legacy) | purple-scanout, silk-shell, kernel |
| `0xDF` | Set Window Roundness (legacy) | silk-shell, kernel |
| `0xE4` – `0xE8` | Window ops | sex-pdx |
| `0xEB` – `0xEF` | Surface ops (sexdisplay) | sexdisplay main dispatch |
| `0xF0` – `0xF4` | SilkBar protocol | sex-pdx |
| `0xF5` – `0xFB` | **FREE** | — |
| `0xFC` | Appearance tokens | sex-pdx |
| `0xFD` | Surface tab info | sex-pdx |
| `0xFE` – `0x1FF` | **FREE** | — |
| `0x202` | HID event | sex-pdx (MessageType) |
| `0xB0` – `0xB1` | sexstore KV ops | sexstore |

### Proposed Value

```
OP_QUIL_PING = 0xD0
```

### Collision Check
- `0xD0` is **NOT** in `crates/sex-pdx/src/lib.rs`
- `0xD0` is **NOT** handled by any server dispatch loop
- `0xD0` is **NOT** in `kernel/src/syscalls/mod.rs`
- The `0xD0`–`0xDC` range is entirely free

**Verdict:** `OP_QUIL_PING = 0xD0` is clean. No collision. Reserved for Quil app protocol.

---

## 4. Proposed SLOT_QUIL = 11 (in sex-pdx)

```rust
/// Quil PD — app surface server (shell→Quil route, no display caps).
pub const SLOT_QUIL: u64 = 11;
```

**Intended file:** `crates/sex-pdx/src/lib.rs`, after `SLOT_SEXSTORE` (line 350).

---

## 5. Proposed OP_QUIL_PING = 0xD0 (in sex-pdx)

```rust
/// Quil proof ping — shell→Quil route verification. No display authority.
pub const OP_QUIL_PING: u64 = 0xD0;
```

**Intended file:** `crates/sex-pdx/src/lib.rs`, in the opcode section (after silkbar ops at line 96 or after surface tab info at line 100).

---

## 6. Future Kernel Grant (V1D)

In `kernel/src/init.rs`, add after existing cap grant section (after Linen grant at line ~149):

```rust
// Quil route: grant silk-shell capability to ping Quil (no display caps).
if quil_id != 0 && silkshell_id != 0 {
    if let Some(pd) = DOMAIN_REGISTRY.get(silkshell_id) {
        pd.grant_capability(sex_pdx::SLOT_QUIL, CapabilityData::Domain(quil_id));
        serial_println!("[kernel.cap.quil.route] shell->quil slot={}", sex_pdx::SLOT_QUIL);
    }
}
```

**This grant is one-way: silk-shell → Quil.** Quil receives no additional caps.

---

## 7. Caps Explicitly Not Granted

| Cap | Reason |
|-----|--------|
| SLOT_DISPLAY | Quil must not create surfaces or draw framebuffer |
| SLOT_SHELL | Quil must not send to shell (unidirectional ping) |
| SLOT_INPUT | No input handling |
| SLOT_STORAGE | No persistence |
| SLOT_AUDIO | No audio |
| SLOT_SILKBAR | No silkbar interaction |
| SLOT_SEXSTORE | No storage access |

---

## 8. Future V1D Implementation Plan

| Step | File | Change |
|------|------|--------|
| 1 | `crates/sex-pdx/src/lib.rs` | Add `pub const SLOT_QUIL: u64 = 11;` |
| 2 | `crates/sex-pdx/src/lib.rs` | Add `pub const OP_QUIL_PING: u64 = 0xD0;` |
| 3 | `kernel/src/init.rs` | Grant SLOT_QUIL cap to silk-shell |
| 4 | `servers/silk-shell/src/main.rs` | Call `pdx_call(SLOT_QUIL, OP_QUIL_PING, ...)` on Quil open/restore |
| 5 | `servers/quil/src/main.rs` | Add `OP_QUIL_PING` match arm → `[quil.route.recv]` |
| 6 | `docs/handoff/QUIL_PD_ROUTE_PROOF_V1D.md` | New handoff doc |

---

## 9. Safety Invariants (Post-V1D)

- Shell may ping Quil via `pdx_call(SLOT_QUIL, OP_QUIL_PING, 0, 0, 0)`
- Quil may receive and log receipt
- Quil still **cannot** draw framebuffer
- Quil still **cannot** create surfaces
- Shell remains sole owner of Quil surface lifecycle
- Ping is **stateless** — no reply needed, no state mutation

---

## 10. STOP FIRST Conditions (For V1D)

| Condition | Status |
|-----------|--------|
| Slot collision (`SLOT_QUIL`) | ✅ Verified free (slot 11) |
| Opcode collision (`OP_QUIL_PING`) | ✅ Verified free (0xD0) |
| Need for bidirectional caps | ✅ Not needed — one-way ping |
| Need for display/storage/input caps | ✅ Not needed |
| Sexdisplay/protocol surface changes | ✅ Not touched |
| Kernel topology conflict | ✅ Quil domain 9 already exists |
| Broad ABI redesign | ✅ Not needed |

**No STOP FIRST conditions triggered.**

---

## 11. Files Inspected

| File | Finding |
|------|---------|
| `crates/sex-pdx/src/lib.rs` | Slot 11 free, opcode 0xD0 free |
| `kernel/src/init.rs` | Quil domain 9 active, no grant slot used |
| `kernel/src/syscalls/mod.rs` | No 0xD0 reference |
| `servers/sexdisplay/src/main.rs` | No 0xD0 reference |
| `servers/silk-shell/src/main.rs` | No 0xD0 reference |
| `servers/quil/src/main.rs` | No 0xD0 reference |
| `servers/sexstore/src/main.rs` | Uses 0xB0-0xB1 only |
| `purple-scanout/src/main.rs` | Uses 0xDD-0xDE only |

---

## References

- `QUIL_SURFACE_STUB_V1A.md` — shell-side lifecycle proof
- `QUIL_PD_SPAWN_V1B.md` — Quil PD boot (domain 9, no caps)
- `crates/sex-pdx/src/lib.rs` — slot constants (lines 342-350), opcode constants (lines 85-103)
- `kernel/src/init.rs` — existing cap grants (lines 86-149)
