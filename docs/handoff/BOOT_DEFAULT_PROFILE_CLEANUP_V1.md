# BOOT_DEFAULT_PROFILE_CLEANUP_V1

## Summary

Made heavy proof/diagnostic lanes opt-in behind compile-time environment flags.
Default boot now skips:
- e1000 heavy proof sweeps (ring observe, permanent, TX, ARP cache with 50M/100M polls)
- DNS/TCP/HTTP protocol proof suite
- Silk deterministic topstrip render proof
- Live topstrip budgeted proof markers (clear, bounds, audit, tick4, v2.rows diag)
- Frame Rim visual proof markers
- Frame Lights visual proof markers

## Flags

| Flag | Crate | Effect |
|------|-------|--------|
| `SEXOS_SEXNET_PROOF_PROFILE=1` | servers/sexnet | Enables full NIC + protocol proof suite |
| `SEXOS_SILK_RENDER_PROOF_PROFILE=1` | servers/sexdisplay | Enables full visual + render proof suite |

Both flags default to OFF (unset = proofs skipped).

## Files Changed

| File | Change |
|------|--------|
| `servers/sexnet/src/main.rs` | Added `SEXNET_PROOF_PROFILE_ENABLED` gate + calm boot markers |
| `servers/sexdisplay/src/main.rs` | Added `SILK_RENDER_PROOF_PROFILE_ENABLED` gate + firstpaint markers |
| `servers/silk-shell/src/main.rs` | Added `[boot.firstpaint.shell_ready]` marker |

## Boot Calm Markers

Emitted in default boot:
- `[boot.default.profile] proofs=light ok=1` (sexnet, when proof profile OFF)
- `[boot.firstpaint.display_surface] ok=1` (sexdisplay, after first render)
- `[boot.firstpaint.shell_ready] ok=1` (silk-shell, after UI ready)
- `[boot.firstpaint.silkbar_clock_send] ok=1` (sexdisplay, on first silkbar clock)

## Gate Script Impact

The `daily_driver_master_gate.sh` requires NO changes. All heavy proof gates
naturally SKIP when their markers are absent from the log. No false FAILs.

## Proof

- Compilation: sexnet, sexdisplay, silk-shell all compile cleanly (zero new errors)
- Entry: `./scripts/entrypoint_build.sh` already unsets `SEXOS_SEXNET_PROOF_PROFILE`
- Daily gate: naturally SKIPs heavy proof categories on default boot

## To Enable Proofs

```sh
SEXOS_SEXNET_PROOF_PROFILE=1 SEXOS_SILK_RENDER_PROOF_PROFILE=1 ./scripts/entrypoint_build.sh
```
