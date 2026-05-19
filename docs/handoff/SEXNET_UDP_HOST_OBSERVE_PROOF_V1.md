# SEXNET_UDP_HOST_OBSERVE_PROOF_V1

Date: 2026-05-19
Branch: master
Depends: SEXNET_UDP_ECHO_REPLY_PROOF_V1

## Goal

Prove that the host can observe a UDP echo reply from SexOS guest at 10.0.2.15.

## Host Observe Options

| Option | Status | Notes |
|--------|--------|-------|
| A. nc/socat UDP to TAP guest | ACTIVE | nc -u available, TAP interface tap0 exists |
| B. Usernet inbound UDP | NOT SUPPORTED | QEMU SLiRP does not forward inbound UDP to guest by default |
| C. Existing host UDP probe | NEW | `scripts/host_udp_echo_observe_probe.sh` created for Phase E |
| D. root/CAP_NET_RAW | NOT REQUIRED | Normal UDP socket over TAP works without privileges |

## Probe Script

`scripts/host_udp_echo_observe_probe.sh`

Sends a bounded UDP payload to 10.0.2.15:7777 via tap0 and waits for echo reply.
Uses nc (netcat) with timeout. No infinite loops. No root required.

### Markers

| Marker | Meaning |
|--------|---------|
| `[sexnet.phaseE.host_udp.begin]` | Probe started |
| `[sexnet.phaseE.host_udp.send]` | Attempt N sent |
| `[sexnet.phaseE.host_udp.pass]` | Reply received matching payload |
| `[sexnet.phaseE.host_udp.fail]` | All attempts exhausted, no reply |
| `[sexnet.phaseE.host_udp.skip]` | Environment constraint (no TAP, no nc) |

### Parameters (environment overrides)

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST_UDP_MAX_ATTEMPTS` | 3 | Number of send attempts |
| `HOST_UDP_TARGET` | 10.0.2.15 | Target IP |
| `HOST_UDP_PORT` | 7777 | Target UDP port |
| `HOST_UDP_PAYLOAD` | HELLO_SEXNET_UDP_ECHO | Payload string |
| `HOST_UDP_TIMEOUT` | 2 | nc timeout per attempt |
| `HOST_UDP_TAP_IF` | tap0 | TAP interface name |

## PASS Conditions

1. Guest UDP echo reply log shows `sexnet.udp.echo.proof.done` with `ok=1`
2. Host probe observes reply matching sent payload
3. No faults in guest log

## SKIP Conditions

1. TAP interface not available
2. nc not installed
3. No UDP stimulus reaches sexnet NIC (usernet backend)
4. Guest-side UDP echo proof present but host observe not run

## Proof Log Path

- `/tmp/sexnet_phase_e_host_udp.log` — Host UDP probe log
