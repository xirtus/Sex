# SEXNET_IPV4_CHECKSUM_GATE_V1

Date: 2026-05-19
Commit: pending (Phase C checksum gate)
Gate: `sexnet_ipv4_checksum`

## Old State

Before this handoff, IPv4 checksum validation was proven as part of the
`sexnet_ipv4_header_validate` gate (which checks for `checksum=ok` in the rx.validate
marker). There was no standalone checksum gate. This doc adds `sexnet_ipv4_checksum`
as a separate gate entry, providing finer-grained checksum-specific PASS/FAIL/SKIP
reporting while reusing the same runtime markers.

## Gate Name

`sexnet_ipv4_checksum`

## Exact Markers Accepted

The gate evaluates the same markers as `sexnet_ipv4_header_validate`, with additional
focus on the checksum-specific fields:

| Marker | Checksum-Specific Field | Meaning |
|--------|------------------------|---------|
| `[sexnet.ipv4.rx.validate.detail]` | `checksum_ok=1` | One's-complement sum folded to 0xFFFF |
| `[sexnet.ipv4.rx.validate]` | `checksum=ok` | Checksum validation passed (in positive marker) |
| `[sexnet.ipv4.rx.reject.detail]` | `reason=checksum` | Bad checksum rejected |
| `[sexnet.ipv4.proof.done]` | `ok=1` | Proof summary (includes checksum) |

## PASS Behavior

Gate PASSES when all of:
1. `[sexnet.ipv4.rx.validate.detail]` contains `checksum_ok=1` — checksum computed and valid
2. `[sexnet.ipv4.rx.validate]` contains `checksum=ok` and `ok=1` — validate pass includes checksum
3. `[sexnet.ipv4.proof.done]` contains `ok=1` — full proof passed
4. No `[sexnet.ipv4.proof.done]` with `ok=0`
5. No fault/panic markers

Additionally, the gate recognizes that a `reason=checksum` rejection marker proves
the negative checksum path works (bad checksum correctly rejected). However if only
a rejection marker with `reason=checksum` appears without a later positive validate,
the gate gives a qualified PASS with note "negative checksum rejection proven; no
positive frame received."

## FAIL Behavior

Gate FAILS when:
1. `[sexnet.ipv4.rx.validate.detail]` exists with `checksum_ok=0` for a frame that
   should have a valid checksum (header validate passed but checksum compute failed)
2. `[sexnet.ipv4.rx.validate]` exists with `ok=1` but without `checksum=ok`
   (header validate passed without checksum — contract violation)
3. `[sexnet.ipv4.proof.done]` exists with `ok=0` — proof failed
4. Fault/panic markers appear

## SKIP Behavior

Gate SKIPS when no IPv4 markers are present in the log. This is expected on:
- usernet backend (no TAP host stimulus reaches guest NIC)
- TAP backend without host stimulus
- Profile that intentionally disables IPv4 proof
- Build-only runs that don't boot a VM

## Proof Command

Same as header validate proof:

```bash
./scripts/entrypoint_build.sh

QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 \
  ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_c_tap.log
```

## Log Path

`/tmp/sexnet_phase_c_tap.log` (shared with header validate gate)

## Fault Count

Expected: 0. Any fault → FAIL.

## Gate Script Entry (to be added)

The gate evaluation logic will be added to `scripts/daily_driver_master_gate.sh`
as a new section. Suggested implementation:

```bash
# ---- SEXNET_IPV4_CHECKSUM_GATE_V1 ----
# Reuses same markers as sexnet_ipv4_header_validate, focused on checksum fields.
if [ "$(has 'sexnet\.ipv4\.proof\.done.*ok=0')" -eq 1 ]; then
    gate_sexnet_ipv4_checksum="FAIL"
    print_row "sexnet_ipv4_checksum" "FAIL" "proof.done ok=0 — checksum validation failed"
elif [ "$(has 'sexnet\.ipv4\.rx\.validate\.detail.*checksum_ok=0')" -eq 1 ] \
     && [ "$(has 'sexnet\.ipv4\.rx\.validate\.detail.*checksum_ok=1')" -eq 0 ]; then
    gate_sexnet_ipv4_checksum="FAIL"
    print_row "sexnet_ipv4_checksum" "FAIL" "checksum_ok=0 with no later checksum_ok=1"
elif [ "$(has 'sexnet\.ipv4\.rx\.validate\.detail.*checksum_ok=1')" -eq 1 ] \
     && [ "$(has 'sexnet\.ipv4\.rx\.validate.*checksum=ok.*ok=1')" -eq 1 ] \
     && [ "$(has 'sexnet\.ipv4\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_sexnet_ipv4_checksum="PASS"
    print_row "sexnet_ipv4_checksum" "PASS" "IPv4 checksum compute+validate proven"
elif [ "$(has 'sexnet\.ipv4\.rx\.reject\.detail.*reason=checksum')" -eq 1 ]; then
    gate_sexnet_ipv4_checksum="PASS"
    print_row "sexnet_ipv4_checksum" "PASS" "negative checksum rejection proven (no positive frame)"
else
    gate_sexnet_ipv4_checksum="SKIP"
    print_row "sexnet_ipv4_checksum" "SKIP" "no TAP/no ping stimulus — IPv4 checksum markers absent"
fi
```

## ALL_GATES Entry (to be added)

```
"sexnet_ipv4_checksum:$gate_sexnet_ipv4_checksum"
```

Inserted after line 3134 (`sexnet_ipv4_header_validate` entry).

## Declaration (to be added)

```
gate_sexnet_ipv4_checksum="SKIP"
```

Inserted after line 226 (after `gate_sexnet_ipv4_header_validate="SKIP"`).

## Next

NETWORK_STACK_STATUS_ROLLUP_V1 update (this session)
Phase D: SEXNET_ICMP_ECHO_STOP_REVIEW_V1
