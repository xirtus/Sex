# LINEN_ZERO_NAME_CREATE_STORM_V1

Date: 2026-07-02
Status: patched; Linen storm fixed; broader input/perf proof still blocked by stimulus/runtime lane

## Root Cause

`caller=12` maps to the deterministic `spindle` PD:

- `kernel/src/init.rs` spawn order assigns `spindle` domain id 12.
- The kernel grants Spindle `SLOT_LINEN`.

Spindle had a stale boot-time `.spn` session create:

```rust
pdx_call(SLOT_LINEN, OP_LINEN_CREATE_OBJECT, 0, 0, 0)
```

For Linen create, `arg0` packs `kind` in bits 0-7 and `name_len` in bits 8-15.
All-zero args therefore mean `kind=0` and `name_len=0`. Linen correctly rejects
that as `bad_name_len`; the async PDX edge then replayed the invalid request
into a large reject storm.

## Fix

Removed the stale Spindle boot create instead of weakening Linen validation.
Spindle already documents interactive `object-new` as cross-PD create blocked,
so the startup create was inconsistent with the active command model.

Runtime markers added in Spindle:

```text
[linen.zero_name_storm.begin]
[linen.zero_name_storm.source] caller=12 server=spindle reason=stale_spn_boot_create_zero_args
[linen.zero_name_storm.fixed] invalid_sends_before=1 invalid_sends_after=0
[linen.zero_name_storm.ok]
```

## Proof Notes

Before fix:

- `logs/qemu-latest.log` showed `[spindle.linen.spn.create] status=0`.
- Linen emitted first 4 `bad_name_len len=0 caller=12` lines.
- `[perf.noise.summary] name=linen.session.reject count=65536 suppressed=65532`
  confirmed the real storm despite bounded logging.

Expected after fix:

- `bad_name_len len=0 caller=12` count is 0.
- no `perf.noise.summary name=linen.session.reject` appears.
- required zero-name storm markers appear once.

Observed after fix:

- Build: `./scripts/entrypoint_build.sh` PASS.
- QEMU focused runtime proof required escalation for AF_UNIX QMP socket bind
  (`Operation not permitted` inside sandbox).
- `logs/qemu-latest.log` contains all four required zero-name markers.
- `bad_name_len len=0 caller=12`: 0.
- `perf.noise.summary name=linen.session.reject`: 0.
- `spindle.linen.spn.create`: 0.
- Strict raw fault scan on the final short Linen proof log:
  `#PF=0`, `PAGE FAULT=0`, `#GP=0`, `panic=0`, `fault.kill=0`,
  `reboot_loop=0`, `freeze=0`, `storm=0`.

Broader gates were not all green in one final clean log:

- `scripts/input_current_tier_gate.sh logs/qemu-latest.log`: FAIL in the final
  short clean log because drag/click markers were not reached before cutoff.
- A longer adjusted stimulus reached current-tier markers in one run, but later
  hit an unrelated late `KERNEL PAGE FAULT HALT` after the needed markers.
- `scripts/input_control_quality_gate.sh logs/qemu-latest.log`: FAIL/partial
  on the clean short log because drag markers were absent.
- `scripts/perf_bisection_gate.sh logs/qemu-latest.log`: FAIL for Chapter 1
  regression on the clean short log; Linen logvolume stayed fixed at
  `linen_session_reject=0`.

## Why Earlier Gates Missed It

`PERF_LOG_NOISE_ABLATION_V1` budgeted the repeated Linen reject lines down to
the first 4 plus power-of-two summaries. That protected serial throughput and
made input/display gates healthier, but it did not remove the underlying PDX
work until this producer-side fix.

## Remaining Bottleneck

After the Linen storm fix, the remaining blocker is not Linen reject volume.
The current friction is the QMP input proof lane: getting drag move/end,
display present tick chains, and a no-fault cutoff in the same short runtime
window needs a dedicated input-stimulus pass.
