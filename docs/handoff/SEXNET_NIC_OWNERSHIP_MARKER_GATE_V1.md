# SEXNET_NIC_OWNERSHIP_MARKER_GATE_V1

## A. Result
Added a narrow daily-driver gate `sexnet_nic_ownership_init` in `scripts/daily_driver_master_gate.sh` for the non-behavioral sexnet NIC ownership init marker.

The gate is marker-only and remains `SKIP` when the marker is absent on unrelated/non-enabled boots.

## B. Proof command
```bash
QEMU_NET_BACKEND=tap \
QEMU_NET_MODEL=e1000e \
QEMU_TAP_IFNAME=tap0 \
ENABLE_QEMU_USERNET_E1000=1 \
./scripts/run_daily_driver_proof.sh /tmp/sexnet_nic_ownership_marker_gate_v1.log
```

Scan:

```bash
grep -E "sexnet_nic_ownership_init|sexnet.nic.ownership.init|fault.kill|#PF|#GP|panic|KERNEL PANIC|FINAL:" \
/tmp/sexnet_nic_ownership_marker_gate_v1.log | tail -240
```

## C. Marker evidence
Gate PASS requires exactly:

- `[sexnet.nic.ownership.init] rx_owner=0 tx_owner=0 ok=1`

Gate FAIL only if marker exists but violates init contract (nonzero owner and/or `ok!=1`).

Gate SKIP if marker is absent.

## D. What was proven
- Ownership init marker/state-contract is present and parseable.
- Boot-time owner state is HAL diagnostic (`rx_owner=0`, `tx_owner=0`) when marker passes.

## E. What was not proven
- No RX/TX permanent ownership transfer.
- No RX/TX observe success requirement.
- No register-level behavior transition.
- No protocol behavior.

## F. Architecture boundary
This mission is marker/state-contract only.

No ownership transfer happened, no register behavior changed, and HAL diagnostic bridge remains preserved. Permanent RX/TX ownership still requires later STOP FIRST review.

## G. STOP FIRST rules
Stop first if any of these occur:

- Gate starts requiring RX/TX observe markers.
- Gate hard-fails absent marker on unrelated boots.
- Marker name drift requires source-code changes.
- Any non-allowed code file must be touched.

## H. Next missions
1. `SEXNET_RX_PERMANENT_RING_STOP_REVIEW_V1`
