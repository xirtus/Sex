# SEXNET_ICMP_HOST_PING_OBSERVE_PROOF_V1

Date: 2026-05-19
Branch: master
Commit: pending (Phase D host ping observe)
Gate: `sexnet_icmp_host_ping_observe` (new)
Depends: ICMP echo reply proof (guest-side)

## Overview

Proves that when the host sends a ping to 10.0.2.15 and sexnet responds with
an ICMP echo reply, the host ping command observes the reply. This closes the
loop: host → guest → host.

## Host Observe Options

### A. Direct ping (requires root/CAP_NET_RAW)

```bash
sudo ping -I tap0 -c 1 -W 1 10.0.2.15
```

Expected output includes: `1 packets transmitted, 1 received, 0% packet loss`

### B. Host ping observe probe script

A tiny bounded script `scripts/host_icmp_ping_observe_probe.sh` is provided.

### C. If ping cannot run

SKIP honestly with environment reason. Guest-side ICMP echo reply proof
(`sexnet_icmp_echo_reply`) still stands independently.

## Probe Script

`scripts/host_icmp_ping_observe_probe.sh`

Usage:
```bash
./scripts/host_icmp_ping_observe_probe.sh /tmp/sexnet_phase_d_host_ping.log
```

Parameters:
- MAX_ATTEMPTS=3 (bounded)
- TARGET_IP=10.0.2.15
- TAP_IF=tap0 (default, overridable via HOST_PING_TAP_IF)

Behavior:
1. Checks if tap0 exists
2. Runs ping -I tap0 -c 1 -W 2 10.0.2.15
3. Parses output for received count
4. Logs PASS/FAIL/SKIP markers
5. No infinite loops, no password prompts

### Markers Emitted

- `[sexnet.phaseD.host_ping.begin]`
- `[sexnet.phaseD.host_ping.ping] attempt=N`
- `[sexnet.phaseD.host_ping.pass]` — ping reply observed
- `[sexnet.phaseD.host_ping.fail]` — ping sent but no reply
- `[sexnet.phaseD.host_ping.skip] reason=...`

### PASS Conditions

- ping command observes reply from 10.0.2.15
- SexOS log shows ICMP RX echo + TX reply markers
- No faults

### FAIL Conditions

- ping ran but did not see reply
- SexOS emitted ICMP reply proof but host observe contradicts it

### SKIP Conditions

- ping requires root/CAP_NET_RAW and unavailable
- TAP interface not found
- usernet backend (cannot route ping to guest NIC)

## Proof Commands

```bash
# In one terminal: run SexOS with TAP
QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 \
  ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_d_tap.log

# In another terminal: run host ping probe
./scripts/host_icmp_ping_observe_probe.sh /tmp/sexnet_phase_d_host_ping.log
```

## Log Paths

- `/tmp/sexnet_phase_d_host_ping.log` — host ping output
- `/tmp/sexnet_phase_d_tap.log` — SexOS guest log

## Next

SEXNET_ICMP_HOST_PING_GATE_V1 (Task 17)
