# SEXNET_PHASE_I_PROOF_TRIGGER_FIX_V1

## Goal
Ensure the daily proof profile actually reaches the source=3 Phase I TCP/HTTP lane when explicitly requested.

## Root Cause
`run_daily_driver_proof.sh` used a fixed default probe window of 30 seconds.

The source=3 lane markers (`[sexnet.tcp.entry]`, then payload/readiness markers) occur later in `sexnet` runtime after bounded ARP/cache and L2 loops. Under the 30s stop, QEMU was terminated before that lane executed, so gates reported:
- `sexnet_tcp_handshake SKIP`
- `sexnet_tcp_payload SKIP`
- `sexnet_http_phase_i_readiness SKIP`
- `sexnet_http_get_source3 SKIP`

This was a launcher/profile timing issue, not an HTTP parser failure.

## Trigger Fix
File changed: `scripts/run_daily_driver_proof.sh`

Changes:
1. Added explicit profile input:
   - `SEXNET_PHASE_I_HTTP_PROOF="${SEXNET_PHASE_I_HTTP_PROOF:-0}"`
2. Added explicit profile behavior (only when `SEXNET_PHASE_I_HTTP_PROOF=1`):
   - Preserve/force HAL probe quiet by default: `export SEXOS_HAL_TCP_PROBE="${SEXOS_HAL_TCP_PROBE:-0}"`
   - Raise runtime window to at least 90s (`PROBE_SECONDS=90` if lower)
3. Added run banner visibility:
   - `phaseI: <0|1>` printed in launcher header

Default daily behavior remains unchanged when `SEXNET_PHASE_I_HTTP_PROOF` is unset/0.

## Verification Command

```bash
pkill -f "socat TCP-LISTEN:18081" || true
socat TCP-LISTEN:18081,reuseaddr,fork SYSTEM:'printf "HTTP/1.1 200 OK\r\nContent-Length: 13\r\nConnection: close\r\n\r\nhello sexnet\n"' &

SEXOS_HAL_TCP_PROBE=0 \
QEMU_NET_BACKEND=user \
QEMU_NET_MODEL=e1000 \
ENABLE_QEMU_USERNET_E1000=1 \
SEXNET_PHASE_I_HTTP_PROOF=1 \
./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_i_trigger_fix.log
```

## Evidence
From `/tmp/sexnet_phase_i_trigger_fix.log`:
- `[sexnet.tcp.entry] state=CLOSED local_port=7777 remote=10.0.2.2:18081 ok=1`
- `[sexnet.tcp.payload.tx.guard] state=SYN_SENT ok=0 reason=not_established`
- `[sexnet.phaseI.readiness] established=0 payload_tx=0 source=3 ok=0`
- `[qemu.net.config] backend=user model=e1000 usernet=1 hostfwd=none tap_if=tap0`

Gate output for this run:
- `sexnet_tcp_handshake PASS` (env-limited honest)
- `sexnet_tcp_payload PASS` (guard proven)
- `sexnet_http_phase_i_readiness SKIP` (not established)
- `sexnet_http_get_source3 SKIP` (not fully proven)
- Final daily result: `FINAL: PASS (248 gates proved, 49 skipped, 0 faults)`

## Scope Compliance
- Backup created before edits:
  - `/tmp/microkernel-backup/SEXNET_PHASE_I_PROOF_TRIGGER_FIX_V1-20260519-131522.diff`
- No kernel edits
- No sex-pdx/global ABI edits
- No TCP/HTTP logic rewrite
- No gate weakening
- No fake PASS
