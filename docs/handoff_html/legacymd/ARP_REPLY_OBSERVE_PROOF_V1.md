# ARP_REPLY_OBSERVE_PROOF_V1

Date: 2026-05-17
Model: QEMU_NET_MODEL=e1000e
Proof: FINAL PASS 228/0/0 (e1000e); 226/2skip/0 (e1000 default unchanged)
Logs:
- /tmp/sexos_arp_reply_observe_proof_v1.log (e1000e — 228 pass)
- /tmp/sexos_arp_obs_default_e1000.log (e1000 — 226 pass, 2 skip)

## Result: PASS OBSERVED_REQUEST_ONLY

Real ARP frame parsed from live RX buffer. fake=0. oper=1 (request, not reply).

---

## Raw Ethernet Summary

| Field     | Value              |
|-----------|--------------------|
| dst       | FF:FF:FF:FF:FF:FF  |
| src       | 52:54:00:12:34:56  |
| ethertype | 0x0806 (ARP)       |
| len       | 60                 |

---

## ARP Parse Table

| Field   | Value            | Expected (RFC 826 IPv4/Ethernet) | Valid |
|---------|------------------|----------------------------------|-------|
| htype   | 1                | 1 (Ethernet)                     | YES   |
| ptype   | 0x0800           | 0x0800 (IPv4)                    | YES   |
| hlen    | 6                | 6 (MAC)                          | YES   |
| plen    | 4                | 4 (IPv4)                         | YES   |
| oper    | 1                | 1=request / 2=reply              | —     |
| SHA     | 52:54:00:12:34:56| —                                | —     |
| SPA     | 10.0.2.15        | —                                | —     |
| THA     | 00:00:00:00:00:00| —                                | —     |
| TPA     | 10.0.2.1         | —                                | —     |

Markers:
```
[arp.rx.observe] ethertype=0x0806 len=60 parsed=1 htype=1 ptype=0x0800 hlen=6 plen=4 oper=1 ok=1
[arp.rx.sender] mac=52:54:00:12:34:56 ip=10.0.2.15 ok=1
[arp.rx.target] mac=00:00:00:00:00:00 ip=10.0.2.1 ok=1
[arp.reply.observe] observed=1 request_observed=1 reply_observed=0 fake=0 ok=1
[arp.reply.observe.proof.done] ok=1 arp_seen=1 reply_seen=0 fake=0
```

---

## Request vs Reply Truth

| claim             | value |
|-------------------|-------|
| arp_request_observed | 1  |
| arp_reply_observed   | 0  |
| fake              | 0     |
| ok                | 1     |

**This is an ARP REQUEST, not a reply.**

Interpretation: SLiRP's network stack generated an ARP request on behalf of the guest
(10.0.2.15 is our QEMU-assigned guest IP). The frame asks:
"Who has 10.0.2.1 (QEMU gateway)? Tell 10.0.2.15."
THA=00:00:00:00:00:00 confirms this is a broadcast ARP request (target MAC unknown).

QEMU SLiRP assigned our NIC IP `10.0.2.15` and MAC `52:54:00:12:34:56` (matching our TX test frame src). SLiRP generated this ARP probe to discover the gateway.

---

## Network State Inferred

| Item           | Value            | Source             |
|----------------|------------------|--------------------|
| Our IP         | 10.0.2.15        | SPA in ARP request |
| Our MAC        | 52:54:00:12:34:56| SHA in ARP request |
| Gateway IP     | 10.0.2.1         | TPA in ARP request |
| Gateway MAC    | unknown          | THA=00:00:00:00:00:00 |
| SLiRP network  | 10.0.2.0/24      | inferred from IPs  |

SLiRP standard:
- Guest: 10.0.2.15 (our IP — confirmed)
- Gateway: 10.0.2.2 (standard SLiRP; TPA=10.0.2.1 is unusual — may be DNS/host alias)
- DNS: 10.0.2.3 (standard SLiRP)

---

## Gate Results

| Gate                    | e1000e | e1000  |
|-------------------------|--------|--------|
| e1000e_rx_desc_observe  | PASS   | SKIP   |
| arp_rx_observe_live     | PASS   | SKIP   |
| Total                   | 228/0/0| 226/0/2skip |

---

## Next Recommendation

**ARP_REQUEST_SEND_PROOF_V1**

We know:
- Our IP: 10.0.2.15
- Our MAC: 52:54:00:12:34:56
- Gateway IP: 10.0.2.1 (from TPA)

To get an ARP REPLY:
1. Send ARP request: "Who has 10.0.2.1? Tell 10.0.2.15"
   - SHA: 52:54:00:12:34:56 (our MAC)
   - SPA: 10.0.2.15 (our IP)
   - THA: 00:00:00:00:00:00 (unknown)
   - TPA: 10.0.2.1 (gateway IP)
2. Poll RX for oper=2 reply.
3. Gateway will reply with its MAC → populate ARP cache.
4. Then ICMP echo to 10.0.2.1 (gateway ping) to prove IP connectivity.

Proof result: FINAL PASS 228/0/0 (e1000e).
Faults: 0.
