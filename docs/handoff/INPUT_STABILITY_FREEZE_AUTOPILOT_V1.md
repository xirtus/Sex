# INPUT_STABILITY_FREEZE_AUTOPILOT_V1

Date: 2026-05-20
Mission: INPUT_STABILITY_FREEZE_AUTOPILOT_V1

## Changed Files
- servers/sexusb/src/main.rs
- servers/sexinput/src/main.rs
- kernel/src/interrupts.rs
- scripts/daily_driver_master_gate.sh

## Freeze Candidates Addressed
- B1 fixed (bounded xHCI waits): added bounded poll accounting/timeout markers for slot completion and interrupt ring wait.
- B2 deferred (single-device ghost/unready port behavior): no multi-device expansion or scan-policy rewrite in this mission.
- B3 deferred (pdx_call_and_reply spin in Quil/SexFiles path): out of allowed scope; no ABI or storage/quil edits.
- B4 fixed (PS/2 ring overflow risk): INPUT_RING capacity increased from 256 to 512 based on existing `ps2.input_ring.drop` evidence path.
- B5 partially addressed: kept `SEXUSB_SYNTHETIC=false` unchanged, but added bounded BAR retry + timeout/degrade to avoid hard stall if real xHCI path fails.
- B6 fixed (silent sexinput route state): added explicit ready/missing route markers.
- B7 fixed (synthetic click IPC storm): gated silkbar synthetic click proof to one bounded sequence and added proof-gated marker.

## Markers Added
- `[sexusb.xhci.map.bad] reason=bar_zero_after_retry attempts=3 ok=0`
- `[sexusb.xhci.enum.timeout] phase=SLOT polls=N ok=0`
- `[sexusb.xhci.enum.timeout] phase=RING polls=N ok=0`
- `[sexusb.route.sexinput.ready] slot=S ok=1`
- `[sexusb.route.sexinput.missing] slot=S ok=0`
- `[sexinput.synthetic.click.proof.gated] ok=1`

## Kernel Touch Justification
`kernel/src/interrupts.rs` was touched for the one-line permitted capacity change:
- `RingBuffer<u8, 256>` -> `RingBuffer<u8, 512>`
Reason: source contains `ps2.input_ring.drop` marker path proving ring-full drops are a known event surface.

## Added Daily-Driver Gates (Additive)
- `input_freeze_xhci_bounded`
- `input_freeze_route_ready_or_missing`
- `input_freeze_synthetic_click_gated`
- `input_freeze_no_faults`

## Validation Run
- Command: `./scripts/entrypoint_build.sh`
- Result: PASS (`[SEXOS ENTRYPOINT] success`)

## STOP FIRST Boundaries (still active)
- Any fix requiring kernel cap-init grant edits for route wiring.
- Any fix requiring `sex-pdx` ABI changes.
- Any fix requiring scheduler changes.
- Any fix requiring display/sexdisplay or silk-shell policy edits.
- Any fix requiring broad xHCI ring/descriptor redesign.
