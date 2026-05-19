# SEXNET_IPV4_HEADER_VALIDATE_GATE_V1

Date: 2026-05-19
Commit: pending (Phase C gate handoffs)
Gate: `sexnet_ipv4_header_validate`

## Old State

Gate `sexnet_ipv4_header_validate` was committed in `b04fc89 net: gate sexnet IPv4 header validation`.
The runtime IPv4 parse/validate/checksum code was committed in `c432689 sexnet: prove IPv4 header validation`.
Proof doc `SEXNET_IPV4_HEADER_VALIDATE_PROOF_V1.md` was committed alongside.

Before this handoff, the gate existed in `scripts/daily_driver_master_gate.sh` but lacked a
standalone handoff document. This doc provides the formal gate contract.

## Gate Name

`sexnet_ipv4_header_validate`

## Proof Command

```bash
# Host stimulus (separate terminals):
#   while true; do sudo arping -I tap0 -c 1 -w 1 10.0.2.15 2>/dev/null || true; sleep 0.05; done
#   while true; do ping -I tap0 -c 1 -W 1 10.0.2.15 2>/dev/null || true; sleep 0.2; done

QEMU_NET_BACKEND=tap \
QEMU_NET_MODEL=e1000e \
QEMU_TAP_IFNAME=tap0 \
ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_ipv4_header_validate_gate_v1.log
```

## Log Path

`/tmp/sexnet_ipv4_header_validate_gate_v1.log`

## Exact Markers Required for PASS

| # | Marker Pattern | Meaning |
|---|---------------|---------|
| 1 | `[sexnet.ipv4.entry] rx_owner=3 ok=1` | sexnet owns RX ring (NIC_OWNER_SEXNET_FULL) |
| 2 | `[sexnet.ipv4.rx.frame] ... ethertype=0x0800 ok=1` | Frame with IPv4 ethertype received |
| 3 | `[sexnet.ipv4.rx.validate] version=4 ihl=5 ... dst=10.0.2.15 ... checksum=ok ... ok=1` | Header validated: ver/ihl/len/dst/checksum all pass |
| 4 | `[sexnet.ipv4.rx.recycle] ... ok=1` | RX descriptor recycled after processing |
| 5 | `[sexnet.ipv4.proof.done] frames=1 ok=1` | Proof summary: 1 valid IPv4 frame processed |

## PASS Behavior

Gate PASSES when all 5 markers are present with correct field values AND no fault markers
(`fault.kill`, `#PF`, `#GP`, `panic`, `KERNEL PANIC`) appear in the log.

```bash
# Script logic (daily_driver_master_gate.sh lines 2268–2274):
if proof.done frames=1 ok=1
   && entry rx_owner=3 ok=1
   && rx.frame ethertype=0x0800 ok=1
   && rx.validate version=4 ihl=5 dst=10.0.2.15 checksum=ok ok=1
   && rx.recycle ok=1
then PASS
```

## FAIL Behavior

Gate FAILS when:
1. `[sexnet.ipv4.proof.done]` exists with `ok=0` — IPv4 validation failed
2. `[sexnet.ipv4.entry]` exists with `ok=0` — RX owner not acquired
3. `[sexnet.ipv4.rx.validate]` exists with `ok=0` AND no later `ok=1` — header invalid
4. Fault/panic markers appear (`fault.kill`, `#PF`, `#GP`, `panic`, `KERNEL PANIC`)

## SKIP Behavior

Gate SKIPS when none of the IPv4 markers are present in the log. This is expected on:
- usernet backend (no TAP host stimulus, no ping reaches guest NIC)
- TAP backend without host stimulus (no arping/ping flood running)
- Profile that intentionally disables IPv4 proof

## Fault Count

Expected: 0. Any fault marker → FAIL (handled by global fault scan, not gate-specific count).

## Negative Validation

The IPv4 code rejects malformed headers. The rejection marker is:
```
[sexnet.ipv4.rx.reject.detail] idx=N etype=0x0800 reason=<reason> ok=0
```

Where `<reason>` is one of: `short`, `version`, `ihl`, `total_len_min`, `total_len_max`,
`fragmented`, `dst`, `checksum`, `non_ipv4`.

Only the first rejection is logged (`reject_logged == 0` guard) to limit log noise in the
bounded proof loop (max 1 positive frame, 1 rejection logged).

Negative validation is proven by source audit: the code has exhaustive rejection branches
(lines 2014–2048) covering all malformed cases. A separate negative-proof gate may be added
in a future phase if needed; for Phase C the source audit suffices.

## Gate Location in Script

`scripts/daily_driver_master_gate.sh` lines 2245–2278
Declaration: line 226 (`gate_sexnet_ipv4_header_validate="SKIP"`)
ALL_GATES entry: line 3134

## Next

SEXNET_IPV4_CHECKSUM_PROOF_V1 (Task 11)
SEXNET_IPV4_CHECKSUM_GATE_V1 (Task 12)
