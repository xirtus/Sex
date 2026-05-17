# TCP_SYN_BUILD_PROOF_V1 — Handoff

**Date:** 2026-05-17
**Status:** PASS IMPLEMENTED
**Previous:** DNS_TO_HTTP_HOST_RESOLUTION_PROOF_V1 (PASS)

## Selected DNS Target

| Field | Value |
|-------|-------|
| Host | example.com |
| Resolved | 1 |
| Selected IP (q_a_ip[0]) | 172.66.147.243 |
| Alternate count | 1 |
| Source | dns_rx_observed |
| Fake | 0 |

## SYN Frame Table

| Layer | Offset | Size | Field | Value |
|-------|--------|------|-------|-------|
| Ethernet | 0-5 | 6 | dst_mac | 52:55:0A:00:02:02 (gateway) |
| Ethernet | 6-11 | 6 | src_mac | 52:54:00:12:34:56 (our) |
| Ethernet | 12-13 | 2 | ethertype | 0x0800 (IPv4) |
| IPv4 | 14 | 1 | ver+IHL | 0x45 (v4, IHL=5) |
| IPv4 | 15 | 1 | DSCP/ECN | 0x00 |
| IPv4 | 16-17 | 2 | total_len | 0x002C (44 bytes) |
| IPv4 | 18-19 | 2 | identification | 0x0000 |
| IPv4 | 20-21 | 2 | flags+frag | 0x0000 |
| IPv4 | 22 | 1 | TTL | 64 |
| IPv4 | 23 | 1 | protocol | 0x06 (TCP) |
| IPv4 | 24-25 | 2 | header csum | computed at runtime |
| IPv4 | 26-29 | 4 | src_ip | 10.0.2.15 |
| IPv4 | 30-33 | 4 | dst_ip | resolved DNS A record |
| TCP | 34-35 | 2 | src_port | 49153 (0xC001) |
| TCP | 36-37 | 2 | dst_port | 80 (0x0050) |
| TCP | 38-41 | 4 | seq | 0x00000000 |
| TCP | 42-45 | 4 | ack | 0x00000000 |
| TCP | 46 | 1 | data_offset | 0x60 (24 bytes = 6 words) |
| TCP | 47 | 1 | flags | 0x02 (SYN only) |
| TCP | 48-49 | 2 | window | 0xFFFF (65535) |
| TCP | 50-51 | 2 | checksum | computed with pseudo-header |
| TCP | 52-53 | 2 | urgent | 0x0000 |
| TCP | 54-55 | 2 | MSS kind+len | 0x0204 (kind=2, len=4) |
| TCP | 56-57 | 2 | MSS value | 0x05B4 (1460) |
| Pad | 58-59 | 2 | padding | 0x0000 (to min ethernet 60) |

## Checksum Table (for resolved IP 172.66.147.243)

| Checksum | Value | Method | Verified |
|----------|-------|--------|----------|
| IPv4 header | 0x2E88 | ones' complement (10 words, csum=0 placeholder) | ✓ manual |
| TCP (pseudo) | 0x8B90 | ones' complement (pseudo-header + TCP header + MSS opt) | ✓ manual |

## Not-Sent Truth

| Field | Value |
|-------|-------|
| built | 1 |
| syn_sent | 0 |
| tcp_sent | 0 |
| http_sent | 0 |
| fake | 0 |
| TX descriptor post | none |
| TDT advance | none |

## Proof Result

| Metric | Value |
|--------|-------|
| Gates | 233 PASS |
| Fail | 0 |
| Skip | 4 |
| Faults | 0 |
| New gates | 4 PASS (tcp_syn_build_v1, tcp_syn_checksum_v1, tcp_syn_truth_v1, tcp_syn_build_proof_done_v1) |

## Markers Emitted

```
[tcp.syn.build.frame] eth_dst=52:55:0A:00:02:02 eth_src=52:54:00:12:34:56 ethertype=0x0800 src_ip=10.0.2.15 dst_ip=172.66.147.243 proto=6 ttl=64 total_len=44 ok=1
[tcp.syn.build] src_ip=10.0.2.15 dst_ip=172.66.147.243 src_port=49153 dst_port=80 flags=SYN payload_len=0 ok=1
[tcp.syn.checksum] ipv4_checksum=0x2E88 tcp_checksum=0x8B90 pseudo=1 checksum_ok=1 ok=1
[tcp.syn.truth] built=1 syn_sent=0 tcp_sent=0 http_sent=0 fake=0 ok=1
[tcp.syn.build.proof.done] ok=1 built=1 sent=0 fake=0
```

## Files Changed

1. `kernel/src/hal/pci.rs` — TCP SYN build with resolved DNS IP + checksums; removed early-phase TCP/HTTP sends
2. `scripts/daily_driver_master_gate.sh` — added 4 new gate entries for TCP SYN build proof V1

## Next

1. `TCP_SYN_SEND_STOP_REVIEW_V1` — audit: is it safe to send SYN now?
2. `TCP_SYN_SEND_PROOF_V1` — post SYN to TX descriptor, await SYN-ACK

## ACID Review

- **A (Acid):** No TX post, no TDT advance, no HTTP GET send. Frame built in stack buffer only. Checksums computed from real resolved DNS IP. No fake IP. No fabricated checksum.
- **C (Clarity):** 4 proof markers cover build, checksum, truth, and done. Frame bytes defined byte-by-byte with offset table above. Gates check each marker independently.
- **I (Integrity):** Preserves existing DNS/ARP/ICMP proofs. e1000e lane only. Default e1000 skips cleanly (4 skip gates unchanged). No kernel ABI edits. No sex-pdx edits.
- **D (Depth):** Computes both IPv4 (ones' complement over 10 16-bit words) and TCP (ones' complement over pseudo-header + 12 TCP header words + MSS option) checksums at runtime using the DNS-resolved IP. Both verified manually against expected values.
