# SEXNET_PHASE_H_RUNTIME_REPROOF_V1

Date: 2026-05-19
Branch: master
Predecessor: SEXNET_PHASE_G_RUNTIME_REPROOF_V1
Depends on: Phase G proves ESTABLISHED

## Goal

Prove TCP PSH+ACK payload TX after ESTABLISHED state is reached.
Prove the payload guard correctly blocks TX when not ESTABLISHED.
Optionally prove payload RX if host listener echoes data back.

## Preconditions

1. Phase G runtime reproof PASSES with ESTABLISHED proven
2. PSH+ACK payload TX code implemented (behind existing guard)
3. Host TCP listener running on port 18080

## Phase H Implementation

The Phase H payload guard (lines 3278-3347) already checks `is_established` before
allowing payload operations. This detour adds the actual PSH+ACK payload TX code
INSIDE the existing `if is_established == 1` block. No guard weakening.

### PSH+ACK Payload TX

- Ethernet dst: gateway MAC from ARP cache (ARP_CACHE_MAC)
- IPv4: src=10.0.2.15, dst=10.0.2.2, proto=6, total_len=20+20+13
- TCP: src_port=7777, dst_port=18080, seq=local_seq+1, ack=remote_seq+1
- TCP flags: PSH|ACK (0x18), data_offset=5
- Payload: "sexnet-phase-h" (13 bytes, bounded)
- TX via desc 7, TDT=8, bounded DD poll (50M iterations max)

### Payload Content

The payload is the literal string `"sexnet-phase-h"` (13 ASCII bytes).
No HTTP, no protocol parsing, no dynamic content.
Purely a TCP data segment proof.

## Proof Commands

Same as Phase G reproof — host listener must be running:

```bash
# Terminal 1: host listener
./scripts/host_tcp_established_env_probe.sh /tmp/sexnet_phase_ghi_host_env.log 18080

# Terminal 2: proof run
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_ghi_user.log
```

## Required PASS Markers (ESTABLISHED case)

| Marker | Expected Value | Meaning |
|--------|---------------|---------|
| `[sexnet.tcp.payload.tx.guard]` | state=ESTABLISHED ok=1 | Guard allows TX |
| `[sexnet.tcp.psh_ack.build]` | payload_len=13 flags=PSH\|ACK ok=1 | PSH+ACK segment built |
| `[sexnet.tcp.psh_ack.tx.poll.done]` | dd_set=1 ok=1 | TX DD confirmed |
| `[sexnet.tcp.payload.tx.proof.done]` | sent=1 tx_dd=1 ok=1 | Payload TX complete |

## Required PASS Markers (NOT ESTABLISHED case)

| Marker | Expected Value | Meaning |
|--------|---------------|---------|
| `[sexnet.tcp.payload.tx.guard]` | state=SYN_SENT/\* ok=0 reason=not_established | Guard blocks TX |
| `[sexnet.tcp.payload.proof.done]` | established=0 ... reason=guard_blocked_not_established | Honest block |

## Optional RX Markers

| Marker | Expected Value | Meaning |
|--------|---------------|---------|
| `[sexnet.tcp.payload.rx.segment]` | ... ok=1 | Received data segment |
| `[sexnet.tcp.payload.rx.proof.done]` | received=1 bytes=N ok=1 | Payload RX confirmed |

## FIN/RST Handling

If the host listener closes after receiving payload, the guest may observe FIN or RST.
This is optional — if no close event, the markers simply don't appear.

| Marker | Meaning |
|--------|---------|
| `[sexnet.tcp.fin_rst.guard]` | state=... rst=N fin=N ok=1 | Close event observed |
| `[sexnet.tcp.rst.rx]` | RST received | Remote reset |
| `[sexnet.tcp.fin.rx]` | FIN received | Remote close |

## Honest Block Case

If Phase G does NOT prove ESTABLISHED:
- Do NOT modify the guard to force payload TX.
- The guard correctly emits `ok=0 reason=not_established`.
- Phase H runtime reproof is classified as ENV_BLOCKED, not FAILED.
- No payload TX code runs.

## Gate Impact

Gate `sexnet_tcp_payload` in daily_driver_master_gate.sh expects:
- `sexnet.tcp.payload.tx.guard.*state=ESTABLISHED.*ok=1` for PASS (established case)
- `sexnet.tcp.payload.tx.guard.*ok=0.*reason=not_established` for PASS (honest block)
- Additional markers for PSH+ACK TX DD confirmation

## Markers

- [sexnet.phaseH.runtime_reproof]
