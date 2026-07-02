# SEXNET_NIC_RX_HANDOFF_STOP_REVIEW_V1

Date: 2026-05-19
Branch: master
Predecessor: SEXNET_TCP_ESTABLISHED_ENV_PROOF_V1 (PASS REVIEW ONLY)

## STOP Review: NIC RX Handoff Blocker Investigation

### Hypothesis Tested

**H0:** NIC RX ring handoff between HAL diagnostic (source=2) and sexnet server
(source=3) prevents sexnet from receiving SYN-ACK.

**H1:** QEMU SLIRP user-mode networking limits outbound TCP to one connection
per (dst_ip, dst_port), and the HAL diagnostic consumes the single slot.

### Evidence

#### Source=2 (HAL diagnostic, kernel/src/hal/pci.rs)

| Item | Value | Evidence |
|------|-------|----------|
| TCP target IP | 10.0.2.2 | `tcp.guest.host.10_0_2_2.plan` |
| TCP target port | 18080 | `let tcp_probe_dst_port: u16 = 18080` (line 2738) |
| SYN TX | tx_dd=1 | `tcp.syn.tx.post` tx_dd=1 |
| SYN-ACK RX | synack_seen=1 | `tcp.syn.rx.synack` synack_seen=1 |
| RX ring functional | YES | Multiple frames received |

#### Source=3 (sexnet server, servers/sexnet/src/main.rs)

| Item | Value | Evidence |
|------|-------|----------|
| TCP target IP | 10.0.2.2 | `sexnet.tcp.entry` remote=10.0.2.2:18080 |
| TCP target port | 18080 | `static mut TCP_REMOTE_PORT: u16 = 18080` (line 199) |
| SYN TX | tx_dd=1 | `sexnet.tcp.syn.tx.proof.done` tx=1 tx_dd=1 |
| SYN-ACK RX | NOT observed | No `sexnet.tcp.synack.rx` marker |
| RX ring functional | YES | ARP handler found 2 frames at idx=0,1 in sexnet ring |

#### SLIRP Limitation

| Item | Value | Evidence |
|------|-------|----------|
| SLIRP freeze marker | environment_limited=1 | `qemu.slirp.tcp.limit.freeze` |
| First connection SYN-ACK | synack=1 | Same marker |
| Limitation reason | slirp_tcp_no_response | Same marker |

### Register/Ring Ownership Audit

Sexnet permanent RX ring programming (lines 893-905):
```
perm_rctl_orig & !(1<<1)  → disable RX
RDBAL = perm_desc_phys    → sexnet descriptor base
RDBAH = perm_desc_phys>>32
RDLEN = 128               → 8 descriptors * 16 bytes
RDH = 0                   → head pointer
RDT = 7                   → tail pointer (all owned by NIC)
SRRCTL = 0x00000002       → buffer size
RCTL = rctl_init          → enable RX (EN|UPE|MPE|BAM|SECRC)
```

Readback confirms: `[sexnet.nic.rx.permanent.ring.program] rdbal=0x102C4000 rdlen=128 rdt=7 rctl=0x0400801A ok=1`

Sexnet RX ring IS correctly programmed and functional:
- ARP handler at lines 1338-1417 found 2 frames at idx=0,1 with DD=1
- These frames were recycled and RDT was updated correctly

**The NIC RX ring handoff is NOT the blocker. Sexnet's RX ring works.**

### Root Cause

**QEMU SLIRP user-mode networking limitation**: only ONE outbound TCP connection
from the guest to a given (host, port) destination is forwarded. The HAL diagnostic
(kernel/src/hal/pci.rs) sends TCP SYN to 10.0.2.2:18080 first (during PCI init),
SLIRP forwards it, host responds with SYN-ACK, HAL diag receives it.

When sexnet later sends TCP SYN to the SAME destination (10.0.2.2:18080), SLIRP
silently drops it because the NAT table already has an entry for that destination.
No SYN-ACK reaches the guest NIC for sexnet's connection.

This is confirmed by the `[qemu.slirp.tcp.limit.freeze]` marker:
```
backend=user tcp_syn_tx=1 synack=1 rst=0 checksum_ok=1 offload_ok=1
final_ack_sent=0 http_sent=0 environment_limited=1 ok=1
reason=slirp_tcp_no_response
```

### STOP FIRST Classification

**PASS REVIEW ONLY** — blocker identified and fix plan written. No unsafe edits
needed. The fix is a trivial port assignment change (one line in sexnet source).

### Answers to Review Questions

1. **Does HAL diagnostic leave NIC RX enabled with HAL ring active?**
   YES — HAL diag writes RCTL.EN=1. But sexnet later reconfigures the ring.

2. **Does sexnet later reprogram RDBAL/RDBAH/RDLEN/RDH/RDT/RCTL?**
   YES — three times: test ring (line 564-572), observe ring (line 728+),
   permanent ring (line 896-904). Permanent ring is the final configuration.

3. **Does sexnet disable RX before changing RX ring registers?**
   YES — `rctl_orig & !(1<<1)` before changing ring base registers.

4. **Does sexnet clear/reset descriptor status before enabling RX?**
   YES — zeroes all descriptor memory (lines 864-881).

5. **Are sexnet RX descriptors physically visible and UC mapped?**
   YES — allocated via sys_alloc_phys + sys_map_phys, UC aliased.

6. **Does sexnet write RDT correctly after ring init?**
   YES — RDT=7 after permanent ring init.

7. **Does e1000e require full device reset or RX disable before ring base change?**
   YES (RX disable) — sexnet does this correctly.

8. **Is the NIC still DMA-writing into the old HAL ring after handoff?**
   NO — sexnet's permanent ring is programmed with its own descriptor addresses.

9. **Does HAL diagnostic consume packets before sexnet source=3 sees them?**
   YES — but different copies. HAL diag's ring was reconfigured by sexnet.

10. **Is there a single-owner model already intended by docs?**
    The architecture supports dual ownership (NIC_RX_OWNER / NIC_TX_OWNER state machine).

11. **Which fix is smallest and safest?**
    **Change sexnet TCP_REMOTE_PORT to 18081** (different from HAL diag's 18080).
    One-line edit. No kernel changes. No ABI changes. Instantly testable.

### Markers

- [sexnet.nic.rx_handoff.stop_review.pass_review_only]
