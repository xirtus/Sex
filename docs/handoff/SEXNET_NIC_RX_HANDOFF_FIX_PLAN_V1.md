# SEXNET_NIC_RX_HANDOFF_FIX_PLAN_V1

Date: 2026-05-19
Branch: master
Predecessor: SEXNET_NIC_RX_HANDOFF_STOP_REVIEW_V1

## Chosen Fix Path: Sexnet Port Deconfliction (PORT_DECONFLICT_V1)

### Rationale

The root cause is NOT a NIC RX ring handoff bug. The sexnet RX ring is correctly
programmed and functional. The blocker is a QEMU SLIRP limitation: only ONE
outbound TCP connection from the guest to a given (host, port) destination.

The HAL diagnostic (kernel/src/hal/pci.rs, source=2) and sexnet server
(servers/sexnet/src/main.rs, source=3) both target `10.0.2.2:18080`.
HAL diagnostic claims the single SLIRP slot. Sexnet's connection is dropped.

### Fix: Different Destination Ports

Change sexnet's `TCP_REMOTE_PORT` from `18080` to `18081`. This gives sexnet
source=3 its own SLIRP connection slot. HAL diagnostic continues using 18080.

### Allowed Files

- `servers/sexnet/src/main.rs` — change `TCP_REMOTE_PORT: u16 = 18081` (line 199)
- `docs/handoff/SEXNET_NIC_RX_HANDOFF_STOP_REVIEW_V1.md` — this review (already created)
- `docs/handoff/SEXNET_NIC_RX_HANDOFF_FIX_PLAN_V1.md` — this plan (already created)
- `docs/handoff/NETWORK_STACK_STATUS_ROLLUP_V1.md` — update with fix result

### Forbidden Files

- `kernel/src/hal/pci.rs` — NO edits (STOP FIRST)
- Any kernel code
- ABI/sex-pdx definitions
- Browser/server code beyond sexnet
- Scheduler/time/PKRU code

### Exact Source Change

```rust
// Line 199, servers/sexnet/src/main.rs
// BEFORE:
static mut TCP_REMOTE_PORT: u16 = 18080;
// AFTER:
static mut TCP_REMOTE_PORT: u16 = 18081;
```

### Host Configuration

```bash
# Start listeners on BOTH ports (HAL diag uses 18080, sexnet uses 18081)
socat TCP-LISTEN:18080,reuseaddr,fork - &
socat TCP-LISTEN:18081,reuseaddr,fork - &
```

Or single listener for sexnet only:
```bash
socat TCP-LISTEN:18081,reuseaddr,fork - &
```

### Required Proof Markers After Fix

Phase G (source=3):
- `[sexnet.tcp.synack.rx] src_port=... dst_port=7777 flags=SYN|ACK ok=1`
- `[sexnet.tcp.synack.validate] ack_ok=1 ports_ok=1 checksum_ok=1 ok=1`
- `[sexnet.tcp.handshake.state] state=ESTABLISHED ok=1`
- `[sexnet.tcp.ack.tx.proof.done] ack_sent=1 tx_dd=1 ok=1`

Phase H (source=3):
- `[sexnet.tcp.payload.tx.guard] state=ESTABLISHED ok=1`
- `[sexnet.tcp.psh_ack.build] payload_len=13 flags=PSH|ACK ok=1`
- `[sexnet.tcp.psh_ack.tx.poll.done] dd_set=1 ok=1`
- `[sexnet.tcp.payload.tx.proof.done] sent=1 tx_dd=1 ok=1`

Phase I readiness:
- `sexnet_http_phase_i_readiness` gate → PASS

### Rollback Plan

Revert `TCP_REMOTE_PORT` from 18081 back to 18080:
```bash
git checkout servers/sexnet/src/main.rs
```

### Diagnostic Markers (Optional, for future diagnostics)

- `[sexnet.nic.rx_handoff.audit.begin]`
- `[sexnet.nic.rx_handoff.regs.before]` rdbal=X rdlen=Y rdt=Z
- `[sexnet.nic.rx_handoff.regs.after]` rdbal=X rdlen=Y rdt=Z
- `[sexnet.nic.rx_handoff.proof.done] ok=1`

### Proof Commands After Fix

```bash
./scripts/entrypoint_build.sh

# Start host listener on port 18081
socat TCP-LISTEN:18081,reuseaddr,fork - &
LISTENER_PID=$!

# Run proof
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_port_deconflict_user.log

# Kill listener
kill $LISTENER_PID 2>/dev/null || true

# Verify markers
grep -E "sexnet\.tcp\.(synack|handshake\.state|payload\.tx\.guard|psh_ack|payload\.tx\.proof)" \
  /tmp/sexnet_port_deconflict_user.log
```

### Expected Result

- Phase G: ESTABLISHED proven (SYN → SYN-ACK → final ACK)
- Phase H: PSH+ACK payload TX proven (after ESTABLISHED)
- Phase I: readiness gate PASS
- Faults: 0
- Daily gate: PASS

### Markers

- [sexnet.nic.rx_handoff.fix_plan]
