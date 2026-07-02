# NETWORK_RELIABILITY_GATE_V1

Date: 2026-05-19
Branch: master
Task: 67 — Phase M network reliability gate

## Goal

Add a `network_reliability` aggregate gate that gates Phase M reliability/stress proofs.

## Sub-Gates

| Gate | Description | Markers Required |
|------|-------------|-----------------|
| `sexnet_source3_multi_fetch` | N=3 repeated HTTP GET | `sexnet.source3.multi_fetch.done` success>=3 ok=1 |
| `sexnet_descriptor_reuse` | TX/RX descriptor reuse proof | `sexnet.descriptor.reuse.proof.done` tx_reuse>=3 ok=1 |
| `sexnet_http_retry_policy` | Bounded retry/timeout | `sexnet.http.retry.proof.done` bounded=1 ok=1 |
| `browser_remote_render_stability` | Browser render stability | `browser.sexnet.render.stability.done` iterations>=3 ok=1 |
| `network_source3_long_run` | Long-run no fault | `network.source3.long_run.done` faults=0 ok=1 |

## Aggregate Gate

`network_reliability` PASS only when:
- All 5 Phase M sub-gates PASS
- Existing source3 primary gates PASS (sexnet_http_get_source3, sexnet_netdiag_source3_primary, browser_sexnet_remote_page, network_source3_primary)
- Zero faults

`network_reliability` SKIP when:
- All 5 Phase M sub-gates SKIP (profile not enabled)
- Default daily run without explicit Phase M profile

`network_reliability` FAIL when:
- Any sub-gate fails
- Faults detected
- Multi-fetch done but iterations corrupted
- Body/status mismatch
- Unbounded retry detected

## Implementation

- Added to `scripts/daily_driver_master_gate.sh`
- Added to `scripts/run_daily_driver_proof.sh` (SEXNET_PHASE_M_RELIABILITY_PROOF=1)
- Source markers in `servers/sexnet/src/main.rs` (PHASE_M_RELIABILITY_ENABLED)
- Browser markers in `servers/silk-shell/src/main.rs` (BROWSER_RENDER_STABILITY_PROOF_ENABLED)

## Proof Commands

```bash
# Start HTTP peer
pkill -f "python3 /tmp/sexnet_http_peer.py" 2>/dev/null || true
python3 /tmp/sexnet_http_peer.py &

# Build
./scripts/entrypoint_build.sh

# Run Phase M reliability proof (120s probe)
SEXNET_PHASE_M_RELIABILITY_PROOF=1 \
SEXOS_HAL_TCP_PROBE=0 \
QEMU_NET_BACKEND=user \
QEMU_NET_MODEL=e1000 \
ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_m_reliability.log

# Gate
./scripts/daily_driver_master_gate.sh /tmp/sexnet_phase_m_reliability.log
```
