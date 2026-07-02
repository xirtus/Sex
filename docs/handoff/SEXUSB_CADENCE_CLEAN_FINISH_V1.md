# SEXUSB_CADENCE_CLEAN_FINISH_V1

Status: partial PASS.

## Scope

Edited only `servers/sexusb/src/main.rs` for code. No kernel, sex-pdx ABI, sexdisplay, sexinput, silk-shell, gesture, trackpad, or descriptor redesign changes.

Backup captured before edits:

```text
/tmp/sexusb_pre_codex_cadence.diff
```

## Dirty Patch Cleanup

- `UF_STRIDE` was not present in the current file.
- The prior partial patch had `HID_INFLIGHT = 1` and `SLOT2_INFLIGHT = 1`, so it still behaved like a single outstanding TRB per endpoint.
- The prior partial patch also used producer-style rearm (`intr_prod` / `slot2_intr_prod`) instead of completed-index rearm.
- The old single-TRB priming path was replaced by one bounded eight-TRB priming loop per HID endpoint.

## Implementation

- `HID_INFLIGHT = 8`.
- Each HID endpoint ring uses eight Normal TRBs at indexes `0..7` and a Link TRB at index `8`.
- Each TRB index uses a unique report buffer slot:

```text
report_phys + index * 64
```

- Transfer Event TRB pointer is mapped back to a bounded index.
- Slot1 and slot2 both decode only the completed index buffer.
- Slot1 and slot2 both rearm exactly one replacement TRB at the same completed index.
- Rearm marker now includes the rearmed index:

```text
[sexusb.hid.rearm.ok] slot=N ep=N index=N
```

- Transfer marker now includes the completed index:

```text
[sexusb.hid.transfer.event] slot=N ep=N index=N
```

## Proof

Build:

```text
./scripts/entrypoint_build.sh
PASS
```

Runtime:

```text
./scripts/qemu_harness.sh --timeout 30 --markers
exit=124 timeout after bounded run
log=logs/qemu-latest.log
```

Observed runtime markers:

```text
[sexusb.hid.inflight.init] slot=1 ep=3 count=8
[usb.hid.pointer.emit]
[input.button.down.ok]
[silk.drag.begin.ok]
[input.button.up.ok]
[silk.drag.end.ok]
```

Not observed in the headless run:

```text
[sexusb.hid.transfer.event]
[sexusb.hid.rearm.ok]
[usb.pointer.shell.apply]
[input.pointer.move.ok]
[silk.drag.move.ok]
```

Input gate:

```text
scripts/input_current_tier_gate.sh logs/qemu-latest.log
INPUT_100_CURRENT_TIER_V1: FAIL
```

Fault scan from input gate:

```text
[input.faultscan.ok] pf=0 gp=0 panic=0 fault_kill=0 storm=0
```

Direct counts:

```text
#PF=0
#GP=0
panic=0
fault.kill=0
usb.pointer.shell.apply=0
```

## Remaining Failures

- Headless QEMU did not produce transfer/rearm cadence markers in the 30 second window.
- `usb.pointer.shell.apply` missing.
- `input.pointer.move.ok` missing.
- `silk.drag.move.ok` missing.
- Keyboard markers still missing, expected for this lane.
- Do not claim `INPUT_100_CURRENT_TIER` from this run.

## Next Narrow Step

Run an operator or evdev-backed USB pointer probe so QEMU emits real HID completions:

```text
USB_POINTER_PROBE_SECONDS=45 ./scripts/usb_pointer_real_report_operator_probe.sh evdev /tmp/sexusb_cadence_real_pointer.log
scripts/input_current_tier_gate.sh /tmp/sexusb_cadence_real_pointer.log
```

Use the same fault scan terms: `#PF`, `#GP`, `panic`, `fault.kill`, reboot loop, freeze, and IPC storm.
