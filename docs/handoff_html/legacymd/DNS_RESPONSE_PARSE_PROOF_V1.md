# DNS_RESPONSE_PARSE_PROOF_V1

Date: 2026-05-17
Model: QEMU_NET_MODEL=e1000e
Proof: FINAL PASS IMPLEMENTED 231/0/2skip (e1000e); e1000 default unchanged
Log: /tmp/sexos_dns_response_parse_proof_v1.log

---

## Result: PASS IMPLEMENTED

Resent DNS query for example.com A. Real DNS response received, header and A record
answers parsed from RX buffer. Two A records extracted. Bounded, no heap, no fake.

---

## Precheck Result

```
[dns.parse.precheck] dd=0 icr=0x00000000 rdh=3 rdt=2 ok=1 reason=precheck_before_dns_parse
```

Ring clean before send.

---

## Query Resend

```
[dns.parse.query.send] dst_ip=10.0.2.3 dst_port=53 txid=0x1234 tx_dd=1 fake=0 ok=1 reason=dns_parse_query_resend
```

tx_dd=1: hardware consumed frame. Same query as UDP_DNS_PROBE_V1.

---

## DNS Header Table

```
[dns.response.header] txid=0x1234 qr=1 qd=1 an=2 ns=0 ar=0 rcode=0 ok=1 reason=dns_response_header_parsed
```

| Field   | Value  |
|---------|--------|
| txid    | 0x1234 |
| QR      | 1 (response) |
| QDCOUNT | 1      |
| ANCOUNT | 2      |
| NSCOUNT | 0      |
| ARCOUNT | 0      |
| RCODE   | 0 (NOERROR) |

---

## Answer Table

```
[dns.response.answer] idx=0 type=1 class=1 ttl=223 rdlen=4 a=104.20.23.154 ok=1 reason=dns_a_record_extracted
[dns.response.answer] idx=1 type=1 class=1 ttl=223 rdlen=4 a=172.66.147.243 ok=1 reason=dns_a_record_extracted
```

| idx | type | class | TTL | rdlen | A record IP    |
|-----|------|-------|-----|-------|----------------|
| 0   | 1    | 1     | 223 | 4     | 104.20.23.154  |
| 1   | 1    | 1     | 223 | 4     | 172.66.147.243 |

---

## Extracted A Records

| # | IP Address      | Notes |
|---|-----------------|-------|
| 0 | 104.20.23.154   | Cloudflare anycast (example.com) |
| 1 | 172.66.147.243  | Cloudflare anycast (example.com) |

```
[dns.response.parse.truth] parsed=2 a_records=2 a0=104.20.23.154 fake=0 bounded=1 ok=1 reason=dns_answer_parse_complete
[dns.response.parse.proof.done] ok=1 a_records=2 fake=0
```

---

## RX Scan Table

| Round | ICR        | rx_dd | response | a_records | RDH | RDT | Result |
|-------|------------|-------|----------|-----------|-----|-----|--------|
| 0     | 0x80000083 | 1     | 1        | 2         | 4   | 2   | PARSED |

Response and parse complete in round 0.

---

## Parse Discipline

| Rule | Applied |
|------|---------|
| Precheck before rearm | YES |
| Never write RDH | YES |
| Selective rearm | YES |
| Bounded QNAME walk | YES — 64-iteration limit |
| Compressed pointer support | YES — 0xC0xx handled |
| Bounds check every offset | YES — all reads check `< rx_len64` |
| No heap | YES — stack only |
| No fake answers | YES — fake=0 |
| At most 2 answers parsed | YES — max_ans = min(ancount,2) |

---

## Gate Results

| Gate                      | Result |
|---------------------------|--------|
| dns_response_parse_proof  | PASS   |
| Total                     | 231/0/2skip |

---

## Cumulative Network State

| Item              | Value                  | Confidence |
|-------------------|------------------------|------------|
| Our IP            | 10.0.2.15              | confirmed  |
| Our MAC           | 52:54:00:12:34:56      | confirmed  |
| Gateway IP        | 10.0.2.2               | confirmed  |
| Gateway MAC       | 52:55:0A:00:02:02      | confirmed  |
| DNS server IP     | 10.0.2.3               | confirmed  |
| example.com A[0]  | 104.20.23.154          | confirmed (SLiRP live DNS) |
| example.com A[1]  | 172.66.147.243         | confirmed (SLiRP live DNS) |
| UDP/DNS round-trip| functional             | prior proof |
| DNS parse         | functional             | ancount=2 extracted |

---

## Next: DNS_TO_HTTP_HOST_RESOLUTION_PROOF_V1

Resolved IPs for example.com available in memory.
Next: emit resolution proof marker with confirmed IPs, then use IP for outbound HTTP probe.

Proof result: FINAL PASS IMPLEMENTED 231/0/2skip (e1000e).
Faults: 0.
