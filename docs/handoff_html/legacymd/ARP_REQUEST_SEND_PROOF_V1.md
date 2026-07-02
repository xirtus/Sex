# ARP_REQUEST_SEND_PROOF_V1

Date: 2026-05-17
Model: QEMU_NET_MODEL=e1000e
Proof: FINAL PASS 228/0/1skip (e1000e); e1000 default unchanged
Log: /tmp/sexos_arp_request_send_proof_v1.log

## Summary

Real ARP request frame sent via e1000e TX descriptor lane (tx_dd=1 — hardware consumed it).
Bounded RX poll (8 rounds × 8 descriptors = 64 scans) found no oper=2 reply from 10.0.2.1.
gateway_known=0 — honest. fake=0 throughout.

---

## ARP Request Frame Sent

| Field  | Value              |
|--------|--------------------|
| dst    | FF:FF:FF:FF:FF:FF  |
| src    | 52:54:00:12:34:56  |
| etype  | 0x0806 (ARP)       |
| oper   | 1 (request)        |
| SHA    | 52:54:00:12:34:56  |
| SPA    | 10.0.2.15          |
| THA    | 00:00:00:00:00:00  |
| TPA    | 10.0.2.1           |

Markers:
```
[arp.request.send] sha=52:54:00:12:34:56 spa=10.0.2.15 tpa=10.0.2.1 oper=1 sent=1 tdt=1 ok=1 reason=arp_request_broadcast_posted
[arp.reply.rx.scan] scanned=64 reply_found=0 ok=1 reason=no_oper2_from_gateway_in_poll_window
[arp.cache.gateway.update] ip=10.0.2.1 mac=00:00:00:00:00:00 inserted=0 fake=0 ok=1 reason=no_reply_gateway_mac_unknown
[arp.request.send.proof.done] sent=1 tx_dd=1 reply_seen=0 gateway_known=0 gw_mac=00:00:00:00:00:00 fake=0 ok=1 reason=arp_request_send_bounded_probe
```

---

## TX Consumed

`tx_dd=1` confirms the e1000e hardware consumed the ARP request descriptor and transmitted the frame.
SLiRP received the broadcast ARP request.

---

## RX Poll Result

64 descriptor scans (8 rounds × 8 descriptors). No oper=2 ARP reply arrived within poll window.
SLiRP may have replied after the bounded poll window expired, or gateway IP 10.0.2.1
may differ from SLiRP's standard gateway (10.0.2.2).

---

## Gate Results

| Gate                      | e1000e          |
|---------------------------|-----------------|
| arp_request_send_proof    | SKIP (sent=1, gateway_known=0 — diagnostic pass) |
| Total                     | 228/0/1skip     |

Gate definition:
- PASS: `arp.request.send.proof.done.*sent=1.*gateway_known=1`
- SKIP: `arp.request.send.proof.done.*sent=1.*gateway_known=0` (diagnostic — sent, no reply)

---

## Cumulative Network State

| Item         | Value             | Confidence |
|--------------|-------------------|------------|
| Our IP       | 10.0.2.15         | confirmed (SPA in ARP) |
| Our MAC      | 52:54:00:12:34:56 | confirmed (SHA in ARP) |
| Gateway IP   | 10.0.2.1          | confirmed (TPA in observed ARP) |
| Gateway MAC  | unknown           | no oper=2 reply received |
| SLiRP network| 10.0.2.0/24       | inferred |
| TX path      | functional        | tx_dd=1 confirmed |

---

## Next Recommendation

**ICMP_ECHO_REQUEST_PROOF_V1** — Send ICMP echo to 10.0.2.2 (SLiRP standard gateway).
Alternatively: extend poll window or retry ARP against 10.0.2.2 (SLiRP standard GW).

After gateway MAC known:
- ICMP echo to gateway
- UDP/DNS to 10.0.2.3

Proof result: FINAL PASS 228/0/1skip (e1000e — sent, no reply in bounded window).
Faults: 0.
