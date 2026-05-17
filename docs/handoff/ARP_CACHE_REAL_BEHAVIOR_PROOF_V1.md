# ARP_CACHE_REAL_BEHAVIOR_PROOF_V1

Date: 2026-05-17
Model: QEMU_NET_MODEL=e1000e
Proof: FINAL PASS 229/0/0 (e1000e); 226/3skip/0 (e1000 default unchanged)
Logs:
- /tmp/sexos_arp_cache_real_behavior_proof_v1.log (e1000e — 229 pass)
- /tmp/sexos_arpcache_e1000_check.log (e1000 — 226 pass, 3 skip)

## Summary

Live ARP parse (SPA→SHA from RX buffer) stored in bounded stack-local cache.
Cache lookup verified. Gateway MAC confirmed unknown. fake=0 throughout.

---

## Cache Table

| IP         | MAC               | source       | inserted | fake |
|------------|-------------------|--------------|----------|------|
| 10.0.2.15  | 52:54:00:12:34:56 | rx_observed  | 1        | 0    |

Markers:
```
[arp.cache.update] ip=10.0.2.15 mac=52:54:00:12:34:56 source=rx_observed inserted=1 fake=0 ok=1
[arp.cache.lookup] ip=10.0.2.15 found=1 mac=52:54:00:12:34:56 ok=1
[arp.cache.real.behavior.done] ok=1 entries=1 fake=0 gateway_known=0
```

---

## Gateway Truth

```
[arp.gateway.truth] ip=10.0.2.1 mac_known=0 fake=0 ok=1 reason=gateway_mac_requires_arp_reply
```

Gateway IP 10.0.2.1 (from TPA in observed ARP request).
Gateway MAC not yet known — no oper=2 ARP reply received from that IP.

---

## Gate Results

| Gate                   | e1000e | e1000        |
|------------------------|--------|--------------|
| e1000e_rx_desc_observe | PASS   | SKIP         |
| arp_rx_observe_live    | PASS   | SKIP         |
| arp_cache_real_behavior| PASS   | SKIP         |
| Total                  | 229/0/0| 226/0/3skip  |

---

## Cumulative Network State

| Item         | Value             | Confidence |
|--------------|-------------------|------------|
| Our IP       | 10.0.2.15         | confirmed (SPA in ARP) |
| Our MAC      | 52:54:00:12:34:56 | confirmed (SHA in ARP) |
| Gateway IP   | 10.0.2.1          | confirmed (TPA in ARP) |
| Gateway MAC  | unknown           | needs ARP reply        |
| SLiRP network| 10.0.2.0/24       | inferred               |

---

## Next Recommendation

**ARP_REQUEST_SEND_PROOF_V1**

We have all the fields needed to send a proper ARP request:
- SHA: 52:54:00:12:34:56 (our MAC)
- SPA: 10.0.2.15 (our IP)
- THA: 00:00:00:00:00:00 (unknown — broadcast)
- TPA: 10.0.2.1 (gateway IP we want to resolve)

Frame: Ethernet broadcast (dst=FF:FF:FF:FF:FF:FF, src=52:54:00:12:34:56, etype=0x0806)
       ARP request (htype=1, ptype=0x0800, hlen=6, plen=4, oper=1)

After gateway MAC is known:
- ARP cache: 10.0.2.1 → <gateway_mac> (oper=2 reply)
- Then ICMP echo to 10.0.2.1
- Then UDP/DNS to 10.0.2.3

Proof result: FINAL PASS 229/0/0 (e1000e).
Faults: 0.
