# SEXNET_TCP_PSH_ACK_WIRESHAPE_FIX_V1

Date: 2026-05-19
Mission: SEXNET_TCP_PSH_ACK_WIRESHAPE_FIX_V1
Status: PASS IMPLEMENTED / REVIEW ONLY (runtime lane did not reach Phase I in this environment)

## Scope
- File edited: `servers/sexnet/src/main.rs`
- No kernel/ABI/parser changes.
- Added bounded diagnostics for PSH+ACK shape and peer ACK progression.
- Fixed TX tail publish for descriptor 7 on 8-descriptor ring.

## Root Cause (implemented)
PSH+ACK payload TX used descriptor index 7 but published `TDT=8` on an 8-descriptor ring (`TDLEN=128`, 8 x 16-byte descriptors). Tail publication is ring-indexed and should wrap after descriptor 7. This was corrected to publish wrapped tail (`TDT=0`) for the desc7 payload post.

## Code Changes
1. `servers/sexnet/src/main.rs`
- Added `expected_ack_after_payload = tcp_seq + payload_len`.
- Added diagnostics:
  - `[sexnet.tcp.psh_ack.shape] ...`
  - `[sexnet.tcp.psh_ack.payload.peek.hex] ...`
  - `[sexnet.tcp.psh_ack.payload.peek.ascii] ...`
  - `[sexnet.tcp.psh_ack.ack_expect] ...`
  - `[sexnet.tcp.psh_ack.peer_ack] ...`
- Fixed payload TX tail publish:
  - before: `write TDT=8` after desc7
  - after: `write TDT=0` after desc7 (ring wrap)

## Proof Attempt (this environment)
Commands executed:
- `./scripts/entrypoint_build.sh`
- `SEXOS_HAL_TCP_PROBE=0 QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000 ENABLE_QEMU_USERNET_E1000=1 ./scripts/run_daily_driver_proof.sh /tmp/sexnet_tcp_psh_ack_wireshape_fix.log`
- `./scripts/daily_driver_master_gate.sh /tmp/sexnet_tcp_psh_ack_wireshape_fix.log`

Observed blocker markers in this run:
- `[sexnet.http.handshake] ... allowed=0 ... reason=no_network_grant_no_route`
- `[sexnet.http.truth] request_sent=0 ... dns=0 tcp=0 http=0 ...`
- Gate output: `sexnet_tcp_handshake SKIP`, `sexnet_tcp_payload SKIP`, `sexnet_http_phase_i_readiness SKIP`, `sexnet_http_get_source3 SKIP`

Meaning: the run did not enter the Phase I TCP handshake/payload lane in this environment, so peer ACK advance (`43 -> 127`) and HTTP payload RX cannot be claimed from this proof log.

## Next Verification Required
Re-run in a lane that actually reaches Phase I TCP handshake (same command profile as known-good handshake/payload run). Confirm:
- `[sexnet.tcp.psh_ack.peer_ack] ... advanced=1 ...`
- `[sexnet.http.response.rx] bytes>0 ... ok=1`
- `[sexnet.http.status.proof.done] status=200 ok=1`
- `sexnet_http_get_source3 PASS`
- `faults_zero PASS`
