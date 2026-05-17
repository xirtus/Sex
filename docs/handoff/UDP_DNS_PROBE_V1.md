# UDP_DNS_PROBE_V1

Date: 2026-05-17
Model: QEMU_NET_MODEL=e1000e
Proof: FINAL PASS IMPLEMENTED 230/0/2skip (e1000e); e1000 default unchanged
Log: /tmp/sexos_udp_dns_probe_v1.log

---

## Result: PASS IMPLEMENTED

Real UDP DNS query for example.com sent to 10.0.2.3:53.
Real DNS response received in round 0. txid_match=1 qr=1 ancount=2. fake=0.

---

## Precheck Result

Ring scanned before any rearm or send.

```
[udp.dns.query.precheck] dd=0 icr=0x00000000 rdh=2 rdt=1 ok=1 reason=precheck_before_dns_send
```

No pending frames. ICR clean. Ring ready.

---

## UDP DNS Query

### Frame Layout

| Layer    | Field         | Value                |
|----------|---------------|----------------------|
| Ethernet | dst MAC       | 52:55:0A:00:02:02    |
| Ethernet | src MAC       | 52:54:00:12:34:56    |
| Ethernet | ethertype     | 0x0800               |
| IPv4     | src           | 10.0.2.15            |
| IPv4     | dst           | 10.0.2.3             |
| IPv4     | proto         | 17 (UDP)             |
| IPv4     | TTL           | 64                   |
| IPv4     | total_len     | 57                   |
| UDP      | src_port      | 49152                |
| UDP      | dst_port      | 53                   |
| UDP      | udp_len       | 37                   |
| UDP      | checksum      | 0 (omitted)          |
| DNS      | txid          | 0x1234               |
| DNS      | flags         | 0x0100 (RD=1)        |
| DNS      | QDCOUNT       | 1                    |
| DNS      | QNAME         | example.com          |
| DNS      | QTYPE         | A (1)                |
| DNS      | QCLASS        | IN (1)               |

### Checksum Table

| Checksum | Computed | Expected | Match |
|----------|----------|----------|-------|
| IPv4     | 0x62A1   | 0x62A1   | YES   |
| UDP      | 0x0000   | 0 (omitted) | N/A |

```
[udp.dns.query.send] dst_ip=10.0.2.3 dst_port=53 src_port=49152 tx_dd=1 ipv4_checksum_ok=1 udp_len=37 dns_len=29 fake=0 ok=1 reason=udp_dns_query_to_slirp_dns
```

tx_dd=1: hardware consumed the frame.

---

## RX Scan Table

| Round | ICR        | rx_dd | ipv4 | udp | dns | response | RDH | RDT | Result |
|-------|------------|-------|------|-----|-----|----------|-----|-----|--------|
| 0     | 0x80000083 | 1     | 1    | 1   | 1   | 1        | 3   | 1   | RESPONSE FOUND |

Response found in round 0. ICR had RXT0 (bit 7) set.

---

## DNS Response Truth

```
[udp.dns.response.observe] src_ip=10.0.2.3 src_port=53 dst_port=49152 txid_match=1 qr=1 ancount=2 response_seen=1 fake=0 ok=1 reason=dns_response_classification
[udp.dns.probe.done] ok=1 sent=1 tx_dd=1 response_seen=1 fake=0
```

| Field       | Value      |
|-------------|------------|
| src_ip      | 10.0.2.3   |
| src_port    | 53         |
| dst_port    | 49152      |
| txid_match  | 1          |
| QR          | 1 (response) |
| ANCOUNT     | 2 (two A records for example.com) |
| response_seen | 1        |
| fake        | 0          |

SLiRP DNS resolver at 10.0.2.3 responded with 2 A records for example.com in round 0.

---

## Gate Results

| Gate           | Result |
|----------------|--------|
| udp_dns_probe  | PASS   |
| Total          | 230/0/2skip |

---

## Cumulative Network State

| Item             | Value                | Confidence |
|------------------|----------------------|------------|
| Our IP           | 10.0.2.15            | confirmed  |
| Our MAC          | 52:54:00:12:34:56    | confirmed  |
| Gateway IP       | 10.0.2.2             | confirmed  |
| Gateway MAC      | 52:55:0A:00:02:02    | confirmed  |
| DNS server IP    | 10.0.2.3             | confirmed  |
| DNS server port  | 53                   | confirmed  |
| SLiRP network    | 10.0.2.0/24          | confirmed  |
| TX path          | functional           | tx_dd=1    |
| RX path          | functional           | dns reply round 0 |
| IPv4 TX          | functional           | checksum_ok=1 |
| ICMP round-trip  | functional           | prior proof |
| UDP round-trip   | functional           | txid_match=1 qr=1 ancount=2 |

---

## Probe Discipline Applied

| Rule | Applied |
|------|---------|
| Precheck ring before rearm | YES — dd=0 confirmed clean |
| Never write RDH | YES |
| Selective rearm only consumed descs | YES |
| Use confirmed gateway MAC | YES — 52:55:0A:00:02:02 |
| Bounded poll window | YES — 8 rounds × 500k spins |
| No fake response | YES — fake=0 throughout |

---

## Next: DNS_RESPONSE_PARSE_PROOF_V1

ancount=2 confirmed. Next probe: parse the two A record answers from the DNS response.
Fields to extract: name, type, class, TTL, rdlength, rdata (4-byte IPv4 address).
Emit per-answer markers. Prove at least one valid A record with non-zero IP.

Proof result: FINAL PASS IMPLEMENTED 230/0/2skip (e1000e).
Faults: 0.
