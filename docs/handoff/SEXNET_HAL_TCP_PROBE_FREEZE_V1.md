# SEXNET_HAL_TCP_PROBE_FREEZE_V1

Date: 2026-05-19
Branch: master
Predecessor: SEXNET_NIC_RX_HANDOFF_STOP_REVIEW_V1

## Goal

Freeze the HAL diagnostic TCP probe (kernel/src/hal/pci.rs) at compile time
so sexnet source=3 gets the sole SLIRP TCP connection attempt, then rerun
the GHI proof to reach ESTABLISHED.

## Implementation

### Compile-Time Gate

Added `option_env!("SEXOS_HAL_TCP_PROBE")` gate to kernel/src/hal/pci.rs:

- **Default** (unset or any value except "0"): HAL TCP probe runs normally.
  Marker: `[hal.tcp.probe.gate] enabled=1 ok=1`
- **SEXOS_HAL_TCP_PROBE=0**: HAL TCP probe is skipped entirely.
  Marker: `[hal.tcp.probe.gate] enabled=0 reason=SEXOS_HAL_TCP_PROBE=0 ok=1`

### Gated Code

The entire TCP probe block (SYN build, TX post, RX poll for SYN-ACK, HTTP GET attempt)
is wrapped in `if hal_tcp_probe_enabled { ... }`. When disabled:
- No TCP SYN is built or transmitted by HAL diagnostic
- No SYN-ACK poll occurs
- No HTTP GET attempt
- `tcp_built`, `checksum_ok`, `ipv4_csum_built`, `tcp_csum_built`, `tcp_ok` default to 0
- DNS-targeted TCP SYN markers (post-probe) still appear with built=0
- ARP, IPv4, ICMP, UDP, DNS diagnostics remain fully active

### Variables Changed

- `let tcp_built: u8;` → `let mut tcp_built: u8 = 0;`
- `let ipv4_csum_built: u16;` → `let mut ipv4_csum_built: u16 = 0;`
- `let tcp_csum_built: u16;` → `let mut tcp_csum_built: u16 = 0;`
- `let checksum_ok: u8;` → `let mut checksum_ok: u8 = 0;`
- `let tcp_ok: u8;` → `let mut tcp_ok: u8 = 0;`
- `let mut resolved_dst_ip: [u8; 4]` → `let resolved_dst_ip: [u8; 4]` (immutable, always [10,0,2,2])

## Proof Result

### With SEXOS_HAL_TCP_PROBE=0

```
[hal.tcp.probe.gate] enabled=0 reason=SEXOS_HAL_TCP_PROBE=0 ok=1
```

Sexnet source=3 results:
```
[sexnet.tcp.syn.tx.proof.done] tx=1 tx_dd=1 ok=1
[sexnet.tcp.handshake.state] state=SYN_SENT ok=1
[sexnet.tcp.synack.rx] NOT PRESENT
[sexnet.tcp.handshake.state] state=ESTABLISHED NOT REACHED
```

Sexnet RX ring: 0 real packets received (no stale frames without HAL probe traffic)
- Test ring dd_poll: `dd_set=0`
- Observe ring: `dd_set=0`
- Permanent ring: `dd_set=0`
- ARP handler: `rx_arp=0` (no frames at all)
- L2 poll: `frames_rx=0`
- IPv4 poll: only self-test at idx=7

### Gate Results (250 PASS, 0 FAIL, 45 SKIP)

- `sexnet_tcp_handshake`: PASS (SYN TX proven, no SYN-ACK, env-limited honest)
- `sexnet_tcp_payload`: PASS (guard proven, honest block)
- `sexnet_http_phase_i_readiness`: SKIP (no ESTABLISHED)
- `faults_zero`: PASS (0 faults)

## Root Cause Analysis Update

The SLIRP TCP connection limitation hypothesis is **insufficient** to explain
the failure. Even with the HAL TCP probe disabled (sexnet is the ONLY TCP user),
sexnet's RX ring receives zero packets. The NIC stops delivering packets to
sexnet's RX descriptors after the NIC handoff from kernel HAL to sexnet server.

Evidence:
1. With HAL probe enabled: sexnet ARP handler found 2 stale IPv4 frames (from HAL's traffic)
2. With HAL probe disabled: sexnet ARP handler found 0 frames (no HAL traffic to create stale frames)
3. In both cases: no new frames (SYN-ACK) are ever received by sexnet
4. HAL diagnostic's own RX ring works perfectly (receives SYN-ACK when enabled)
5. Sexnet's TX works (SYN DD=1 confirmed)

The NIC receive path is broken after sexnet reconfiguration. The e1000e likely
requires a more complete initialization sequence beyond just ring register
programming (possible requirements: device reset via CTRL.RST, full MAC/PHY
re-init, interrupt mask reconfiguration).

## Conclusion

**STOP FIRST** — the blocker is deeper than expected. The sexnet NIC initialization
needs a full device reset or the NIC ownership model needs restructuring.
Neither can be done safely without a dedicated NIC reset/review mission.

## Rollback

Remove the env var or set `SEXOS_HAL_TCP_PROBE=1` to restore HAL TCP probe:
```bash
# Unset (default behavior):
./scripts/entrypoint_build.sh
./scripts/run_daily_driver_proof.sh /tmp/sexos_proof.log

# Or explicit enable:
SEXOS_HAL_TCP_PROBE=1 ./scripts/entrypoint_build.sh
SEXOS_HAL_TCP_PROBE=1 ./scripts/run_daily_driver_proof.sh /tmp/sexos_proof.log
```

## Files Changed

- `kernel/src/hal/pci.rs` — added option_env! gate, variable initialization (+25/-16)
- `servers/sexnet/src/main.rs` — TCP_REMOTE_PORT 18081 (port deconfliction, 1 line)

## Markers

- [sexnet.hal_tcp_probe.freeze.implemented]
