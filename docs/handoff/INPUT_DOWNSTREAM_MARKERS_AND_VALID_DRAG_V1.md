# INPUT_DOWNSTREAM_MARKERS_AND_VALID_DRAG_V1

Status: PASS.

## Scope

This pass did not change `servers/sexusb/src/main.rs`, kernel, sex-pdx ABI, sexdisplay, sexinput behavior, gestures, trackpad, or app-owned drag policy.

Backups made before edits:

```text
/tmp/sexusb_cadence_current_before_downstream.diff
/tmp/silk_shell_before_downstream.diff
/tmp/input_current_tier_gate_before_downstream.diff
```

## Root Cause

Movement was working, but the canonical success markers were attached to the main OP_HID_EVENT receive branch while the proven live path used the inline/drain HID handler. That inline path applied relative pointer movement through `apply_rel_pointer()` and emitted `[shell.rel.transfer]` / `[usb.pointer.cursor.bounds]`, but did not emit:

```text
[usb.pointer.shell.apply]
[input.pointer.move.ok]
```

Drag rejection was a proof stimulus bug, not a shell policy bug. The previous QMP sequence pressed on Mesh/app-owned content and correctly hit:

```text
[shell.drag.begin.reject] reason=focused_not_shell_surface
```

The policy was left intact. The valid proof target is shell-owned surface 100 content. The successful run started drag at:

```text
x=591 y=304 target=100 app_owned=0 allow=1
```

## Implementation

`servers/silk-shell/src/main.rs`:

- Added canonical movement markers to the inline/drain EV_REL path after `apply_rel_pointer()` succeeds.
- Added inline/drain EV_REL drag/resize movement application so real captured drag updates the target while pointer motion is handled by the drain path.
- Added canonical drag end marker on the main OP_HID_EVENT release path.

`scripts/input_current_tier_gate.sh`:

- Added required transfer/rearm markers.
- Kept keyboard required for `INPUT_100_CURRENT_TIER_V1`.
- Expanded fault scan output to include reboot loop and freeze checks without matching benign `frozen=0` checkpoint markers.

`scripts/gate_0_2.sh`:

- Changed QMP pointer stimulus to steer to shell-owned surface 100 content.
- Added a drain delay before button-down so the guest observes the drag start after cursor motion reaches the target.

## Proof

Build:

```text
./scripts/entrypoint_build.sh
[SEXOS ENTRYPOINT] success
```

Runtime:

```text
GATE_DIR=/tmp/input_downstream_valid_drag ./scripts/gate_0_2.sh
cp /tmp/input_downstream_valid_drag/sexos-input.log logs/qemu-latest.log
```

The wrapper exited nonzero after terminating the bounded QEMU probe, but the generated serial log contains the required runtime proof.

Gate:

```text
scripts/input_current_tier_gate.sh logs/qemu-latest.log
INPUT_100_CURRENT_TIER_V1: PASS
[input.faultscan.ok] pf=0 gp=0 panic=0 fault_kill=0 reboot_loop=0 freeze=0 storm=0
```

Observed markers:

```text
[sexusb.hid.transfer.event]
[sexusb.hid.rearm.ok]
[usb.hid.pointer.emit]
[usb.pointer.shell.apply]
[input.pointer.move.ok]
[input.button.down.ok]
[input.button.up.ok]
[silk.click.hit.live.ok]
[silk.focus.set.ok]
[silk.drag.begin.ok] sid=100 zone=content x=591 y=304
[silk.drag.move.ok] sid=100 dx=-7 dy=4
[silk.drag.end.ok]
[input.keyboard.keydown.ok]
[input.keyboard.keyup.ok]
```

Drag policy proof:

```text
[shell.drag.policy] target=100 kind=1 app_owned=0 chrome_owned=0 allow=1 reason=none
```

## Remaining Failures

None in this proof log. Keyboard unexpectedly passed in the final QMP run, so `INPUT_100_CURRENT_TIER_V1` passed honestly rather than being promoted with keyboard still failing.
