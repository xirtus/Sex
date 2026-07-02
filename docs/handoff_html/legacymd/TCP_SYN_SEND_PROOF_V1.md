# TCP_SYN_SEND_PROOF_V1 — Handoff

**Date:** 2026-05-17
**Status:** PASS IMPLEMENTED
**Previous:** TCP_SYN_BUILD_PROOF_V1 (PASS)

## SYN Send Table

| Field | Value |
|-------|-------|
| dst_ip (target) | 104.20.23.154 (DNS-resolved example.com) |
| src_port | 49153 |
| dst_port | 80 |
| seq | 0 |
| flags | SYN (0x02) |
| window | 65535 |
| MSS | 1460 |
| tdt_before | 0 |
| tdt_after | 1 |
| tx_dd | 1 |
| syn_sent | 1 |
| http_sent | 0 |
| fake | 0 |

## SYN-ACK RX Table

| Field | Value |
|-------|-------|
| syn_rx rounds | 1 (response in first poll round) |
| rx_dd total | 1 |
| tcp_seen | 1 |
| synack_seen | 1 |
| rst_seen | 0 |
| src_ip | 104.20.23.154 (matches target) |
| dst_ip | 10.0.2.15 (our IP) |
| src_port | 80 |
| dst_port | 49153 |
| flags | 0x12 (SYN+ACK) |
| ack_num | 1 (our seq=0, so ack=1 ✓) |
| peer_seq | 64001 (captured for final ACK) |

## Checksum Truth

| Checksum | Value | Verified |
|----------|-------|----------|
| IPv4 header | 0xEF0F | ✓ manual recomputation |
| TCP (pseudo-header) | 0x4C18 | ✓ manual recomputation |
| SYN-ACK IPv4 | not recomputed (ipv4_checksum_ok=1, real network) | defer |
| SYN-ACK TCP | not recomputed (tcp_checksum_checked=0 honest) | defer |

## Final ACK / HTTP Not-Sent Truth

| Field | Value |
|-------|-------|
| final_ack_sent | 0 |
| http_sent | 0 |
| syn_sent | 1 |
| tx_dd | 1 |
| synack_seen | 1 |
| rst_seen | 0 |

## Proof Result

| Metric | Value |
|--------|-------|
| Gates | **238 PASS** |
| Fail | 0 |
| Skip | 4 |
| Faults | 0 |
| New gates | 5 PASS (tcp_syn_tx_post_v1, tcp_syn_rx_synack_v1, tcp_syn_rx_synack_valid_v1, tcp_syn_truth_send_v1, tcp_syn_send_proof_done_v1) |
| Final | PASS |

## Markers Emitted

```
[tcp.syn.tx.post] dst_ip=104.20.23.154 src_port=49153 dst_port=80 seq=0 tdt_before=0 tdt_after=1 tx_dd=1 syn_sent=1 http_sent=0 fake=0 ok=1
[tcp.syn.rx.scan] round=1 rdh=5 rdt=7 rx_dd=1 tcp_seen=1 synack_seen=1 rst_seen=0 ok=1
[tcp.syn.rx.synack] rounds=1 rx_dd=1 tcp_seen=1 synack_seen=1 rst_seen=0 fake=0 ok=1
[tcp.syn.rx.synack.valid] src_ip=104.20.23.154 dst_ip=10.0.2.15 src_port=80 dst_port=49153 flags=0x12 ack_num=1 peer_seq=64001 ipv4_checksum_ok=1 tcp_checksum_checked=0 tcp_checksum_ok=0 ok=1
[tcp.syn.truth] sent=1 tx_dd=1 synack_seen=1 rst_seen=0 final_ack_sent=0 http_sent=0 fake=0 ok=1
[tcp.syn.send.proof.done] ok=1 sent=1 tx_dd=1 synack_seen=1 rst_seen=0 final_ack_sent=0 http_sent=0 fake=0
```

## Files Changed

1. `kernel/src/hal/pci.rs` — TCP SYN TX post + SYN-ACK RX poll after build
2. `scripts/daily_driver_master_gate.sh` — 5 new gate entries for TCP SYN send proof
3. `docs/handoff/TCP_SYN_SEND_PROOF_V1.md` — this handoff

## Next

- **TCP_HANDSHAKE_PROOF_V1** — send final ACK (ack=peer_seq+1), complete 3-way handshake
  - synack_seen=1 ✓, peer_seq=64001 ✓, ack_num=1 ✓
  - Needs: final ACK (flags=0x10, seq=1, ack=peer_seq+1=64002)

## ACID Review

- **A (Acid):** Real SYN sent to DNS-resolved example.com IP. Real SYN-ACK received with flags=0x12, ack_num=1, peer_seq=64001. No final ACK sent. No HTTP sent. No fake packets. No fabricated fields.
- **C (Clarity):** 5 new proof markers cover TX post, RX poll, SYN-ACK validation, truth, and done. SYN-ACK fields fully parsed from RX buffer bytes. Per-round scan markers for diagnostics.
- **I (Integrity):** Preserves all existing proofs. TX DD confirmed. SYN-ACK matched by IP src/dst + port pairs + flag check. honest: TCP checksum of SYN-ACK not verified (marked tcp_checksum_checked=0). e1000e lane only.
- **D (Depth):** 8 RX descriptors rearmed before send. 8 rounds × 500k spin polls. Selective rearm per consumed descriptor. SYN-ACK found in round 1 (fast SLiRP response). peer_seq captured for handshake completion.
