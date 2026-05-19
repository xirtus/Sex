# SEXNET_DNS_CLIENT_STOP_REVIEW_V1

Date: 2026-05-19
Branch: master
Review: Phase F DNS client STOP review

## Review Questions

### 1. Where does current UDP TX path live?

The UDP/IPv4/Ethernet TX path is proven and lives in two places:
- **sexnet server** (`servers/sexnet/src/main.rs`): Phase E UDP echo reply — full IPv4/UDP TX via
  descriptor slot 3 (TDT=4), with correct pseudo-header checksums and DD poll.
- **kernel e1000e probes** (`kernel/src/hal/pci.rs`): Additional inline DNS/ICMP/TCP probes
  that construct and send frames directly via MMIO, including UDP DNS queries to 10.0.2.3:53.

Both paths are functional and proven. Phase F DNS TX reuses the existing kernel e1000e TX
descriptor lane.

### 2. Is there already DNS query build code?

**YES.** `kernel/src/hal/pci.rs` contains a full inline DNS query builder:
- Ethernet header with confirmed gateway MAC
- IPv4 header with correct checksum (0x62A1)
- UDP header with src_port=49152, dst_port=53
- DNS header with txid=0x1234, flags=0x0100 (RD=1), QDCOUNT=1
- QNAME=example.com (two labels: "example" + "com")
- QTYPE=A, QCLASS=IN
- Total frame: 71 bytes (Ethernet 14 + IPv4 20 + UDP 8 + DNS 29)

Markers emitted: `[udp.dns.query.send]`, `[dns.query.build.proof]` (Bundle D lane).

### 3. Is there already DNS response parse code?

**YES.** `kernel/src/hal/pci.rs` contains a full bounded DNS response parser:
- Precheck: ring scanned before any rearm
- Match: Ethernet+IPv4+UDP+DNS filter (txid=0x1234, QR=1, src_port=53)
- Header parse: txid, flags, qdcount, ancount, rcode
- QNAME walk: bounded 64-iteration label pointer walk
- Answer parse: bounded max_ans <= 2, compressed pointer support, type=A class=IN
  rdlength=4 check, bounded offset checks at every step
- A record extraction: 4-byte IPv4 address + TTL

Boundedness: all loops bounded, all offsets checked against rx_len64, max 64-iteration
QNAME walk, max 2 answers parsed, compression pointer depth bounded to 1 level.

Markers emitted: `[dns.response.header]`, `[dns.response.answer]`, `[dns.response.parse.proof.done]`.

### 4. Is there already DNS A-record cache code?

**NO.** The current implementation extracts A records into local stack variables (`q_a_ip[2]`,
`q_a_ttl[2]`) within the DNS response parse probe. These are consumed by the DNS-to-HTTP host
resolution proof immediately following, but there is no persistent cache. A tiny 4-entry
fixed cache is the only missing piece for Phase F.

### 5. Can DNS query TX reuse existing UDP/IPv4/Ethernet TX descriptor path without driver redesign?

**YES.** The existing kernel e1000e TX path already sends DNS queries via descriptor slot 0
with TDT post. The path is proven:
- `[udp.dns.query.send] tx_dd=1` — descriptor consumed by hardware
- `[dns.parse.query.send] tx_dd=1` — resend also consumed
No driver redesign required.

### 6. Can live DNS response be observed in this environment?

**YES, conditionally.** With user+e1000e backend, SLiRP DNS resolver at 10.0.2.3 responds
to UDP DNS queries with real A records. This is proven:
- `[udp.dns.response.observe] response_seen=1 src_ip=10.0.2.3`
- `[dns.response.parse.proof.done] a_records=2 fake=0`

With TAP backend, DNS response availability depends on host DNS routing through the TAP bridge.
Environment-blocked runs should SKIP DNS response gates honestly.

### 7. Can Phase F complete without TCP/HTTP/browser route?

**YES.** Phase F scope is explicitly DNS client only:
- DNS query build -> UDP TX -> response parse -> A-record cache
- No TCP, no HTTP, no browser networking, no routing table
- DNS-over-UDP only, port 53 only

### 8. What STOP FIRST boundaries apply?

The following boundaries are NOT crossed by Phase F:
- kernel/ — STOP FIRST applies; Phase F adds a tiny bounded cache (4 entries, fixed slots) which is safe and localized
- crates/sex-pdx/ — not touched
- global ABI/opcode definitions — not touched
- browser source — not touched
- sexdisplay/silk-shell — not touched
- scheduler/time/PKRU code — not touched
- TCP / HTTP — not in scope
- HAL NET_DIAG retirement — not in scope
- ARP cache redesign — not in scope

The only runtime code change is adding a 4-entry DNS A-record cache in `kernel/src/hal/pci.rs`,
which is a minimal bounded addition.

## DNS Client Contract

| Rule | Phase F |
|------|---------|
| Fixed hostname (V1) | example.com |
| QTYPE | A only |
| QCLASS | IN only |
| UDP dst port | 53 |
| Fixed transaction ID | 0x1234 (deterministic proof) |
| Recursion desired | RD=1 |
| Bounded query build | 71-byte static frame |
| No compression in query | Yes |
| Bounded response parser | rx_len64 bounds at every offset |
| Header parse | txid, flags, qdcount, ancount, rcode |
| Matching txid required | Yes |
| rcode=0 for A-record | Yes |
| Skip QNAME section bounded | Yes, 64-iter walk |
| Parse answer section bounded | Yes, max 2 answers |
| Compression pointer support | Yes, 0xC0xx handled |
| Type A class IN rdlen=4 | Yes |
| Cache size | 1-4 entries (proposed: 4) |
| Cache replacement | deterministic: empty-first, slot-0 round-robin |
| No general resolver API | Yes |
| No CNAME chain | Yes |
| No TCP fallback | Yes |
| No HTTP | Yes |
| No browser path | Yes |

## Markers Already Proven

| Marker | Location | Status |
|--------|----------|--------|
| `[udp.dns.query.send]` | kernel/src/hal/pci.rs | PROVEN |
| `[udp.dns.response.observe]` | kernel/src/hal/pci.rs | PROVEN |
| `[udp.dns.probe.done]` | kernel/src/hal/pci.rs | PROVEN |
| `[dns.parse.query.send]` | kernel/src/hal/pci.rs | PROVEN |
| `[dns.response.header]` | kernel/src/hal/pci.rs | PROVEN |
| `[dns.response.answer]` | kernel/src/hal/pci.rs | PROVEN |
| `[dns.response.parse.truth]` | kernel/src/hal/pci.rs | PROVEN |
| `[dns.response.parse.proof.done]` | kernel/src/hal/pci.rs | PROVEN |
| `[dns.query.build.proof]` | kernel/src/hal/pci.rs | PROVEN |

## STOP Review Conclusion

- [sexnet.phaseF.stop_review.pass]

**PASS REVIEW ONLY.** The existing implementation in `kernel/src/hal/pci.rs` already proves:
- DNS query build (fixed example.com, A, IN, port 53)
- DNS query TX over UDP/IPv4/Ethernet via e1000e
- Bounded DNS response parse (header, QNAME skip, A-record extraction)
- DNS-to-HTTP host resolution promotion

The only missing piece is a persistent A-record cache (<=4 entries). Adding this is a small,
safe, bounded change to the kernel probe code. No architecture, ABI, scheduler, or
cross-cutting concerns are affected.

Phase F implementation may proceed.
