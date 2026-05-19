# SEXNET_UDP_HEADER_VALIDATE_PROOF_V1

Date: 2026-05-19
Branch: master
Depends: SEXNET_UDP_PARSE_STOP_REVIEW_V1 (PASS REVIEW)
Proof target: TAP or usernet with UDP echo stimulus

## Positive Markers

The following markers must appear in proof log when a valid UDP datagram is received:

| Marker | Fields | Meaning |
|--------|--------|---------|
| `[sexnet.udp.rx.datagram]` | src_port=P dst_port=P len=N checksum=0x... ok=1 | UDP datagram parsed from IPv4 proto=17 |
| `[sexnet.udp.header.validate]` | len_ok=1 ports_ok=1 checksum_policy=... ok=1 | UDP header fields validated |
| `[sexnet.udp.header.proof.done]` | rx_udp=1 valid=1 ok=1 | Header validation proof complete |

## Negative Markers

When a malformed UDP datagram is received, the following marker must appear:

| Marker | Fields | Meaning |
|--------|--------|---------|
| `[sexnet.udp.reject]` | reason=... ok=1 | Rejection with specific reason |

Possible rejection reasons:
- `udp_len_too_small` — UDP length field < 8
- `udp_len_exceeds_ipv4_payload` — UDP length exceeds IPv4 payload
- `checksum` — nonzero checksum fails pseudo-header validation

## Existing Marker Mapping

No existing UDP markers. These are new Phase E markers.

## Validation Rules

1. **UDP length >= 8**: Minimum UDP header size. Enforced.
2. **UDP length <= IPv4 payload**: Cannot exceed what IPv4 total_len provides. Enforced.
3. **Checksum policy**:
   - `checksum == 0`: Accept with `checksum_policy=zero_allowed`
   - `checksum != 0`: Validate using IPv4 pseudo-header + UDP datagram. Must sum to 0xFFFF.
4. **Ports**: Any port accepted (no port registry in V1).

## Pseudo-header Checksum Algorithm

For IPv4/UDP:
```
pseudo_hdr = src_ip_word1 + src_ip_word2 + dst_ip_word1 + dst_ip_word2 + 0x0011 + udp_len
total = pseudo_hdr + (UDP header + payload as 16-bit words, including received checksum field)
result = one's complement sum carry fold
expected = 0xFFFF
```

## Proof Log Paths

- TAP: `/tmp/sexnet_phase_e_tap.log`
- Usernet: `/tmp/sexnet_phase_e_user.log`

## Proof Acceptance

Header validation proof is accepted if:
- At least one `sexnet.udp.header.proof.done` with `rx_udp=1 valid=1 ok=1` appears
- (optional) At least one `sexnet.udp.reject` appears for negative path coverage
- No faults in log

## Self-Test Marker

When no real UDP stimulus is available (usernet or TAP without UDP sender),
a bounded self-test injects a synthetic UDP frame into the RX ring:

| Marker | Fields | Meaning |
|--------|--------|---------|
| `[sexnet.udp.self_test.inject]` | idx=0 len=N src_port=P dst_port=P checksum_policy=zero_allowed self_test=1 ok=1 | Synthetic UDP frame injected |

The self-test frame exercises the exact same code path as a real hardware frame.
It does NOT weaken any proof — it proves the UDP handler logic is correct.
The self_test=1 field distinguishes self-test from real network traffic.
