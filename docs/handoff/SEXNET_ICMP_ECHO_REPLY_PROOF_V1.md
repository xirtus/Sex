# SEXNET_ICMP_ECHO_REPLY_PROOF_V1

Date: 2026-05-19
Branch: master
Commit: pending (Phase D ICMP echo reply)
Gate: `sexnet_icmp_echo_reply` (new)
Depends: Phase C IPv4 validate + checksum (proven)

## Overview

Proves that sexnet receives a valid ICMP echo request (type=8, code=0),
validates the ICMP checksum, builds a correct ICMP echo reply (type=0, code=0),
builds a valid IPv4 reply header with correct checksum, builds an Ethernet
reply frame, and transmits it successfully via the e1000e TX descriptor ring,
observing the DD (Descriptor Done) bit.

## Proof Contract

### Positive Path

When host sends `ping 10.0.2.15`:
1. Ethernet frame with ethertype=0x0800 arrives at NIC RX ring
2. IPv4 header validated (Phase C), proto=1 detected
3. ICMP header parsed: type=8, code=0
4. ICMP identifier and sequence extracted
5. ICMP checksum validated over header+payload
6. ICMP echo reply built in TX frame buffer:
   - Ethernet: dst=request src MAC, src=NIC MAC, ethertype=0x0800
   - IPv4: ver=4, ihl=5, src=10.0.2.15, dst=request src, proto=1, ttl=64
   - IPv4 header checksum computed
   - ICMP: type=0, code=0, id/seq preserved from request
   - ICMP checksum computed
   - Payload copied verbatim
7. TX descriptor 3 set up, TDT=4 posted
8. DD bit polled until set (bounded 50M iterations)

### Required Positive Markers

- `[sexnet.icmp.rx.echo] type=8 code=0 len=N id=I seq=S ok=1`
- `[sexnet.icmp.checksum.validate] ok=1`
- `[sexnet.icmp.tx.reply.build] type=0 code=0 len=N id=I seq=S ok=1`
- `[sexnet.icmp.tx.reply.checksum] ok=1`
- `[sexnet.ipv4.tx.icmp_reply.build] src=10.0.2.15 dst=A.B.C.D total_len=N checksum=ok ok=1`
- `[sexnet.eth.tx.icmp_reply.desc] len=N ok=1`
- `[sexnet.icmp.tx.poll.done] dd_set=1 ok=1`
- `[sexnet.icmp.echo.proof.done] rx_echo=1 tx_reply=1 tx_dd=1 ok=1`

### Negative Path

Invalid ICMP frames must be rejected:

- ICMP type != 8 or code != 0:
  `[sexnet.icmp.reject] reason=not_echo_request type=N code=N ok=1`
- IPv4 total_len < 28 (IPv4 header + ICMP header minimum):
  `[sexnet.icmp.reject] reason=too_short_for_icmp ok=1`

Fragmented IPv4 is already rejected in Phase C (`reason=fragmented`).

## Implementation

All ICMP runtime code is in `servers/sexnet/src/main.rs`, inserted in the
IPv4 RX poll path after a validated IPv4 frame (`ipv4_ok=1`).

Key implementation details:
- ICMP base offset: `pkt_buf + 34` (14 eth + 20 ipv4)
- ICMP header: type(1) + code(1) + checksum(2) + id(2) + seq(2) = 8 bytes
- TX descriptor: index 3 (offset 48 from TX_PERM_DESC_VA)
- TDT post: value 4
- TX frame buffer: shared `TX_PERM_FRAME_VA`
- Minimum frame: padded to 60 bytes (Ethernet minimum)
- Bounded DD poll: 50M iterations (same as ARP/L2 TX)
- NIC MAC: re-derived from RAL/RAH registers in-scope
- Source MAC: read from RX ethernet header bytes 6-11

## No Forbidden Edits

- No kernel edits
- No sex-pdx/ABI changes
- No driver redesign
- No UDP/TCP/DNS/HTTP
- No routing table
- No scheduler/PKRU/time changes

## Proof Commands

```bash
./scripts/entrypoint_build.sh

# TAP backend (host must send ping stimulus)
# In another terminal:
#   sudo ping -I tap0 -c 1 -W 1 10.0.2.15
QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 \
  ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_d_tap.log

# User backend (ICMP proof SKIPs if no ping reaches NIC)
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_d_user.log
```

## Log Paths

- `/tmp/sexnet_phase_d_tap.log` — TAP backend proof
- `/tmp/sexnet_phase_d_user.log` — user backend proof

## Next

SEXNET_ICMP_ECHO_REPLY_GATE_V1 (Task 15)
SEXNET_ICMP_HOST_PING_OBSERVE_PROOF_V1 (Task 16)
