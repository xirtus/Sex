# SEXNET_TCP_ESTABLISHED_ENV_PROOF_V1

Date: 2026-05-19
Branch: master
Mission: SEXNET_PHASE_GHI_ESTABLISHED_ENV_AUTOPILOT_V1

## STOP Review: Environment for TCP Established Proof

[sexnet.phaseGHI.env_review.pass]

### Review Questions

1. **Which backend can produce SYN-ACK to guest?**
   - QEMU SLIRP user-mode networking (QEMU_NET_BACKEND=user): guest outbound TCP to
     10.0.2.2:P reaches host 127.0.0.1:P via SLIRP NAT. If a host TCP listener exists
     on that port, the host kernel's TCP stack completes the 3-way handshake and the
     guest receives a real SYN-ACK.
   - TAP (QEMU_NET_BACKEND=tap): tap0 exists but is DOWN (NO-CARRIER). Requires root
     re-setup. Not functional without host intervention.
   - **Selected: QEMU SLIRP user-mode with host TCP listener.**

2. **Does current sexnet source=3 TCP target point at a reachable peer/port?**
   - YES — TCP_REMOTE_IP = [10, 0, 2, 2] (gateway, reachable via SLIRP).
   - PARTIAL — TCP_REMOTE_PORT = 80 (hardcoded) requires root for host listener.
   - **Fix: change TCP_REMOTE_PORT to 18080 (tiny proof-target edit, not STOP FIRST).**

3. **Does run_daily_driver support passing hostfwd or target port env vars?**
   - YES — supports QEMU_USERNET_HOSTFWD for hostfwd option. Not needed for outbound
     guest-to-host connections; SLIRP NAT handles those without explicit hostfwd.
   - NO — no compile-time env var for TCP_REMOTE_PORT. Hardcoded to 80.

4. **Is a host listener needed?**
   - YES — SLIRP forwards guest SYN to host loopback. The host must have a TCP
     listener on the target port to complete the handshake. Without a listener,
     the host kernel sends RST (no service on that port), or the guest SYN times out
     (SLIRP may silently drop if no route).

5. **Can host listener run unprivileged?**
   - YES — on port >= 1024 (18080) using nc, socat, or python3.
   - NO — on port 80 (requires root/cap_net_bind_service). No passwordless sudo
     available on this system.

6. **Can proof run without root/CAP_NET_RAW?**
   - YES — QEMU SLIRP user-mode networking requires no host privileges. The host
     TCP listener on port 18080 runs unprivileged. No raw sockets needed.

7. **Does current TCP path expect dst_port=80 or 18080?**
   - TCP_REMOTE_PORT is hardcoded to 80. Changed to 18080 in this detour.

8. **If mismatch exists, can it be configured without source edit?**
   - NO — TCP_REMOTE_PORT is a `static mut u16 = 80` with no option_env! gate.

9. **If source edit is needed, is it a tiny proof-target edit or a STOP FIRST?**
   - **Tiny proof-target edit.** Changing one port constant from 80 to 18080
     affects only the TCP SYN destination port. No ABI, kernel, protocol, or
     architectural changes. No new APIs. Bounded, reversible.

10. **What are exact commands to reproduce?**
    ```bash
    # Start host TCP listener in background
    nc -l -p 18080 &
    LISTENER_PID=$!

    # Run proof
    QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
      ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_ghi_user.log

    # Cleanup
    kill $LISTENER_PID 2>/dev/null || true
    ```

### Classification

- **PASS REVIEW ONLY** — environment CAN be built safely.
  - No forbidden edits required.
  - Tiny port constant edit needed (80→18080) for unprivileged listener.
  - SLIRP backend is sufficient.
  - Host listener runs unprivileged.
  - No root/CAP_NET_RAW required.

### Selected Environment Approach

- Backend: QEMU SLIRP user-mode networking (QEMU_NET_BACKEND=user)
- NIC model: e1000e (QEMU_NET_MODEL=e1000e)
- Guest IP: 10.0.2.15 (existing)
- Gateway/target: 10.0.2.2
- Target port: 18080 (changed from 80)
- Host listener: nc -l -p 18080 (unprivileged)
- Guest local port: 7777 (existing)
- Guest local seq: 42 (existing)

### Markers

- [sexnet.phaseGHI.env_review.pass]
