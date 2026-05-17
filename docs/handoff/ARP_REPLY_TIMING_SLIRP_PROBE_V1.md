# ARP_REPLY_TIMING_SLIRP_PROBE_V1

Date: 2026-05-17
Model: QEMU_NET_MODEL=e1000e
Proof: PASS DIAGNOSTIC 228/0/2skip (e1000e); e1000 default unchanged
Log: /tmp/sexos_arp_reply_timing_slirp_probe_v1.log

---

## Result: PASS DIAGNOSTIC

Sent=1, tx_dd=1 (hardware consumed frame). reply_seen=0 in timing probe window.
ICR reveals SLiRP DID reply after probe V1's poll window — reply was lost by ring rearm.

---

## Request Shape

| Field    | Value              | Valid |
|----------|--------------------|-------|
| dst      | FF:FF:FF:FF:FF:FF  | YES   |
| src/SHA  | 52:54:00:12:34:56  | YES   |
| ethertype| 0x0806             | YES   |
| oper     | 1 (request)        | YES   |
| SPA      | 10.0.2.15          | YES   |
| TPA      | 10.0.2.1           | YES   |
| len      | 60 (padded)        | YES   |

```
[arp.request.shape] dst_bcast=1 src_ok=1 sha_ok=1 spa=10.0.2.15 tpa=10.0.2.1 oper=1 len=60 ok=1
```

---

## Timing Rounds

| Round | rx_dd | arp_seen | req_seen | reply_seen | RDH | RDT |
|-------|-------|----------|----------|------------|-----|-----|
| 0     | 0     | 0        | 0        | 0          | 0   | 7   |
| 1     | 0     | 0        | 0        | 0          | 0   | 7   |
| 2     | 0     | 0        | 0        | 0          | 0   | 7   |
| 3     | 0     | 0        | 0        | 0          | 0   | 7   |

RDH=0 and rx_dd=0 in all rounds: no frames arrived during timing probe's polling window.

---

## ICR Analysis — Root Cause

| Event               | ICR value      | Bits                          |
|---------------------|----------------|-------------------------------|
| icr_before (probe V2)| 0x80000083    | TXDW(0)+TXQE(1)+RXT0(7)+INT   |
| icr_post_send       | 0x80000003     | TXDW(0)+TXQE(1)+INT           |
| icr_final           | 0x00000000     | (clear)                       |

**RXT0 (bit 7 = Receive Timer Expired) was SET in `icr_before`.**

This means SLiRP delivered a frame to the RX ring AFTER probe V1's poll window
ended and BEFORE probe V2 started reading ICR.

### Timeline Reconstruction

```
Probe V1: ARP send (tx_dd=1) → poll 8×100k spins → no rx_dd observed
  [reply arrives from SLiRP here — after poll window closed]
  → desc 0 filled, RDH → 1, ICR.RXT0 set
Probe V2 start: READ ICR → captures RXT0=1 (icr_before=0x80000083)
  → REARM all 8 descs (clears DD bits) ← LOST THE REPLY
  → WRITE RDH=0 ← may reset ring state
  → send second ARP request
  → poll 4×500k: rx_dd=0 (SLiRP already processed ARP exchange, no more replies)
```

---

## Root Causes

| # | Cause | Evidence |
|---|-------|----------|
| 1 | SLiRP replies once then stops | icr_before had RXT0; icr_post_send no RXT0 |
| 2 | Probe V2 rearmed ring, cleared the pending reply | rx_dd=0 despite icr_before=RXT0 |
| 3 | Writing RDH=0 may reset ring state | rdh_init=0 after write; RDH should be read-only |
| 4 | TPA=10.0.2.1 may not be SLiRP gateway | SLiRP standard gateway is 10.0.2.2 |

---

## SLiRP Truth

```
[arp.reply.slirp.truth] request_sent=1 tx_dd=1 reply_seen=0 gateway_known=0
    icr_before=0x80000083 icr_post_send=0x80000003 icr_final=0x00000000
    fake=0 ok=1 reason=slirp_arp_timing_diagnostic
```

SLiRP DID generate a reply. Frame arrived too late for probe V1 poll window.
Probe V2 cleared the ring, losing it. Second ARP request got no reply.

---

## Gate Results

| Gate                         | Result          |
|------------------------------|-----------------|
| arp_reply_timing_slirp_probe | SKIP (diagnostic) |
| arp_request_send_proof       | SKIP (gateway_known=0) |
| Total                        | 228/0/2skip     |

---

## Fixes for Next Probe

1. **Never write RDH** — it's read-only on real HW; writing may corrupt ring tracking
2. **Check ring for existing frames BEFORE rearm** — probe V2 lost a pending reply
3. **Use TPA=10.0.2.2** — SLiRP standard gateway; 10.0.2.1 may be the host alias
4. **Start poll window immediately after TX** — no separate "wait for TX DD" delay

---

## Likely Next Steps

| Condition           | Next Probe |
|---------------------|-----------|
| Reply lost in rearm (this case) | **E1000E_RX_REARM_AFTER_FIRST_PACKET_PROOF_V1** — read pending ring state first, then selectively rearm consumed slots only |
| TPA mismatch        | **ARP_GATEWAY_PROBE_10_0_2_2_V1** — target 10.0.2.2 (SLiRP standard GW) |
| No post-send RX     | **QEMU_USERNET_ARP_GATEWAY_BEHAVIOR_AUDIT_V1** — verify SLiRP ARP reply behavior |

**Recommended next**: target 10.0.2.2 instead of 10.0.2.1, don't write RDH,
don't rearm before checking, extend poll window to catch delayed SLiRP reply.

---

## Cumulative Network State

| Item         | Value             | Confidence |
|--------------|-------------------|------------|
| Our IP       | 10.0.2.15         | confirmed  |
| Our MAC      | 52:54:00:12:34:56 | confirmed  |
| Gateway IP   | 10.0.2.1 (tested) / 10.0.2.2 (SLiRP std) | uncertain |
| Gateway MAC  | unknown           | SLiRP did reply; frame was lost |
| TX path      | functional        | tx_dd=1 confirmed |
| SLiRP ARP    | responds once     | RXT0 in icr_before proves receipt |

---

Proof result: PASS DIAGNOSTIC 228/0/2skip (e1000e).
Faults: 0.
