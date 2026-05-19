# SEXNET_ARP_MULTI_REQUEST_GATE_V1

Date: 2026-05-19
Branch: master
Commit baseline: ceba1fc (net: gate phase A ARP host observe proof)

## Gate: `sexnet_arp_multi_request`

Added to `scripts/daily_driver_master_gate.sh` in this session.

### Evaluation Logic

```
PASS: sexnet.arp.cache.proof.done replies=2 ok=1
      AND sexnet.arp.cache.reply.dd n=1 dd_set=1 ok=1
      AND sexnet.arp.cache.reply.dd n=2 dd_set=1 ok=1
      AND sexnet.arp.cache.learn n=1 ok=1
      AND sexnet.arp.cache.learn n=2 ok=1

SKIP: no TAP / no ARP cache markers in this boot
      (honest: usernet/SLiRP hides ARP on sexnet NIC, or TAP unavailable)

FAIL: cache proof present with ok=0
   OR reply.dd dd_set=0
   OR learn markers absent despite proof.done ok=1
```

### Marker Contract

| Marker | Required | Notes |
|--------|----------|-------|
| `sexnet.arp.cache.proof.done` replies=2 ok=1 | yes | maps to `sexnet.arp.multi.done` |
| `sexnet.arp.cache.reply.dd` n=1 dd_set=1 ok=1 | yes | maps to `sexnet.arp.multi.tx` n=1 |
| `sexnet.arp.cache.reply.dd` n=2 dd_set=1 ok=1 | yes | maps to `sexnet.arp.multi.tx` n=2 |
| `sexnet.arp.cache.learn` n=1 ok=1 | yes | maps to `sexnet.arp.multi.rx` n=1 |
| `sexnet.arp.cache.learn` n=2 ok=1 | yes | maps to `sexnet.arp.multi.rx` n=2 |
| No `#PF` / `#GP` / `panic` / `KERNEL PANIC` | yes | fault scan |

### PASS/FAIL/SKIP Behavior

| Case | Verdict | Reason |
|------|---------|--------|
| TAP+e1000e, cache proof done, faults=0 | PASS | repeated ARP request/reply proven |
| TAP+e1000e, cache proof ok=0 | FAIL | contract failed |
| TAP+e1000e, dd_set=0 | FAIL | TX not consumed |
| usernet+e1000e, no ARP markers | SKIP | honest: no TAP/ARP stimulus |
| Default e1000, no RX | SKIP | model-limited RX path |

### Proof Command

```bash
# TAP lane (full repeated ARP proof)
QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 \
  ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_arp_cache_gate_and_handoff_v1.log

# Usernet lane (may SKIP)
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_b_user.log
```

### Gate Verification

```bash
./scripts/daily_driver_master_gate.sh /tmp/sexnet_arp_cache_gate_and_handoff_v1.log
./scripts/daily_driver_master_gate.sh /tmp/sexnet_phase_b_user.log
```

### What Is Proven

- Repeated ARP request reception (≥2) on sexnet NIC
- Repeated ARP reply transmission (≥2) via E1000 TX descriptor lane
- Cache state maintained across repeated request/reply cycles
- Bounded poll loop — no unbounded wait
- No faults in multi-request path

### What Is NOT Proven

- >2 request cycles (bounded at 2)
- IRQ-driven repeated ARP (poll-driven only)
- Multi-entry eviction (1-entry cache)
- IPv4/ICMP/UDP/DNS/TCP/HTTP (Phase C+)

### File Changes

- `scripts/daily_driver_master_gate.sh` — added `sexnet_arp_multi_request` gate
- `docs/handoff/SEXNET_ARP_MULTI_REQUEST_GATE_V1.md` — this handoff

### Next

**Phase C: SEXNET_IPV4_PARSE_STOP_REVIEW_V1**
