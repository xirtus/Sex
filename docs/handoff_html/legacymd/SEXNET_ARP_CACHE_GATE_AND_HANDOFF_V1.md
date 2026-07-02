# SEXNET_ARP_CACHE_GATE_AND_HANDOFF_V1

Date: 2026-05-19
Branch: master
Commit baseline: ceba1fc (net: gate phase A ARP host observe proof)

## Gate: `sexnet_arp_cache_proof`

Already implemented in `scripts/daily_driver_master_gate.sh` (lines 2194–2215).

### Old State

Gate already existed as PASS/SKIP before this handoff was written.
Runtime markers emitted by `servers/sexnet/src/main.rs` lines 1558–1765.

### Evaluation Logic

```
PASS: sexnet.arp.cache.proof.done replies=2 ok=1
      AND sexnet.arp.cache.reply.dd n=1 dd_set=1 ok=1
      AND sexnet.arp.cache.reply.dd n=2 dd_set=1 ok=1

FAIL: proof.done ok=0
   OR reply.dd dd_set=0
   OR reply present with wrong slot/tdt pair

SKIP: no TAP / no ARP cache markers in this boot
```

### Exact Markers Accepted

| Marker | Required For |
|--------|-------------|
| `[sexnet.arp.cache.poll.begin] max_iters=100000000 target_replies=2` | informational |
| `[sexnet.arp.cache.learn] n=1 sha=... spa=... ok=1` | insert proof |
| `[sexnet.arp.cache.learn] n=2 sha=... spa=... ok=1` | repeated insert |
| `[sexnet.arp.cache.reply] n=1 slot=3 tdt=4 ok=1` | reply tx |
| `[sexnet.arp.cache.reply] n=2 slot=4 tdt=5 ok=1` | repeated reply tx |
| `[sexnet.arp.cache.reply.dd] n=1 dd_set=1 ok=1` | TX DD consumed |
| `[sexnet.arp.cache.reply.dd] n=2 dd_set=1 ok=1` | repeated TX DD |
| `[sexnet.arp.cache.poll.done] outer=... replies=2 ok=1` | poll loop done |
| `[sexnet.arp.cache.proof.done] replies=2 ok=1` | **gate entry point** |

### PASS/FAIL/SKIP Behavior

| Case | Verdict | Reason |
|------|---------|--------|
| TAP+e1000e, 2 replies with DD | PASS | bounded ARP cache proof |
| TAP+e1000e, proof.done ok=0 | FAIL | cache contract failed |
| usernet+e1000e, no ARP on NIC | SKIP | honest: no TAP/ARP stimulus |
| Default e1000, no RX | SKIP | no ARP cache markers |

### Proof Command

```bash
# TAP lane (full ARP cache proof on sexnet NIC)
QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 \
  ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_arp_cache_gate_and_handoff_v1.log

# Usernet lane (may SKIP if ARP not on sexnet NIC)
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_b_user.log
```

### Log Paths

- `/tmp/sexnet_arp_cache_gate_and_handoff_v1.log` — TAP backend proof
- `/tmp/sexnet_phase_b_user.log` — user backend proof

### Fault Count

Expected: 0. Fault scan checks for `#PF`, `#GP`, `panic`, `KERNEL PANIC`, `fault.kill`.

### What Is Proven

- Bounded 1-entry ARP cache learn from valid ARP request sender fields
- Cache entry used to build ARP reply (hit behavior implicit)
- TX descriptor completion (DD) for both bounded reply events (n=1, n=2)
- Repeated reply behavior: 2 ARP requests → 2 replies
- Slot/TDT progression: n=1 → slot=3/tdt=4, n=2 → slot=4/tdt=5

### What Is NOT Proven

- Multi-entry cache eviction (1-entry design, no eviction needed)
- IRQ-driven ARP flow (poll-driven only)
- IPv4, ICMP, UDP, DNS, TCP, HTTP
- Browser networking
- HAL NET_DIAG retirement

### File Changes

- `scripts/daily_driver_master_gate.sh` — gate already present (prior commit)
- `docs/handoff/SEXNET_ARP_CACHE_PROOF_V1.md` — proof doc (prior commit)
- `docs/handoff/SEXNET_ARP_CACHE_STOP_REVIEW_V1.md` — STOP review (this session)
- `docs/handoff/SEXNET_ARP_CACHE_GATE_AND_HANDOFF_V1.md` — this handoff

### Next

**SEXNET_ARP_MULTI_REQUEST_PROOF_V1** — documented proof that cache loop
already satisfies multi-request contract (2 requests, 2 replies, bounded).
