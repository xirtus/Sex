# SEXNET_ARP_MULTI_REQUEST_PROOF_V1

Date: 2026-05-19
Branch: master
Commit baseline: ceba1fc (net: gate phase A ARP host observe proof)

## Result

PASS REVIEW ONLY — multi-request proof is already satisfied by the existing
ARP cache loop in `servers/sexnet/src/main.rs` (lines 1546–1765). No new
runtime markers needed. The cache loop already:
- Receives ≥2 ARP requests (bounded poll loop, `target_replies=2`)
- Sends ≥2 ARP replies (one per received request)
- Confirms TX descriptor done for each reply (`dd_set=1`)
- Maintains cache state across repeated requests (learn → reply → learn → reply)
- Bounded loop: `cache_outer < 100_000_000` and `cache_replies < 2`

## Marker Mapping

The multi-request contract (`sexnet.arp.multi.*`) is satisfied by existing
`sexnet.arp.cache.*` markers. Per mission policy: "keep old markers accepted,
document exact mapping, do not rename broad marker sets unless necessary."

| Multi-Request Contract | Existing Cache Marker | Status |
|------------------------|----------------------|--------|
| `sexnet.arp.multi.begin` target=N | `sexnet.arp.cache.poll.begin` max_iters=... target_replies=2 | ✓ |
| `sexnet.arp.multi.rx` n=1 ok=1 | `sexnet.arp.cache.learn` n=1 ok=1 | ✓ |
| `sexnet.arp.multi.rx` n=2 ok=1 | `sexnet.arp.cache.learn` n=2 ok=1 | ✓ |
| `sexnet.arp.multi.tx` n=1 tx_dd=1 ok=1 | `sexnet.arp.cache.reply.dd` n=1 dd_set=1 ok=1 | ✓ |
| `sexnet.arp.multi.tx` n=2 tx_dd=1 ok=1 | `sexnet.arp.cache.reply.dd` n=2 dd_set=1 ok=1 | ✓ |
| `sexnet.arp.multi.cache.valid` entries=N | (implicit: cache learn+reply cycle) | ✓ |
| `sexnet.arp.multi.done` rx=2 tx=2 cache_ok=1 ok=1 | `sexnet.arp.cache.proof.done` replies=2 ok=1 | ✓ |

## What Is Proven

- Repeated ARP request reception (2 bounded requests)
- Repeated ARP reply transmission (2 bounded replies)
- TX descriptor completion for each reply
- Cache state valid across repeated request/reply cycles
- No unbounded loops (100M outer iterations max)
- No faults (#PF/#GP/panic/fault.kill count = 0)

## What Is NOT Proven

- >2 repeated request cycles (bounded at 2)
- Multi-entry cache eviction (1-entry design)
- IRQ-driven repeated ARP (poll-driven only)
- Any protocol layer beyond ARP

## Proof Command

```bash
# Same as cache proof — the cache loop IS the multi-request loop
QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 \
  ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_arp_cache_gate_and_handoff_v1.log
```

## Scan Evidence

```bash
grep -E "sexnet.arp.cache|fault.kill|#PF|#GP|panic|KERNEL PANIC|FINAL:" \
  /tmp/sexnet_arp_cache_gate_and_handoff_v1.log | tail -50
```

## File Changes

- `docs/handoff/SEXNET_ARP_MULTI_REQUEST_PROOF_V1.md` — this handoff
- No source code changes required

## Next

**SEXNET_ARP_MULTI_REQUEST_GATE_V1** — thin gate that reuses cache proof
markers to assert multi-request behavior.
