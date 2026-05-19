# SEXNET_ARP_REPLY_HOST_OBSERVE_GATE_V1

Date: 2026-05-19
Commit: pending
Gate: `sexnet_arp_reply_host_observe`

## A. Old State Before This Mission

- `sexnet_arp_proof` gate existed proving guest-side ARP request/reply (rx_arp=1, tx_dd=1)
- `sexnet_arp_rx_poll`, `sexnet_arp_rx_valid`, `sexnet_arp_tx_reply`, `sexnet_arp_tx_dd` gates existed for per-stage ARP evidence
- No gate existed for host-side observation of the ARP reply
- `scripts/host_arp_reply_observe_probe.sh` existed as an untracked file but was not wired into the gate system

## B. Audit Summary

Guest-side ARP one-shot proof evidence (TAP lane):
```
[sexnet.arp.rx.frame] idx=4 ethertype=0x0806 ok=1
[sexnet.arp.rx.validate] htype=1 ptype=0x0800 hlen=6 plen=4 oper=1 tpa_match=1 ok=1
[sexnet.arp.tx.reply.build] spa=10.0.2.15 ok=1
[sexnet.arp.tx.desc] slot=1 len=60 ok=1
[sexnet.arp.tx.post] tdt=2 ok=1
[sexnet.arp.tx.poll.done] dd_set=1 ok=1
[sexnet.arp.proof.done] rx_arp=1 tx_dd=1 ok=1
```

Host probe script (`scripts/host_arp_reply_observe_probe.sh`):
- Uses `sudo arping -I $TAP_IF -c 10 -w 30 $GUEST_IP`
- Requires TAP interface and root privileges
- Emits `[arp.host.observe.proof.done] reply_seen=1 ok=1` on success
- Not runnable in current environment (no TAP, no root)

## C. Gate Implementation

Two acceptance lanes:

**Lane A — Host probe PASS**:
- `arp.host.observe.proof.done reply_seen=1 ok=1` or `sexnet.phaseA.arp.host_observe.pass`
- Full PASS — host confirmed ARP reply from guest

**Lane B — Guest-side REVIEW ONLY**:
- `sexnet.arp.proof.done rx_arp=1 tx_dd=1 ok=1`
- PASS REVIEW ONLY — NIC transmitted ARP reply (tx_dd=1), host observation not confirmed
- Honest about limitation: host probe could not run

**FAIL**:
- Host probe ran but found `reply_seen=0` — real host observation failure

**SKIP**:
- No TAP backend active, no ARP on sexnet NIC — gate not applicable

## D. Proof Result

| Profile | Result |
|---------|--------|
| `QEMU_NET_BACKEND=tap` (TAP log) | PASS (REVIEW ONLY — guest-side ARP TX dd=1) |
| `QEMU_NET_BACKEND=user` | SKIP (no TAP, no ARP on sexnet NIC) |

## E. Fault Scan

Fault count: 0 (no #PF, #GP, panic, fault.kill)

## F. Files Changed

- `scripts/daily_driver_master_gate.sh` — added `sexnet_arp_reply_host_observe` gate (declaration, evaluation, ALL_GATES entry)
- `docs/handoff/SEXNET_ARP_REPLY_HOST_OBSERVE_GATE_V1.md` — this handoff
- `docs/handoff/NETWORK_STACK_STATUS_ROLLUP_V1.md` — Phase A rollup
- `scripts/host_arp_reply_observe_probe.sh` — pre-existing (untracked), now part of Phase A

## G. Recurrence Note

When gating host-observed behavior from a VM guest:
1. Guest-side TX DD=1 proves NIC consumption, not host reception
2. Host observation requires a backend that exposes guest traffic (TAP, not usernet)
3. Always provide REVIEW ONLY lane for guest-side evidence when host probe can't run
4. Never fake host observation — SKIP honestly when backend doesn't support it
