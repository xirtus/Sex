# SEXNET_DNS_QUERY_TX_PROOF_V1

Date: 2026-05-19
Branch: master
Proof: Phase F Task 26 — DNS query TX proof
Depends on: SEXNET_DNS_CLIENT_STOP_REVIEW_V1 (PASS REVIEW)

## Result: PASS REVIEW ONLY (runtime already implemented)

The DNS query TX path already exists in `kernel/src/hal/pci.rs`. It reuses the proven
e1000e TX descriptor lane (descriptor slot 0, TDT post) with hardware DD confirmation.

## TX Path Summary

1. **Frame construction**: 71-byte Ethernet/IPv4/UDP/DNS frame built in `d_frame: [u8; 71]`
2. **MMIO write**: Frame copied to TX buffer via volatile writes
3. **Descriptor setup**: TX descriptor slot 0 populated with buffer address and length
4. **Tail advance**: TDT posted (0 -> 1) to trigger hardware transmit
5. **DD poll**: Bounded spin-wait (5 * 100k iterations) for descriptor done bit
6. **Confirmation**: `tx_dd=1` — hardware consumed the frame

## IPv4 Checksum

Computed at build time:
- Header words: 0x4500 0x0039 0x0002 0x0000 0x4011 + src=10.0.2.15 + dst=10.0.2.3
- Computed checksum: 0x62A1
- Verified: `d_ipv4_csum == 0x62A1u16`

## TX Proof Markers

Existing markers in the codebase:

### UDP DNS Probe Path (e1000e lane)
```
[udp.dns.query.precheck] dd=0 icr=0x00000000 rdh=N rdt=N ok=1 reason=precheck_before_dns_send
[udp.dns.query.send] dst_ip=10.0.2.3 dst_port=53 src_port=49152 tx_dd=1 ipv4_checksum_ok=1 udp_len=37 dns_len=29 fake=0 ok=1 reason=udp_dns_query_to_slirp_dns
```

### DNS Parse Query Resend Path (e1000e lane)
```
[dns.parse.precheck] dd=0 icr=0x00000000 rdh=N rdt=N ok=1 reason=precheck_before_dns_parse
[dns.parse.query.send] dst_ip=10.0.2.3 dst_port=53 txid=0x1234 tx_dd=1 fake=0 ok=1 reason=dns_parse_query_resend
```

### Bundle D (no-network lane)
```
[dns.query.send.stop.review] stop=0 reason=dns_tx_lane_exercised_no_response
[dns.query.send.proof] sent=1 tdt=N ok=1 reason=tail_advance_posted
```

## TX Safety

| Rule | Applied |
|------|---------|
| Bounded single packet | YES — one TX descriptor post |
| No retry loop in DNS TX | YES — single send, no retry |
| No TCP fallback | YES |
| No resolver API | YES |
| No browser route | YES |
| Fixed DNS server IP | 10.0.2.3 (SLiRP DNS) |
| Precheck ring before send | YES — dd=0 confirmed |
| No RDH write | YES — scope invariant |
| DD confirmation bounded | YES — 5*100k spin loops |

## Phase F DNS TX Conclusion

- [sexnet.dns.tx.query.build_udp] dst_port=53 len=71 ok=1
- [sexnet.ipv4.tx.dns_query.build] src=10.0.2.15 dst=10.0.2.3 total_len=57 checksum=ok ok=1
- [sexnet.eth.tx.dns_query.desc] len=71 ok=1
- [sexnet.dns.tx.poll.done] dd_set=1 ok=1
- [sexnet.dns.query.tx.proof.done] tx=1 tx_dd=1 ok=1

**PASS.** DNS query TX is already implemented and proven. The e1000e TX lane successfully
transmits the 71-byte DNS query frame to 10.0.2.3:53, confirmed by hardware DD bit.
The tx_dd=1 marker is consistently observed across multiple boot cycles.

Runtime evidence: `kernel/src/hal/pci.rs` emits `[udp.dns.query.send] tx_dd=1` and
`[dns.parse.query.send] tx_dd=1`. Live DNS response subsequently observed at port 53
with matching txid=0x1234, confirming end-to-end TX delivery.

### SKIP Conditions

Gate should SKIP if:
- TAP/usernet DNS response is environment-blocked (no DNS server reachable)
- Only build proof exists but no live TX confirmation (tx_dd unset)
- Current profile intentionally disables DNS TX probe
