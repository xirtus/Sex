# TAP_HOST_ENV_FIX_PLAN_V1

Date: 2026-05-17
Status: Plan only (no runtime change in this lane)

## Objective
Enable a backend where guest raw TCP receives reply traffic (SYN-ACK/RST), replacing SLiRP-only limitation.

## Current blocker truth
- SLiRP freeze marker: `environment_limited=1`
- Current host: tap startup fails (`/dev/net/tun` unavailable in this environment)

## Plan
1. Host prerequisites
- Ensure `/dev/net/tun` exists and `tun` module is loaded.
- Create TAP interface (for example `tap0`) and grant QEMU access.
- Provide outbound routing/NAT from TAP bridge/network.

2. Runner configuration
- Use existing runner knobs:
  - `QEMU_NET_BACKEND=tap`
  - `QEMU_TAP_IFNAME=tap0`
  - `QEMU_NET_MODEL=e1000e`

3. Verification sequence
- Boot proof lane with tap backend.
- Require markers:
  - `[qemu.net.config] ... backend=tap ...`
  - `[tcp.syn.send.retry.proof] sent=1 tx_dd=1`
- Success criteria:
  - observe `synack_seen=1` or `rst_seen=1` for bounded TCP probe.

4. If tap boot succeeds but still no TCP reply
- Capture-backed mission: `TCP_WITH_CAPTURE_BACKEND_PROOF_V1` to compare TX/RX at host edge.

## Non-goals
- No final ACK mission in this step.
- No HTTP GET mission in this step.
