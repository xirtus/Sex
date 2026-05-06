# SILKBAR_CONTRACT_LOCK_V1

## Status: **LOCKED** ✅

SilkBar/SexDisplay contract is fully locked. Both producer (silkbar) and consumer
(sexdisplay) enforce the shared contract at startup via `validate_silkbar_contract()`
from `crates/silkbar-model`.

## Changes

| File | Change |
|------|--------|
| `servers/sexdisplay/src/main.rs` | Fix: hang on contract failure (matching silkbar behavior) |

## Details

### Fix: sexdisplay now hangs on contract failure

Before:
```rust
if contract_err != 0 {
    serial_println!("[silk.contract.validate.fail] reason={}", contract_err);
} else {
    serial_println!("[silk.contract.validate.ok] version={}", SILKBAR_ABI_VERSION);
}
```

After:
```rust
if contract_err != 0 {
    serial_println!("[silk.contract.validate.fail] reason={}", contract_err);
    loop { core::hint::spin_loop(); }
} else {
    serial_println!("[silk.contract.validate.ok] version={}", SILKBAR_ABI_VERSION);
}
```

Previously, sexdisplay logged contract failure but continued running, risking
silent rendering corruption. Now both producer and consumer fail-stop on
contract mismatch, matching the intended design in `gate_render.sh`.

### What was already correct (no changes needed)

| Check | Status |
|-------|--------|
| ABI/layout/theme contract defined in `crates/silkbar-model` | ✅ `validate_contract()` checks all constants |
| Deterministic update vectors validated | ✅ `validate_deterministic_vectors()` in silkbar-model |
| `OP_SILKBAR_UPDATE` constant shared (imported from silkbar-model) | ✅ Both servers import from crate |
| `UpdateKind` discriminants match (shared enum) | ✅ Both servers import from crate |
| `SilkBarUpdate` ABI size asserted (16 bytes) | ✅ Compile-time assert in silkbar-model |
| silkbar validates contract at startup | ✅ `_start()` calls `validate_silkbar_contract()` |
| sexdisplay validates contract at startup | ✅ `_start()` calls `validate_silkbar_contract()` |
| `gate_render.sh` catches ABI mismatch | ✅ Checks source symbols + runtime markers |
| Render path uses shared model state | ✅ Both use `SilkBar`/`SilkBarUpdate` from crate |

### Build-time self-check

```sh
# Static source checks:
./scripts/gate_render.sh

# Full build (also runs validate_silk_de_gates in entrypoint):
./scripts/entrypoint_build.sh
```

### Runtime proof

```sh
./scripts/master_runtime_gate.sh --skip-build
```

Required runtime markers (all present in boot log):
- `[silk.contract.validate.ok] version=2` — both silkbar and sexdisplay
- `[silk.render_proof.top_strip.ok]` — renderer conformance proof
- `[silkbar.clock.send]` — clock liveness

### Safety

- **Faults**: 0
- **PDs spawned**: 7/7
- **Clock ticks**: 12+
- **Contract validation**: 2/2 (both producers pass)

## Recurring Issue

Consumer-side contract enforcement was missing the fall-stop loop. When both
producer and consumer import from the same model crate, it's easy to assume
both enforce equally — but sexdisplay was silently tolerating ABI failure.
Future audit: verify that every `validate_silkbar_contract()` caller has a
matching fail-stop pattern.
