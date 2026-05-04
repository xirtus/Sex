# INPUT_SOLVE_PLAN_V1

## Status: QEMU 11.0.0 USB HID Input Routing Broken

### Finding

QEMU 11.0.0 on this host does not deliver host input events to emulated USB HID devices. This affects both pointer USB mouse/tablet and keyboard USB input.

Guest stack proven working:

- sexusb enumerates USB HID devices
- Interrupt-IN polling produces periodic HID reports
- Reports are forwarded to sexinput via PDX IPC
- sexinput decodes and routes events to silk-shell

Broken layer:

```text
QEMU host input -> emulated USB HID device
```

Even with:

* `-machine q35,i8042=off`
* `-device usb-kbd,bus=xhci.0,display=sdl`
* QMP `input-send-event`
* HMP `sendkey`

USB HID reports remain zero for live input. QEMU accepts commands but does not forward events into the USB HID report buffer.

## Two-Track Solution

### Track A: Interactive dev.sh modes

| Env var                              | Effect                                             |
| ------------------------------------ | -------------------------------------------------- |
| `SEXOS_QEMU_I8042=off`               | `-machine q35,i8042=off`                           |
| `SEXUSB_QEMU_DEVICE=kbd`             | `-device usb-kbd,bus=xhci.0`                       |
| `SEXUSB_QEMU_DEVICE=kbd-display-sdl` | `-device usb-kbd,bus=xhci.0,display=sdl`           |
| `SEXOS_QEMU_QMP=1`                   | `-qmp unix:/tmp/sexos-qmp.sock,server=on,wait=off` |

Track A configures QEMU correctly for USB HID input, but this host/QEMU combo still does not route host events. It may work with a different QEMU version or host backend.

### Track B: Deterministic QMP injection

`scripts/qmp_input_probe.py` connects to QMP and injects keyboard events using QEMU 11.0.0 `KeyValue` union format:

```json
{
  "type": "key",
  "data": {
    "down": true,
    "key": {"type": "qcode", "data": "w"}
  }
}
```

Usage:

```fish
env SEXOS_QEMU_I8042=off SEXOS_QEMU_QMP=1 SEXUSB_QEMU_DEVICE=kbd ./dev.sh run &
./scripts/qmp_input_probe.py /tmp/sexos-qmp.sock
```

## Why This Is the Best Path

* No guest architecture change
* No xHCI refactor
* No kernel ABI change
* No PS/2 product path
* Deterministic injection path for future proof work
* dev.sh modes remain useful if another QEMU/host routes USB HID correctly

## Testing

```fish
env SEXOS_KEYBOARD_CURSOR=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
env SEXOS_QEMU_I8042=off SEXOS_QEMU_QMP=1 SEXUSB_QEMU_DEVICE=kbd ./dev.sh run &
./scripts/qmp_input_probe.py /tmp/sexos-qmp.sock
```

## Full Cursor Proof Gate

Required markers:

```text
keyboard_cursor.key         > 0
keyboard_cursor.emit.rel    > 0
shell.hid.rel.live          > 0
shell.cursor.surface.update > 0
```

## Production Note

This handoff documents a QEMU host-routing limitation, not a SexOS USB architecture failure.
