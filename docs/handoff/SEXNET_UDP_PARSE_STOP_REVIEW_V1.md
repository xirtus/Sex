# SEXNET_UDP_PARSE_STOP_REVIEW_V1

Date: 2026-05-19
Branch: master
Commit context: Phase D ICMP echo reply DONE; ARP/IPv4/L2 path proven

## Review Questions

### 1. Where does validated IPv4 proto=17 dispatch currently stop?

ANSWER: It does not exist. The IPv4 RX validate path at line ~2080 of `servers/sexnet/src/main.rs`
dispatches only proto==1 (ICMP echo handler). After the `if proto == 1` block and its `else if`
clause, the `if ok == 1` scope closes. There is **no proto==17 branch**.

This is a safe starting point: no UDP code exists, so no existing UDP behavior to break.

### 2. Is there already UDP header parsing?

ANSWER: No. The IPv4 handler parses IPv4 fields up to offset 33 (end of IPv4 header).
Offset 34+ is only read by the ICMP handler for proto==1. No UDP fields
(src_port, dst_port, length, checksum) are parsed.

### 3. Is there already UDP checksum code or checksum policy?

ANSWER: No. The only checksum code in the IPv4 RX path is:
- IPv4 header checksum (10× 16-bit words)
- ICMP checksum validation (inside proto==1 handler)
No UDP pseudo-header checksum exists.

### 4. Is there already UDP echo reply TX code?

ANSWER: No. The TX path is used by:
- ARP request (tx desc offset 0, TDT=1)
- ARP cache reply (tx desc offset 16, TDT=2)
- L2 reuse frame (tx desc offset 32, TDT=3)
- ICMP echo reply (tx desc offset 48, TDT=4)

TX descriptor offset 64 (index 4) is available for UDP echo reply TX.

### 5. Can UDP reply reuse existing Ethernet/IPv4 TX descriptor path without driver redesign?

ANSWER: Yes. The existing permanent TX descriptor ring (8 entries, indices 0-7)
has free entries. The TX frame buffer (TX_PERM_FRAME_VA) is reused per-protocol
by overwriting. The e1000e TX path is:
1. Write frame bytes to TX_PERM_FRAME_VA
2. Write descriptor at TX_PERM_DESC_VA + (idx * 16): buffer addr, length, CMD=0x0B, STA=0
3. Write TDT=next_index to NIC register 0x3818
4. Poll descriptor STA bit 0 (DD) with bounded loop (50M iterations)

This path is proven for ARP, L2, and ICMP. Adding UDP is a direct copy-paste-adapt
exercise with no driver architecture changes.

### 6. Can host UDP observe be done in this environment without forbidden root/raw socket requirements?

ANSWER: With TAP backend: `nc`, `socat`, or `bash` with `/dev/udp` can send UDP without
root (raw socket is NOT needed for UDP over TAP -- a normal UDP socket bound to the TAP
interface's subnet works). `nc -u` or `socat - UDP-SENDTO` are standard tools.

Without TAP (usernet only): SLiRP in QEMU usernet mode does not forward inbound UDP
to the guest by default. Host UDP observe will SKIP honestly.

With CAP_NET_RAW or root: always available but not required for Phase E.

### 7. Can Phase E complete without DNS/TCP/HTTP?

ANSWER: Yes. Phase E scope is strictly:
- Receive IPv4/UDP datagram
- Validate UDP header (length, checksum policy)
- Build and transmit UDP echo reply
- Host observe if environment allows
No DNS interpretation, no TCP state machine, no HTTP parsing.

### 8. What STOP FIRST boundaries apply?

ANSWER: The following are STOP FIRST boundaries:
- **kernel/**: No edits allowed. NIC BAR mapping, PCIe, and page tables are stable.
- **crates/sex-pdx/**: No ABI or syscall changes needed.
- **global ABI/opcode definitions**: No new IPC opcodes needed.
- **scheduler/time/PKRU code**: No changes needed.
- **browser/sexdisplay/silk-shell**: Not involved.
- **DNS/TCP/HTTP**: Outside Phase E scope.
- **ARP cache redesign**: Not needed.
- **HAL NET_DIAG retirement**: Not in Phase E.
- **routing table**: Not needed.
- **fragmentation support**: Explicitly rejected in V1 (fragmented IPv4 already rejected).
- **>1 RX frame per poll**: IPv4 poll already bounded at 1 frame. Safe.

## STOP Review Conclusion

**[sexnet.phaseE.stop_review.pass]**

All conditions for safe Phase E implementation are met:
- No forbidden edits required
- TX path is proven and has a free descriptor slot
- IPv4 RX validation is proven and proto field is already parsed
- No existing UDP behavior to regress
- UDP echo reply is a narrow addition to the existing IPv4+ICMP handler pattern
- Host UDP observe can be done honestly (PASS if TAP available, SKIP otherwise)
- No DNS/TCP/HTTP needed

## Implementation Plan

1. Add `else if proto == 17 && total_len >= 28` block after the ICMP handler
2. Parse UDP header (8 bytes: src_port, dst_port, length, checksum)
3. Validate: udp_len >= 8, udp_len <= ipv4_payload_len, nonzero checksum
4. Accept checksum==0 with policy=zero_allowed
5. Build UDP echo reply: swap ports, same payload, checksum=0
6. Build IPv4 reply header (proto=17)
7. Build Ethernet reply header (swap MACs)
8. TX via descriptor index 4 (offset 64), TDT=5
9. Emit Phase E markers
