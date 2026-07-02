# ARP_REPLY_CAPTURE_FIX_V1

Date: 2026-05-17
Model: QEMU_NET_MODEL=e1000e
Proof: FINAL PASS IMPLEMENTED 229/0/2skip (e1000e); e1000 default unchanged
Log: /tmp/sexos_arp_reply_capture_fix_v1.log

---

## Result: PASS IMPLEMENTED

ARP request sent to 10.0.2.2 (SLiRP standard gateway).
Real ARP reply received in round 0. Gateway MAC confirmed. fake=0. rdh_written=0.

---

## Precheck Result

Ring scanned before any rearm or send.

```
[arp.reply.capture.precheck] dd=0 arp=0 reply=0 icr=0x00000000 rdh=0 rdt=7 ok=1
```

No pending frames. ICR clean. Ring ready at RDH=0, RDT=7.

---

## ARP Request

| Field  | Value              |
|--------|--------------------|
| dst    | FF:FF:FF:FF:FF:FF  |
| src    | 52:54:00:12:34:56  |
| etype  | 0x0806             |
| oper   | 1 (request)        |
| SHA    | 52:54:00:12:34:56  |
| SPA    | 10.0.2.15          |
| TPA    | 10.0.2.2           |
| tx_dd  | 1 (consumed)       |

```
[arp.request.send] target_ip=10.0.2.2 sender_ip=10.0.2.15 tx_posted=1 tx_dd=1 fake=0 ok=1
```

---

## Capture Rounds

| Round | ICR        | dd | arp | reply | RDH | RDT | Result |
|-------|------------|----|----|-------|-----|-----|--------|
| 0     | 0x80000083 | 1  | 1  | 1     | 1   | 7   | REPLY FOUND |

Reply found in round 0. ICR had RXT0 (bit 7) set — frame had already arrived.
One round × 500k spins was sufficient.

---

## Gateway MAC Truth

```
[arp.reply.capture.gateway] ip=10.0.2.2 mac=52:55:0A:00:02:02 known=1 fake=0 ok=1
[arp.reply.capture.fix.done] ok=1 sent=1 reply_seen=1 gateway_known=1 rdh_written=0 fake=0
    gw_mac=52:55:0A:00:02:02 rx_dd_total=1 arp_total=1 reply_total=1 send_tdt=1
```

**Gateway MAC: 52:55:0A:00:02:02** — SLiRP virtual gateway MAC.

---

## Fixes Applied

| Fix | Before | After |
|-----|--------|-------|
| Ring precheck | rearm before check | scan first, rearm only consumed |
| RDH write | `write_volatile(RDH, 0)` | removed — never written |
| Target IP | TPA=10.0.2.1 | TPA=10.0.2.2 (SLiRP std GW) |
| Poll window | 4×100k (probe V1), 4×500k (timing) | 8×500k with per-round rearm |

---

## Cumulative Network State

| Item         | Value                | Confidence |
|--------------|----------------------|------------|
| Our IP       | 10.0.2.15            | confirmed  |
| Our MAC      | 52:54:00:12:34:56    | confirmed  |
| Gateway IP   | 10.0.2.2             | confirmed (oper=2 SPA) |
| Gateway MAC  | 52:55:0A:00:02:02    | confirmed (oper=2 SHA) |
| SLiRP network| 10.0.2.0/24          | confirmed  |
| TX path      | functional           | tx_dd=1    |
| RX path      | functional           | rdh=1 reply seen round 0 |

---

## Gate Results

| Gate                  | Result |
|-----------------------|--------|
| arp_reply_capture_fix | PASS   |
| Total                 | 229/0/2skip |

---

## RDH Write Removal

`rdh_written=0` confirmed in proof marker.
No `write_volatile((virt + 0x2810), ...)` in this probe.

---

## Next: ICMP_ECHO_REQUEST_PROOF_V1

All fields needed for ICMP echo:
- src_mac: 52:54:00:12:34:56
- src_ip: 10.0.2.15
- gw_mac: 52:55:0A:00:02:02
- gw_ip: 10.0.2.2
- Ethernet: dst=gw_mac, src=our_mac, etype=0x0800
- IPv4: src=10.0.2.15, dst=10.0.2.2, proto=ICMP(1), ttl=64
- ICMP: type=8, code=0, id=0x4444, seq=1

Proof result: FINAL PASS IMPLEMENTED 229/0/2skip (e1000e).
Faults: 0.
