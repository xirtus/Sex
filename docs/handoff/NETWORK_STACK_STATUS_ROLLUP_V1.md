# NETWORK_STACK_STATUS_ROLLUP_V1

Date: 2026-05-19
Branch: master
Commit: pending (Phase A gate + docs)

## Phase A Status: DONE

Phase A contains:
- `SEXNET_ARP_REPLY_HOST_OBSERVE_GATE_V1` — gate for host-observed ARP reply
- `NETWORK_STACK_STATUS_ROLLUP_V1` — this rollup

## What Is Proven (Phase A)

| Item | Evidence | Confidence |
|------|----------|------------|
| NIC full ownership | `sexnet.nic.full.ownership` rx_owner=3 tx_owner=3 | PROVEN |
| L2 loop proof | `sexnet.l2.proof.done` rx_frames=1 tx_dd=1 | PROVEN |
| ARP one-shot request/reply | `sexnet.arp.proof.done` rx_arp=1 tx_dd=1 ok=1 | PROVEN |
| ARP TX DD consumed | `sexnet.arp.tx.poll.done` dd_set=1 | PROVEN |
| ARP gateway resolved | `arp.gateway.resolved` gateway_known=1 | PROVEN |
| Host ARP reply observe (guest-side) | `sexnet_arp_reply_host_observe` REVIEW ONLY | NIC TX dd=1 |

## What Is NOT Proven (Phase A)

- **ARP cache** — bounded cache proof (replies=2) is Phase B, not Phase A
- **IPv4 header validation** — proven but belongs to Phase B scope
- **ICMP echo reply** — not in Phase A
- **UDP** — not in Phase A
- **DNS** — not in Phase A
- **TCP SYN/SYN-ACK/handshake** — not in Phase A
- **HTTP GET/response** — not in Phase A
- **Browser networking** — not in Phase A
- **HAL NET_DIAG replacement** — not in Phase A
- **Host-side ARP observation (full)** — requires TAP + root; REVIEW ONLY in Phase A

## Proof Command

```bash
./scripts/entrypoint_build.sh

# User backend (default — ARP on SLiRP path, not sexnet NIC)
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_a_autopilot.log

# TAP backend (full ARP one-shot on sexnet NIC)
QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 \
  ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_arp_cache_gate_and_handoff_v1.log

# Host ARP observe probe (requires TAP + root)
./scripts/host_arp_reply_observe_probe.sh /tmp/sexnet_phase_a_host_arp_observe.log
```

## Log Paths

- `/tmp/sexnet_phase_a_autopilot.log` — user backend proof
- `/tmp/sexnet_arp_cache_gate_and_handoff_v1.log` — TAP backend proof (prior session)
- `/tmp/sexnet_phase_a_host_arp_observe.log` — host probe output (when TAP available)

## Markers Found (TAP lane)

```
[sexnet.nic.full.ownership] rx_owner=3 tx_owner=3 full_ok=1
[sexnet.l2.proof.done] rx_frames=1 tx_dd=1 ok=1
[sexnet.arp.rx.frame] idx=4 ethertype=0x0806 ok=1
[sexnet.arp.rx.validate] htype=1 ptype=0x0800 hlen=6 plen=4 oper=1 tpa_match=1 ok=1
[sexnet.arp.tx.reply.build] spa=10.0.2.15 ok=1
[sexnet.arp.tx.desc] slot=1 len=60 ok=1
[sexnet.arp.tx.post] tdt=2 ok=1
[sexnet.arp.tx.poll.done] dd_set=1 ok=1
[sexnet.arp.proof.done] rx_arp=1 tx_dd=1 ok=1
```

## STOP FIRST Notes

- Do not proceed to Phase B without Phase A gate in place
- Do not claim host ARP observation without host probe or documented REVIEW ONLY limitation
- Do not retire sexnet NIC ownership or L2 proof gates
- Network SPRINT_EXECUTION_V1 remains the authoritative sprint tracker

## Next Phase

**Phase B: SEXNET_ARP_CACHE_STOP_REVIEW_V1**
- Bounded ARP cache (2 replies with DD checks)
- Gate already exists: `sexnet_arp_cache_proof`
- Requires TAP backend for full proof

## Gate Status

| Gate | Phase A Profile | TAP Profile |
|------|----------------|-------------|
| `sexnet_arp_proof` | SKIP | PASS |
| `sexnet_arp_reply_host_observe` | SKIP | PASS (REVIEW ONLY) |
| `sexnet_arp_cache_proof` | SKIP | PASS |
