# TCP_SLIRP_TCP_FORWARDING_AUDIT_V1

Date: 2026-05-17

## Goal
Determine whether QEMU SLiRP (`-netdev user`) is forwarding raw outbound TCP SYNs from this e1000e driver lane, and whether deterministic TCP proof needs hostfwd, local server targeting, or a backend shift (`tap`).

## Current Lane Truth
From `scripts/run_daily_driver_proof.sh`:
- Network args are currently:
  - `-netdev user,id=net0`
  - `-device e1000e,netdev=net0` (with `QEMU_NET_MODEL=e1000e`)
- No `hostfwd` is configured.
- No tap/bridge backend is configured in this proof script.

From latest proofs:
- ARP gateway resolution is reliable (`gateway_known=1`, real gw MAC).
- TX descriptor post is reliable for SYN (`tx_dd=1`, repeated attempts).
- SYN-ACK absent in bounded window across:
  - `example.com` Cloudflare target path
  - controlled known-good override path (`34.223.124.45:80`)
- RST absent.
- Final ACK correctly deferred.
- HTTP GET correctly deferred.

Interpretation:
- The blocker is likely outside L2 and frame construction.
- Most probable remaining domains: SLiRP forwarding behavior, outbound TCP policy, or NAT/state expectations not met by this raw driver path.

## Audit Matrix (Next Execution)

1) `user` backend baseline (current)
- Keep: `-netdev user,id=net0 -device e1000e,netdev=net0`
- Expectation: reproduce current no-SYN-ACK/no-RST profile.

2) `user` backend with `hostfwd` + local host listener
- Add: `-netdev user,id=net0,hostfwd=tcp::18080-:80`
- Start host local listener (`python3 -m http.server 18080` or equivalent) before boot.
- Guest target remains `10.0.2.2:80` or explicit forwarded path under mission wiring.
- Purpose: validate SLiRP NAT/state + TCP return path with controlled host endpoint.

3) `tap` backend (if host permissions/network allow)
- Use `-netdev tap,...` with e1000e.
- Purpose: remove SLiRP from path and check whether SYN/SYN-ACK appears.

## Decision Rules
- If case (2) works and (1) fails:
  - likely SLiRP policy/path issue for arbitrary external TCP, not stack serialization.
- If case (3) works and (1)/(2) fail:
  - SLiRP limitation/mismatch strongly indicated.
- If all fail with `tx_dd=1` and no SYN-ACK/RST:
  - inspect e1000e TX checksum/offload expectations vs backend parser assumptions.

## Guardrails
- No final ACK unless SYN-ACK in same attempt.
- No HTTP GET.
- Keep bounded retries/polls.
- Preserve e1000 default skip behavior.

## Proposed Immediate Next Mission
`TCP_SLIRP_TCP_FORWARDING_AUDIT_EXEC_V1`
- add model-gated netdev option support in proof runner for:
  - `QEMU_USERNET_HOSTFWD=tcp::18080-:80`
  - optional backend selector (`QEMU_NET_BACKEND=user|tap`)
- add markers that record exact QEMU net mode used per run.
- run three-case matrix above and classify result.

---

## Execution Results (This Session)

Runner updates applied:
- `scripts/run_daily_driver_proof.sh` now supports:
  - `QEMU_NET_BACKEND=user|tap`
  - `QEMU_USERNET_HOSTFWD=<rule>` (user mode only)
  - `QEMU_TAP_IFNAME=<ifname>` (tap mode)
- It appends a runtime marker to each log:
  - `[qemu.net.config] backend=... model=... usernet=... hostfwd=... tap_if=...`

### Matrix

| Case | Command Profile | Boot Result | TCP Probe Result |
|---|---|---|---|
| A | `backend=user`, no hostfwd | QEMU booted | SYN tx_dd=1 on all attempts; `synack_seen=0`, `rst_seen=0`; final ACK/HTTP deferred |
| B | `backend=user`, `hostfwd=tcp:127.0.0.1:18081-:80` | QEMU failed early | `Could not set up host forwarding rule` (log truncated, no guest runtime) |
| C | `backend=tap`, `tap0` | QEMU failed early | `Could not open '/dev/net/tun': No such file or directory` |

### Evidence

- Case A log: `/tmp/sexos_tcp_slirp_audit_user.log`
  - `[qemu.net.config] backend=user model=e1000e usernet=1 hostfwd=none tap_if=tap0`
  - `[tcp.syn.tx.post]` attempts 1..3 with `tx_dd=1`
  - `[tcp.syn.rx.synack] ... synack_seen=0 rst_seen=0`
  - `[tcp.http.target.known_good.probe.done] ... ok=1`
  - `[tcp.handshake.ack.tx.post] ... sent=0`
  - `[http.get.send.proof] sent=0`

- Case B log: `/tmp/sexos_tcp_slirp_audit_user_hostfwd.log`
  - `[qemu.net.config] backend=user model=e1000e usernet=1 hostfwd=tcp:127.0.0.1:18081-:80 tap_if=tap0`
  - QEMU error: `Could not set up host forwarding rule`

- Case C log: `/tmp/sexos_tcp_slirp_audit_tap.log`
  - `[qemu.net.config] backend=tap model=e1000e usernet=1 hostfwd=none tap_if=tap0`
  - QEMU error: `Could not open '/dev/net/tun': No such file or directory`

## Classification

- Stack TX path remains healthy and bounded under user backend.
- Hostfwd path is unavailable in current QEMU/host configuration.
- Tap backend is unavailable in current environment (`/dev/net/tun` missing).
- Next reliable discriminator should be a controlled guest-to-host TCP endpoint reachable via SLiRP gateway (`10.0.2.2`) without hostfwd dependency.
