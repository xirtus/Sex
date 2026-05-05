# QUIL_PD_ROUTE_PROOF_V1D

**Status:** Complete — built, committed.
**Build:** ISO produced, no errors.

---

## Summary

Implements shell→Quil PDX route proof using `SLOT_QUIL` (11) and `OP_QUIL_PING` (0xD0).
One-way ping only: silk-shell calls `pdx_call(SLOT_QUIL, OP_QUIL_PING, ...)` when opening
Quil; Quil receives and logs `[quil.route.recv]`. No display caps, no surface ownership shift.

---

## Files Changed

| File | Change |
|------|--------|
| `crates/sex-pdx/src/lib.rs` | Added `SLOT_QUIL = 11`, `OP_QUIL_PING = 0xD0` |
| `kernel/src/init.rs` | Granted silk-shell capability via `SLOT_QUIL` → `quil_id` |
| `servers/silk-shell/src/main.rs` | Added import + `pdx_call` in `open_quil_in_active_scene()` |
| `servers/quil/src/main.rs` | Added `OP_QUIL_PING` match arm with `[quil.route.recv]` |
| `sexos_build_spec.toml` | Updated `abi_version_hash` (sex-pdx changed) |
| `docs/handoff/QUIL_PD_ROUTE_PROOF_V1D.md` | New handoff doc |

---

## Changes Detail

### 1. `crates/sex-pdx/src/lib.rs` — Constants

After `SLOT_SEXSTORE` (line 350):
```rust
pub const SLOT_QUIL: u64 = 11;    // Quil app surface server (shell→Quil route, no display caps)
```

After `SILKBAR_ABI_VERSION` (line 97):
```rust
/// Quil proof ping — shell→Quil route verification (QUIL_PROTOCOL_ASSIGN_V1C).
/// No display authority. Quil receives and logs, does not draw or create surfaces.
pub const OP_QUIL_PING: u64 = 0xD0;
```

### 2. `kernel/src/init.rs` — Cap Grant

Added after the Linen grant block:
```rust
// Quil route: grant silk-shell capability to ping Quil (no display caps).
if quil_id != 0 && silkshell_id != 0 {
    use crate::ipc::DOMAIN_REGISTRY;
    use crate::capability::CapabilityData;
    if let Some(pd) = DOMAIN_REGISTRY.get(silkshell_id) {
        pd.grant_capability(sex_pdx::SLOT_QUIL, CapabilityData::Domain(quil_id));
        serial_println!("[kernel.cap.quil.route] shell->quil slot={}", sex_pdx::SLOT_QUIL);
    }
}
```

Grant is **one-way** (silk-shell → Quil). Quil receives no additional caps.

### 3. `servers/silk-shell/src/main.rs` — Ping Call

Added to import block (line 7-12):
```rust
SLOT_QUIL, OP_QUIL_PING,
```

Added in `open_quil_in_active_scene()` after the fill-rect `pdx_call` to sexdisplay:
```rust
// V1D: Route proof — ping Quil PD to confirm shell→Quil PDX path.
pdx_call(SLOT_QUIL, OP_QUIL_PING, 0, 0, 0);
static mut QUIL_ROUTE_BUDGET: u32 = 8;
let b = &mut QUIL_ROUTE_BUDGET;
if *b > 0 { *b -= 1; serial_println!("[shell.quil.route.ping] fid={}", fid); }
```

### 4. `servers/quil/src/main.rs` — Receive Handler

Added to dispatch match:
```rust
OP_QUIL_PING => {
    unsafe {
        static mut QUIL_ROUTE_BUDGET: u32 = 8;
        let b = &mut QUIL_ROUTE_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[quil.route.recv]");
        }
    }
}
```

---

## Proof Markers

| Marker | Budget | Location | Condition |
|--------|--------|----------|-----------|
| `[shell.quil.route.ping]` | 8 | `open_quil_in_active_scene()` | After fill rect, before `snap_capture_layout()` |
| `[quil.route.recv]` | 8 | Quil `OP_QUIL_PING` arm | On successful route receive |

---

## Capability Topology (Post-V1D)

```
silk-shell ──SLOT_QUIL──→ Quil (domain 9)
    │                         │
    │ SLOT_DISPLAY             │ (no display caps)
    ↓                         │
 sexdisplay                   │
    │                         │
    │ SLOT_SHELL              │
    └─────────────────────────┘
```

- Shell owns Quil surface lifecycle and placeholder rendering
- Quil receives pings, logs them, draws nothing
- Sexdisplay unchanged

---

## Build Verification

```sh
./scripts/entrypoint_build.sh
# Result: ISO produced, no errors
# Warnings: only pre-existing (unused import in sexstore, etc.)
```

---

## References

- `QUIL_SURFACE_STUB_V1A.md` — shell-side lifecycle proof
- `QUIL_PD_SPAWN_V1B.md` — Quil PD boot (domain 9, no caps)
- `QUIL_PROTOCOL_ASSIGN_V1C.md` — slot/opcode audit and assignment
- `crates/sex-pdx/src/lib.rs` — slot constants (lines 342-351), opcode constants (lines 85-99)
- `kernel/src/init.rs` — cap grants (lines 86-168)
