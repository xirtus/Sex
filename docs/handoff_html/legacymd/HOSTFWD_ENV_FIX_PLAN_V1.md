# HOSTFWD_ENV_FIX_PLAN_V1

Date: 2026-05-17
Status: Plan only (no runtime change in this lane)

## Objective
Repair hostfwd path so user-mode backend can be exercised against a deterministic host listener.

## Current blocker truth
- Current hostfwd attempt failed before guest runtime setup.
- SLiRP lane runs only without hostfwd (`hostfwd=none`).

## Plan
1. Hostfwd rule validation
- Validate rule format accepted by local QEMU build.
- Confirm selected host port is free.
- Bind local listener on host (plain HTTP test service).

2. Runner configuration
- Use existing runner knobs:
  - `QEMU_NET_BACKEND=user`
  - `QEMU_USERNET_HOSTFWD=tcp::<hostport>-:<guestport>`
  - `QEMU_NET_MODEL=e1000e`

3. Verification sequence
- Boot proof lane with hostfwd enabled.
- Require marker:
  - `[qemu.net.config] ... backend=user ... hostfwd=tcp::<hostport>-:<guestport>`
- Probe guest->`10.0.2.2:<hostport>` with bounded SYN retries.
- Success criteria:
  - `synack_seen=1` or `rst_seen=1`.

4. Escalation
- If hostfwd still fails pre-boot, freeze with exact QEMU stderr and pivot to tap/capture backend plan.

## Non-goals
- No final ACK mission in this step.
- No HTTP GET mission in this step.
