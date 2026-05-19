# SEXNET_IPV4_PARSE_STOP_REVIEW_V1

Date: 2026-05-19
Branch: master
Commit: pending (Phase C docs)
Review: STOP review for IPv4 parse path before Phase C gate/docs finalization

## Review Questions

### 1. Where does current Ethernet RX dispatch live?
`servers/sexnet/src/main.rs`, inside the sexnet server's `NIC_OWNER_SEXNET_FULL` poll loop.
ARPs are dispatched first (L2 loop, lines ~1303–1543). After ARP proof completes (or times out),
the IPv4 path runs at line 1904 under `ipv4_rx_own == NIC_OWNER_SEXNET_FULL`.

### 2. Does ethertype 0x0800 reach an IPv4 parse path?
Yes. Line 1931: `if ethertype == 0x0800`. The ethertype is read from bytes 12–13 of the
Ethernet frame (standard type/length field). Non-0x0800 frames log a reject marker and are skipped.

### 3. Is there already IPv4 header validation code?
Yes. Lines 1956–2049 parse and validate:
- version (must be 4)
- IHL (must be 5; no options support in V1)
- total_len (must be >= 20, must be <= pkt_len - 14)
- fragmentation (DF or no-frag only; frag_masked != 0 → reject)
- destination IP (must be 10.0.2.15)
- header checksum (one's-complement 16-bit sum over 10 words = 20 bytes, must fold to 0xFFFF)

### 4. Is there already IPv4 checksum code?
Yes. Lines 2027–2048 compute the one's-complement 16-bit sum over IHL*4 = 20 bytes
(10 words, hardcoded `while ci < 10`). The sum is folded via carry-add until it fits in
16 bits, then compared against 0xFFFF. Valid checksum → `checksum_ok=1`.

### 5. Is the proof path real RX, synthetic frame, or mixed?
Real RX only. The proof loop polls the e1000e RX ring for hardware-delivered frames.
On TAP backend, a host `ping -I tap0 10.0.2.15` delivers a real ICMP echo request frame
through the tap0 interface into the QEMU guest NIC. No synthetic frame injection.

### 6. Can Phase C complete without kernel/ABI/sex-pdx edits?
Yes. The IPv4 parse/validate/checksum code lives entirely in `servers/sexnet/src/main.rs`.
It uses only:
- Existing e1000e NIC BAR mapping
- `RX_PERM_DESC_VA` / `RX_PERM_PKT_VA` (set up by sexnet's NIC init)
- `core::ptr::read_volatile` / `write_volatile`
- `serial_println!` for markers
No kernel syscall, ABI opcode, or sex-pdx change is needed.

### 7. Can Phase C complete without ICMP/UDP/TCP?
Yes. Phase C only validates the IPv4 header and checksum. The proto field is parsed
and logged but never dispatched. No ICMP echo reply, no UDP datagram processing,
no TCP segment processing is implemented or required.

### 8. What STOP FIRST boundaries apply?
- Do not edit kernel/
- Do not edit crates/sex-pdx/
- Do not edit global ABI/opcode definitions
- Do not implement ICMP echo reply
- Do not implement UDP/DNS/TCP/HTTP
- Do not implement routing
- Do not implement fragmentation reassembly
- Do not add IP options support beyond IHL=5
- Do not retire HAL NET_DIAG
- Do not change scheduler/PKRU/time
- Do not change browser/sexdisplay/shell/Silk

None of these boundaries are threatened by Phase C. All IPv4 parse/validate/checksum
code already exists within the allowed file set.

## STOP Review Conclusion

**PASS REVIEW** — IPv4 parse path is already implemented, bounded, and safe.
Phase C requires only documentation (STOP review, gate handoffs, checksum proof doc,
rollup update). No runtime code changes are needed. No forbidden boundaries are crossed.

### Marker
[sexnet.phaseC.stop_review.pass]

## What Already Exists

| Component | Status | Location |
|-----------|--------|----------|
| IPv4 ethertype dispatch | IMPLEMENTED | `servers/sexnet/src/main.rs:1931` |
| IPv4 header field parse | IMPLEMENTED | `servers/sexnet/src/main.rs:1956–2013` |
| IPv4 header validation (ver/ihl/len/dst) | IMPLEMENTED | `servers/sexnet/src/main.rs:2014–2025` |
| IPv4 header checksum validation | IMPLEMENTED | `servers/sexnet/src/main.rs:2027–2048` |
| Positive validation markers | IMPLEMENTED | `servers/sexnet/src/main.rs:2070,2113–2122` |
| Negative rejection markers | IMPLEMENTED | `servers/sexnet/src/main.rs:2082–2096` |
| Gate `sexnet_ipv4_header_validate` | IMPLEMENTED | `scripts/daily_driver_master_gate.sh:2245–2278` |
| Proof doc | IMPLEMENTED | `docs/handoff/SEXNET_IPV4_HEADER_VALIDATE_PROOF_V1.md` |

## What Phase C Docs Remain

| Task | Doc |
|------|-----|
| 08 | STOP review (this doc) |
| 09 | Proof already exists (SEXNET_IPV4_HEADER_VALIDATE_PROOF_V1.md) |
| 10 | Header validate gate handoff (SEXNET_IPV4_HEADER_VALIDATE_GATE_V1.md) |
| 11 | Checksum proof doc (SEXNET_IPV4_CHECKSUM_PROOF_V1.md) |
| 12 | Checksum gate handoff (SEXNET_IPV4_CHECKSUM_GATE_V1.md) |
| — | Rollup update (NETWORK_STACK_STATUS_ROLLUP_V1.md) |

## Proof Commands

```bash
./scripts/entrypoint_build.sh

# TAP backend
QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 \
  ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_c_tap.log

# User backend (may SKIP IPv4 gate if no IPv4 stimulus)
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_c_user.log
```

## Next

SEXNET_IPV4_HEADER_VALIDATE_PROOF_V1.md (already exists — Task 09 satisfied)
SEXNET_IPV4_HEADER_VALIDATE_GATE_V1.md (Task 10)
