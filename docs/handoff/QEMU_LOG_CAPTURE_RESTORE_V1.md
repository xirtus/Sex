# QEMU_LOG_CAPTURE_RESTORE_V1 — Handoff

## Root Cause: Empty /tmp/sexos-input.log

The existing `qemuX.sh` uses `-serial stdio`, which sends guest serial output to
QEMU's stdout (the terminal). No file is written unless stdout is redirected
(e.g., `./qemuX.sh > /tmp/sexos-input.log`). If the user ran the script
standalone without redirect, `/tmp/sexos-input.log` would be empty (or stale
from a previous `-serial file:` run).

## Working Capture Method

Use `-serial file:/tmp/sexos-input.log` instead of `-serial stdio`.

QEMU's `-serial file:PATH` writes all guest serial output directly to the
specified file. This is the simplest and most reliable approach.

## Verified: Capture Works

Script: `/tmp/run_local_input_probe_working_capture.sh`

Test run (2026-05-06):
- Boot time: ~6s to desktop
- Log size: 85 KB, 1644 lines
- Pointer recv: 15 events (all dropped = idle/no-edges, no mouse movement)
- Cursor draw: 3 (initial center position)
- No PS/2 markers: expected, no keyboard input during passive capture

## Architecture

SexOS guest uses `serial_println!()` macros which output to the serial port
(COM1 / port 0x3f8). QEMU's `-serial file:PATH` captures this.

### Known marker chains

Pointer (USB tablet → sexusb → sexinput → silk-shell → sexdisplay):
```
[sexinput.pointer.recv] class=X a0=X a1=X
[sexinput.pointer.send] class=X a0=X a1=X   (only if deltas non-zero)
[sexinput.pointer.drop] reason=...          (if deltas zero / idle)
[silk-shell.pointer.recv] class=X a0=X a1=X
[silk-shell.cursor.update] x=X y=X
[sexdisplay.cursor.draw] x=X y=X
```

PS/2 keyboard (i8042 → kernel INPUT_RING → sexinput):
(not yet verified — no keyboard injection in passive test)

## Script Usage

```bash
/tmp/run_local_input_probe_working_capture.sh
```

Opens GUI QEMU, waits for desktop (~6s), captures serial for ~2s, then
auto-kills QEMU and extracts markers. Results printed to stdout.
Log preserved at /tmp/sexos-input.log.

## Next Steps

To test PS/2 keyboard path, inject keys via QMP after desktop ready:

```bash
python3 - /tmp/sexos-qmp.sock scroll_lock 800 <<'PYEOF'
import socket, sys
path, qcode, hold_ms = sys.argv[1], sys.argv[2], int(sys.argv[3])
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect(path)
s.recv(4096)
s.sendall(b'{"execute":"qmp_capabilities"}\n')
s.recv(4096)
cmd = ('{"execute":"send-key","arguments":{"keys":[{"type":"qcode","data":"' +
       qcode + '"}],"hold-time":' + str(hold_ms) + '}}\n').encode()
s.sendall(cmd)
s.recv(4096)
s.close()
PYEOF
```

Then grep for:
- `[sexinput.ps2.scancode]`
- `[sexinput.kbd.recv]`
- `[silk-shell.keyboard.recv]` (if present)
- `[shell.action.spindle]`
