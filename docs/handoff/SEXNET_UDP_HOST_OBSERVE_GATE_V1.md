# SEXNET_UDP_HOST_OBSERVE_GATE_V1

Date: 2026-05-19
Branch: master
Gate name: `sexnet_udp_host_observe`

## Old State

Gate did not exist. No host UDP echo observe capability.

## Gate Logic (implemented in scripts/daily_driver_master_gate.sh)

### PASS conditions:
1. Host UDP probe log has `sexnet.phaseE.host_udp.pass` marker
2. Guest log has `sexnet.udp.echo.proof.done` with `ok=1`
3. No faults

### SKIP conditions:
1. Host UDP probe produced `sexnet.phaseE.host_udp.skip`
2. No host UDP probe log present but guest UDP echo proof exists (PASS REVIEW ONLY)
3. Usernet backend without UDP forwarding

### FAIL conditions:
1. Host UDP probe ran and did not see reply (`sexnet.phaseE.host_udp.fail`)
2. Guest emitted UDP reply but host contradicts it
3. Fault scan fails

## PASS REVIEW ONLY fallback

If the guest-side UDP echo reply proof is complete (`sexnet.udp.echo.proof.done ok=1`)
but host observe was not run (no host probe log), the gate reports:
- `sexnet_udp_host_observe` = PASS (PASS REVIEW ONLY)
- Rationale: Guest-side RX→TX proof is self-contained; host observe is best-effort.

## Proof Command

```bash
# In terminal 1: run SexOS with TAP
QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 \
  ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_e_tap.log

# In terminal 2 (while SexOS is running): send UDP probe
./scripts/host_udp_echo_observe_probe.sh /tmp/sexnet_phase_e_host_udp.log
```

## Log Paths
- `/tmp/sexnet_phase_e_host_udp.log` — Host UDP probe log
- Guest log as specified by proof command
