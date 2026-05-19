# SEXNET_PHASE_G_RUNTIME_REPROOF_V1

Date: 2026-05-19
Branch: master
Predecessor: SEXNET_TCP_ESTABLISHED_ENV_PROOF_V1
Depends on: SEXNET_TCP_ESTABLISHED_ENV_PROOF_V1 (environment), host_tcp_established_env_probe.sh (listener)

## Goal

Rerun Phase G TCP handshake proof with a real host TCP listener so the guest
can receive a genuine SYN-ACK and complete the 3-way handshake.

## Preconditions

1. Host TCP listener running on port 18080 (via host_tcp_established_env_probe.sh)
2. TCP_REMOTE_PORT changed from 80 to 18080 in servers/sexnet/src/main.rs
3. QEMU SLIRP user-mode networking (QEMU_NET_BACKEND=user, QEMU_NET_MODEL=e1000e)

## Proof Commands

```bash
# Terminal 1: start host TCP listener (keep running)
./scripts/host_tcp_established_env_probe.sh /tmp/sexnet_phase_ghi_host_env.log 18080
# Take note of LISTENER_PID printed by the script

# Terminal 2: run proof
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_ghi_user.log

# Cleanup listener
kill $LISTENER_PID 2>/dev/null || true
```

## Required PASS Markers

| Marker | Expected Value | Meaning |
|--------|---------------|---------|
| `[sexnet.tcp.syn.build]` | ok=1 | TCP SYN built correctly |
| `[sexnet.tcp.syn.tx.proof.done]` | tx=1 tx_dd=1 ok=1 | SYN TX DD confirmed |
| `[sexnet.tcp.synack.rx]` | flags=SYN\|ACK ok=1 | Received SYN-ACK from host |
| `[sexnet.tcp.synack.validate]` | ack_ok=1 ports_ok=1 ok=1 | SYN-ACK validation passed |
| `[sexnet.tcp.ack.tx.proof.done]` | ack_sent=1 tx_dd=1 ok=1 | Final ACK sent and DD confirmed |
| `[sexnet.tcp.handshake.state]` | state=ESTABLISHED ok=1 | State transitioned to ESTABLISHED |

## Honest Non-Pass Outcomes

| Outcome | Marker | Meaning |
|---------|--------|---------|
| RST observed | `state=FAILED_RST rst=1 ok=0 honest=1` | Remote refused connection |
| Timeout | `rx_synack=0 rst=0 timeout=0` (no SYN-ACK observed) | No listener or routing issue |
| No listener | Gate SKIP | Environment cannot produce SYN-ACK |

## Proof Flow

1. Guest builds TCP SYN: src=10.0.2.15:7777, dst=10.0.2.2:18080, seq=42, flags=SYN
2. Guest transmits SYN via e1000e TX desc 5, TDT=6
3. QEMU SLIRP forwards SYN to host 127.0.0.1:18080
4. Host kernel TCP stack (nc listener) sends SYN-ACK
5. Guest receives SYN-ACK in IPv4 RX loop
6. Guest validates SYN-ACK: checksum, ports, ACK=43
7. Guest transitions to ESTABLISHED, stores remote_seq
8. Guest sends final ACK via e1000e TX desc 6, TDT=7
9. Host receives final ACK, connection ESTABLISHED on both sides

## Source Changes Required

- `TCP_REMOTE_PORT`: 80 → 18080 (tiny proof-target edit)
- No other source changes for Phase G

## Gate Impact

Gate `sexnet_tcp_handshake` in daily_driver_master_gate.sh expects:
- `sexnet.tcp.synack.rx.proof.done.*rx_synack=1.*ok=1` — will PASS if host listener responds
- `sexnet.tcp.ack.tx.proof.done.*ack_sent=1.*tx_dd=1.*ok=1` — will PASS if final ACK sent
- `sexnet.tcp.handshake.state.*state=ESTABLISHED` — will PASS if handshake completes

## Markers

- [sexnet.phaseG.runtime_reproof]
