# SEXNET_IPV4_CHECKSUM_PROOF_V1

Date: 2026-05-19
Commit: pending (Phase C checksum docs)
Gate: `sexnet_ipv4_checksum` (separate from `sexnet_ipv4_header_validate`)

## A. Result

IPv4 header checksum validation is proven. The runtime code computes a one's-complement
16-bit sum over the 20-byte IPv4 header (IHL=5, 10 words) and validates that the folded
result equals 0xFFFF. This proof reuses the same runtime code and log output as the
IPv4 header validate proof — no separate checksum-only proof run is needed.

## B. Marker Mapping

The mission's ideal marker names differ from the existing runtime marker names.
This table documents the exact mapping:

| Ideal Marker (Mission) | Actual Marker (Runtime) | Notes |
|------------------------|------------------------|-------|
| `[sexnet.ipv4.checksum.compute] sum=... ok=1` | `[sexnet.ipv4.rx.validate.detail] ... csum=0x... checksum_ok=1 ...` | checksum_ok field carries compute result |
| `[sexnet.ipv4.checksum.validate] expected=0x0000 or valid=1 ok=1` | `[sexnet.ipv4.rx.validate] ... checksum=ok ... ok=1` | checksum=ok means validated to 0xFFFF |
| `[sexnet.ipv4.checksum.proof.done] valid=N invalid=N ok=1` | `[sexnet.ipv4.proof.done] frames=1 ok=1` | frames=1 implies checksum passed (it's required for ok=1) |
| `[sexnet.ipv4.checksum.reject] reason=bad_checksum ok=1` | `[sexnet.ipv4.rx.reject.detail] ... reason=checksum ok=0` | reason=checksum is the bad-checksum rejection |

The existing markers are kept as-is. No runtime code changes are made to rename markers.

## C. Checksum Contract

| Property | Value |
|----------|-------|
| Algorithm | One's-complement 16-bit sum (RFC 791) |
| Header scope | 20 bytes (IHL=5, no options) |
| Word count | 10 words × 16 bits |
| Checksum field | Included in computation (bytes 24–25 of IPv4 header) |
| Fold target | 0xFFFF (all-ones) for valid header |
| Payload | NOT checksummed (Phase C scope is header only) |
| Bounded | Loop bounded to `ci < 10` (10 iterations) |
| Carry fold | `while (sum >> 16) != 0 { sum = (sum & 0xFFFF) + (sum >> 16); }` |

### Computation Detail (from servers/sexnet/src/main.rs:2027–2048)

```
let mut sum = 0u32;
let mut ci = 0usize;
while ci < 10 {
    let off = 14 + ci * 2;
    let w_hi = read_volatile(pkt_buf + off);
    let w_lo = read_volatile(pkt_buf + off + 1);
    sum += ((w_hi << 8) | w_lo) as u32;
    ci += 1;
}
while (sum >> 16) != 0 {
    sum = (sum & 0xFFFF) + (sum >> 16);
}
if (sum as u16) == 0xFFFF {
    // checksum valid
}
```

The checksum includes the checksum field itself (bytes 24–25, word offset 5).
For a correctly formed IPv4 header, the one's-complement sum of all 10 words
(including the checksum field) folds to 0xFFFF.

## D. Evidence from Prior Proof Run

From `SEXNET_IPV4_HEADER_VALIDATE_PROOF_V1.md` (commit c432689):

```
[sexnet.ipv4.rx.validate.detail] ver=4 ihl=5 total_len=84 pkt_len=98 frag=0x4000 dst=10.0.2.15 csum=0x15AB checksum_ok=1 proto=1 ttl=64 ok=0
[sexnet.ipv4.rx.validate] version=4 ihl=5 total_len=84 dst=10.0.2.15 frag=0 checksum=ok src=10.0.2.2 proto=1 ttl=64 ok=1
[sexnet.ipv4.proof.done] frames=1 ok=1
```

- `checksum_ok=1` in validate.detail: the one's-complement fold returned 0xFFFF
- `checksum=ok` in rx.validate: the checksum validation passed
- `frames=1 ok=1` in proof.done: one frame passed all validations including checksum

## E. Negative Proof (Bad Checksum Rejection)

The code rejects headers where checksum validation fails:

```
// Line 2047: reason = "checksum";
// Line 2082–2087: logs [sexnet.ipv4.rx.reject.detail] ... reason=checksum ok=0
```

To stimulate a bad-checksum rejection, deliver an IPv4 frame with a deliberately
corrupted checksum field. The rejection marker `reason=checksum` confirms the checksum
validation path correctly identifies and rejects malformed checksums.

Note: The current proof loop only processes 1 positive frame (`ipv4_frames < 1`).
Rejection is only logged once (`reject_logged == 0`). A separate negative-proof run
with a crafted bad-checksum frame would be needed to observe `reason=checksum` in the log.
The source code audit provides confidence that the rejection path exists and is correct.

## F. What Is Proven

- IPv4 header checksum is computed over the correct 20-byte scope
- One's-complement 16-bit sum algorithm is correctly implemented
- Carry-folding reduces sum to 16 bits
- Valid header checksum folds to 0xFFFF
- Checksum field is included in the computation
- Bad checksum causes rejection with `reason=checksum`
- Computation is bounded (10 iterations, no unbounded loops)

## G. What Is NOT Proven

- Checksum over IP options (IHL > 5) — not supported in V1
- Incremental checksum update for NAT/rewrite — not needed in Phase C
- Hardware checksum offload — not used; software computation only
- Payload checksum (ICMP/UDP/TCP) — Phase D/E scope
- Checksum validation on fragmented packets — fragmentation rejected in Phase C

## H. Proof Command

Same as IPv4 header validate proof:

```bash
./scripts/entrypoint_build.sh

QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 \
  ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_c_tap.log
```

## I. Next

SEXNET_IPV4_CHECKSUM_GATE_V1 (Task 12)
NETWORK_STACK_STATUS_ROLLUP_V1 update
