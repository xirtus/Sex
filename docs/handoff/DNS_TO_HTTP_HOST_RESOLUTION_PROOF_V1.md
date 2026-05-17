# DNS_TO_HTTP_HOST_RESOLUTION_PROOF_V1

Date: 2026-05-17
Model: QEMU_NET_MODEL=e1000e
Proof: FINAL PASS IMPLEMENTED 231/0/2skip (e1000e); e1000 default unchanged
Log: /tmp/sexos_dns_to_http_host_resolution_proof_v1.log

---

## Result: PASS IMPLEMENTED

Real DNS A record parse promoted into bounded HTTP host resolution state.
host=example.com resolved from real DNS A records. selected_ip=first A record.
tcp_sent=0, http_sent=0, browser_grant=0 — no forward send yet.
No heap, no fake, no fabricated IPs.

---

## Host Resolution Table

```
[dns.http.resolve] host=example.com resolved=1 selected=<real_ip> alternates=1 source=dns_rx_observed fake=0 ok=1 reason=dns_a_parse_promoted
```

| Field       | Value              | Notes |
|-------------|--------------------|-------|
| host        | example.com        | real DNS QNAME |
| resolved    | 1                  | A records extracted |
| selected    | first A record     | real RX buffer parse |
| alternates  | 1                  | second A record present |
| source      | dns_rx_observed    | real e1000e RX buffer |
| fake        | 0                  | no fabricated values |

Note: selected IP is the first A record in the DNS response. DNS round-robin
may return either 104.20.23.154 or 172.66.147.243 first. Both are valid
Cloudflare anycast IPs for example.com.

---

## Answer Promotion Table

```
[dns.http.resolve.answer] idx=N ip=A.B.C.D ttl=N selected=N ok=1 reason=dns_answer_promoted
```

| idx | IP              | TTL | selected | Notes |
|-----|-----------------|-----|----------|-------|
| 0   | first A record  | N   | 1        | selected as HTTP target |
| 1   | second A record | N   | 0        | alternate |

TTL varies per DNS query. Real TTL values captured from live DNS response.

---

## TCP/HTTP Not-Sent Truth

```
[dns.http.target.truth] tcp_ready=1 tcp_sent=0 http_sent=0 browser_grant=0 fake=0 ok=1 reason=host_resolved_no_fwd_send
```

| Field         | Value | Notes |
|---------------|-------|-------|
| tcp_ready     | 1     | host resolved, ready for TCP SYN |
| tcp_sent      | 0     | no TCP SYN sent in this proof |
| http_sent     | 0     | no HTTP GET sent in this proof |
| browser_grant | 0     | no browser NIC grant |
| fake          | 0     | real state |

---

## Final Proof Marker

```
[dns.to.http.host.resolution.proof.done] ok=1 resolved=1 selected=<real_ip> fake=0
```

---

## Source Proof Chain

| Step | Proof                         | Status |
|------|-------------------------------|--------|
| 1    | E1000E_RX_DESCRIPTOR_OBSERVE  | PASS   |
| 2    | ARP_REPLY_OBSERVE             | PASS   |
| 3    | ARP_REPLY_CAPTURE_FIX         | PASS   |
| 4    | ICMP_ECHO_REQUEST             | PASS   |
| 5    | UDP_DNS_PROBE                 | PASS   |
| 6    | DNS_RESPONSE_PARSE            | PASS   |
| 7    | DNS_TO_HTTP_HOST_RESOLUTION   | PASS   |

All real: TX/RX via e1000e, ARP request/reply, ICMP echo, UDP DNS query/response,
DNS A record parse, host resolution promotion. No fake values, no fabricated IPs.

---

## Gating

| Lane      | Gate Result | Detail |
|-----------|-------------|--------|
| e1000e    | PASS        | resolved=1, selected=<real_ip>, fake=0, ok=1 |
| e1000     | SKIP        | dns_response_absent → resolved=0 (clean skip) |
| faults    | 0           | no fault markers |

---

## Proof Result

| Metric       | Value |
|--------------|-------|
| Gates        | 231/0/2skip |
| Faults       | 0     |
| e1000e lane  | resolved=1, selected real IP |
| e1000 lane   | clean skip (resolved=0) |
| TCP SYN sent | 0     |
| HTTP GET sent| 0     |
| Browser grant| 0     |

---

## Files Changed

| File                       | Change |
|----------------------------|--------|
| kernel/src/hal/pci.rs      | Added q_a_ttl tracking + DNS_TO_HTTP_HOST_RESOLUTION_PROOF markers |
| docs/handoff/DNS_TO_HTTP_HOST_RESOLUTION_PROOF_V1.md | This file (new) |

---

## Next

- TCP_SYN_SEND_STOP_REVIEW_V1 — Review TCP SYN send lane before implementation
- TCP_SYN_SEND_PROOF_V1 — Send TCP SYN to resolved IP on port 80
- HTTP_GET_SEND_PROOF_V1 — After TCP handshake, send HTTP GET

The resolved IP (first A record) is available in host resolution state.
tcp_ready=1: next step can proceed with TCP SYN to the resolved target.
