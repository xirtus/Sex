# SEXUSB_TRANSFER_EVENT_RESTORE_V1

Status: transfer/rearm restored under real injected USB tablet input; current-tier gate still FAILS downstream.

## Scope

Allowed source scope for this mission was `servers/sexusb/src/main.rs`, with this handoff document. No kernel, sex-pdx ABI, sexdisplay, sexinput, silk-shell, gesture, trackpad, descriptor, or broad USB rewrite was required.

## Backup

Start diff was saved before inspection:

```text
/tmp/sexusb_transfer_event_restore_start.diff
```

## Root Cause

The earlier "no transfer events" runtime result was not reproduced once the run injected real absolute USB tablet events through QMP. With absolute tablet input, xHCI/HID interrupt-IN Transfer Events are recognized and the cadence patch re-arms completed TRBs one-for-one.

No slot1 xHCI invariant violation was observed in the successful runtime proof:

- Cycle/DCS: the endpoint produced Transfer Events from the primed ring, so the producer cycle and endpoint DCS are compatible for slot1.
- Link TRB: completions reached indexes `0..7` and wrapped to `0..3`, so data TRBs did not overwrite the link TRB at index 8 in this proof.
- Doorbell: slot1 doorbell was effective after priming.
- Event filter: valid slot1 endpoint 3 Transfer Events were accepted.
- Pointer mapping: event TRB pointers mapped to completed indexes and did not fail silently.
- Wait loop: multiple events were handled; the loop did not stop at the first event in this proof.

Slot2 priming and doorbell remain structurally present in `servers/sexusb/src/main.rs`, but this single-tablet QEMU proof did not enumerate a second HID endpoint, so slot2 is not runtime-proven here.

## Current sexusb Behavior

- `HID_INFLIGHT = 8`.
- Each interrupt-IN TRB uses `intr_report_phys + index * 64`.
- Transfer Event TRB pointer maps back to the transfer-ring index.
- Completed index `N` decodes report buffer `N`.
- Completed index `N` is immediately re-armed with exactly one replacement TRB.
- Observed runtime index sequence: `0,1,2,3,4,5,6,7,0,1,2,3`.

## Proof

Build:

```text
./scripts/entrypoint_build.sh
[SEXOS ENTRYPOINT] success
```

Runtime:

```text
/tmp/sexusb_transfer_restore_abs.log
logs/qemu-latest.log
```

The absolute QMP tablet injection produced 12 transfer/rearm pairs:

```text
[sexusb.hid.transfer.event] slot=1 ep=3 index=0
[sexusb.hid.rearm.ok] slot=1 ep=3 index=0
...
[sexusb.hid.transfer.event] slot=1 ep=3 index=7
[sexusb.hid.rearm.ok] slot=1 ep=3 index=7
[sexusb.hid.transfer.event] slot=1 ep=3 index=0
[sexusb.hid.rearm.ok] slot=1 ep=3 index=0
```

Pointer delivery reached the shell path through existing markers:

```text
[usb.hid.pointer.emit]
[shell.rel.transfer]
[usb.pointer.cursor.bounds]
```

The literal current-tier gate still fails because these markers are absent in this run:

```text
[usb.pointer.shell.apply]
[input.pointer.move.ok]
[silk.drag.move.ok]
```

Gate:

```text
scripts/input_current_tier_gate.sh logs/qemu-latest.log
INPUT_100_CURRENT_TIER_V1: FAIL
[input.faultscan.ok] pf=0 gp=0 panic=0 fault_kill=0 storm=0
```

## Remaining Failures

- Keyboard remains FAIL as expected for this lane.
- `usb.pointer.shell.apply` is missing, even though movement reaches `shell.rel.transfer` and `usb.pointer.cursor.bounds`.
- `input.pointer.move.ok` is missing.
- Real drag move is missing. The proof showed the real button-down path hit a focused app-owned target and shell drag begin was rejected:

```text
[shell.drag.begin.reject] reason=focused_not_shell_surface target=202 kind=1 buttons=0x1 dx=0 dy=0
```

Fixing the missing shell/app drag marker would require work outside the allowed sexusb transfer-event restore scope, or a different proof sequence that starts drag on an allowed shell surface.

## Result

Do not merge or label the broader cadence lane as fully fixed. The transfer/rearm link is restored/proven for slot1 under real absolute tablet input, but the full input current-tier gate remains FAIL.
