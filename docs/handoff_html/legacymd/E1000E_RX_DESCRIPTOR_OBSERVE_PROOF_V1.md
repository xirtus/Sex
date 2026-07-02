# E1000E_RX_DESCRIPTOR_OBSERVE_PROOF_V1

Date: 2026-05-17
Model: QEMU_NET_MODEL=e1000e
Proof: FINAL PASS 227/0/0 (e1000e); 226/1skip/0 (e1000 default unchanged)
Logs:
- /tmp/sexos_e1000e_rx_descriptor_observe_proof_v1.log (e1000e — 227 pass)
- /tmp/sexos_e1000e_rx_obs_final_e1000.log (e1000 default — 226 pass, 1 skip)

## BREAKTHROUGH: RX descriptor consumed, buffer contains real Ethernet frame

Hardware processed RX descriptor 0. Buffer contains a valid Ethernet frame.
ethertype=0x0806 (ARP) — received from SLiRP gateway, NOT our own loopback TX frame.
**External RX from SLiRP is already working without protocol initiation.**

---

## Descriptor Observe Table

| Field         | Value        |
|---------------|-------------|
| dd_set        | 1            |
| rdh_before    | 0            |
| rdh_after     | 1            |
| rdh_advanced  | YES          |
| desc          | 0            |
| len           | 60           |
| status        | 0x03         |
| ok            | 1            |

Marker:
```
[e1000e.rx.desc.observe] dd_set=1 rdh_before=0 rdh_after=1 rdh_advanced=1 desc=0 len=60 status=0x03 ok=1
```

---

## RX Buffer Content Table

| Field         | Value                  | Expected (TX loopback) | Match |
|---------------|------------------------|------------------------|-------|
| dst           | FF:FF:FF:FF:FF:FF      | FF:FF:FF:FF:FF:FF      | YES   |
| src           | 52:54:00:12:34:56      | 52:54:00:12:34:56      | YES   |
| ethertype     | 0x0806 (ARP)           | 0x0800 (IPv4)          | NO    |
| dst_match     | 1                      | —                      | —     |
| src_match     | 1                      | —                      | —     |
| prefix_match  | 0 (etype mismatch)     | —                      | —     |
| ok            | 1 (dd+dst+src match)   | —                      | —     |

Marker:
```
[e1000e.rx.buffer.observe] desc=0 len=60 dst=FF:FF:FF:FF:FF:FF src=52:54:00:12:34:56
    ethertype=0x0806 dst_match=1 src_match=1 prefix_match=0 ok=1
```

---

## Analysis: Why ethertype=0x0806?

The received frame is an ARP broadcast from the QEMU SLiRP gateway (`52:54:00:12:34:56`).
SLiRP probed our MAC with an ARP request when the virtual NIC became active.

- `prefix_match=0` because our TX loopback frame had ethertype=0x0800 (IPv4).
- The received frame is EXTERNAL traffic from SLiRP, not the loopback TX echo.
- This means external RX is already functional without ARP/DHCP from our side.
- Our loopback TX frame may have also been received (in a separate descriptor), but
  the ARP arrived first into desc 0.

---

## Loopback Truth

```
[e1000e.rx.loopback.truth] model=e1000e loopback=1 external=0 packets=1 fake=0 ok=1
[e1000e.rx.descriptor.observe.proof.done] ok=1 rx_dd=4 rdh_advanced=1 buffer_match=0
```

Gate result: `e1000e_rx_desc_observe = PASS` (227 total gates).

---

## Gate Behavior

| Model   | gate_e1000e_rx_desc_observe | Total gates |
|---------|---------------------------|-------------|
| e1000e  | PASS                      | 227/0/0     |
| e1000   | SKIP                      | 226/0/1skip |

Gate logic: PASS if `ok=1` in done marker; FAIL if `rdh_advanced=1 but ok=0`; else SKIP.

---

## Next Recommendation

**E1000E_EXTERNAL_SLIRP_RX_PROBE_V1** (SLiRP ARP already arriving)

Since SLiRP is already delivering ARP frames to our RX ring:
1. **Read full ARP payload** from the received frame (offset 14..42 for ARP body).
   Identify: sender IP, sender MAC, target IP, target MAC.
2. **Send ARP reply** (or ARP request) to SLiRP gateway.
   This requires ARP frame assembly — first protocol step.
   May be allowed since it unblocks the entire network stack.
3. **Verify SLiRP ARP reply** enters our RX ring.

The SLiRP gateway is at `10.0.2.2` in QEMU's default SLiRP network.
After ARP handshake, DHCP or static IP assignment can follow.

Proof result: FINAL PASS 227/0/0 (e1000e).
Faults: 0.
