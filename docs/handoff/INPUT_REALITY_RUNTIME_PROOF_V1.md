# INPUT_REALITY_RUNTIME_PROOF_V1

Date: 2026-05-20
Mission: OS_100_DAILY_DRIVER_AUTOPILOT_V1
Phase: INPUT_REALITY_RUNTIME_PROOF_V1

## Result
- INPUT_REALITY_RUNTIME_PROOF_V1: FAIL
- reason: required marker `sexinput.synthetic.click.proof.gated` not observed in runtime log

## Build
- command: `./scripts/entrypoint_build.sh`
- result: PASS (`[SEXOS ENTRYPOINT] success`)

## Runtime
- command: `./scripts/qemu_harness.sh --timeout 30`
- runtime exit: timeout expected (`124`) after 30s capture window
- log path: `/tmp/input_reality_runtime_proof_v1.log`

## Marker Summary
- `sexusb.xhci.map.ok`: 1
- `sexusb.xhci.map.bad`: 0
- `sexusb.xhci.enum.timeout`: 55
- `sexusb.route.sexinput.ready`: 1
- `sexusb.route.sexinput.missing`: 0
- `sexinput.synthetic.click.proof.gated`: 0
- `ps2.input_ring.drop`: 0

## Fault Scan
- `#PF`: 0
- `#GP`: 0
- `panic`: 0
- `fault.kill`: 0

## Notes
- Boot reached normal runtime markers (scheduler and server loops active).
- xHCI enum timeouts are present but runtime continues; acceptable per phase criteria.
- Phase verdict is FAIL strictly because the required synthetic click gating marker is absent.

## Rerun - 2026-05-20 (MISSION: INPUT_SYNTHETIC_CLICK_GATING_MARKER_FIX_V1)
- Backup patch: `/tmp/microkernel-backup/INPUT_SYNTHETIC_CLICK_GATING_MARKER_FIX_V1-20260520-014518.patch`
- Build command: `./scripts/entrypoint_build.sh`
- Build result: PASS (`[SEXOS ENTRYPOINT] success`)
- Runtime command: `./scripts/qemu_harness.sh --timeout 30 > /tmp/input_reality_runtime_proof_v1_rerun.log 2>&1 || true`
- Runtime log: `/tmp/input_reality_runtime_proof_v1_rerun.log`

### Marker Summary
- `sexinput.synthetic.click.proof.gated`: present (1)
- `sexusb.xhci.map.ok`: present (1)
- `sexusb.route.sexinput.ready`: present (1)
- `sexusb.route.sexinput.missing`: absent (0)
- `ps2.input_ring.drop`: absent (0)

### Fault Scan
- `#PF`: absent (0)
- `#GP`: absent (0)
- `panic`: absent (0)
- `fault.kill`: absent (0)

### Result
- INPUT_REALITY_RUNTIME_PROOF_V1 blocker resolved for this marker gate: PASS on required marker presence.
