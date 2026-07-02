# USB_HID_QEMU_DEVICE_ROUTE_AUDIT_V1

Date: 2026-05-20

## Scope
- No OS code changes.
- Allowed files only: `dev.sh`, `scripts/qemu_harness.sh`, `scripts/qmp_input_probe.py`, this doc, append-only update to `USB_HID_REAL_REPORT_PROOF_V1.md`.

## Backup before changes
- `cp -a dev.sh /tmp/dev.sh.bak`
- `cp -a scripts/qemu_harness.sh /tmp/qemu_harness.sh.bak`
- `cp -a scripts/qmp_input_probe.py /tmp/qmp_input_probe.py.bak`

## Edits made
- `scripts/qmp_input_probe.py`
  - Added `--delay SECONDS` argument parsing.
  - Added delay value to startup banner.
  - Uses per-run delay value for command pacing.
- `scripts/qemu_harness.sh`
  - Added explicit resolved USB device arg visibility (e.g. `-device usb-mouse,bus=xhci.0` / `-device usb-tablet,bus=xhci.0`).
  - Added explicit QMP arg visibility in banner.
  - Print-cmd mode now shows resolved `usb-*` device name.

## Validation
- `bash -n dev.sh` -> PASS
- `bash -n scripts/qemu_harness.sh` -> PASS
- `python3 -m py_compile scripts/qmp_input_probe.py` -> PASS
- `./scripts/entrypoint_build.sh` -> PASS (warnings only)

## Exact run commands
### Mouse lane
```bash
sock="/tmp/sexos-qmp-mouse-$$.sock"
log="/tmp/usb_hid_qmp_mouse_route.log"
SEXUSB_QEMU_DEVICE=mouse SEXOS_QMP_SOCK="$sock" ./scripts/qemu_harness.sh --timeout 30 --qmp "$sock" > "$log" 2>&1 &
pid=$!
for _ in $(seq 1 80); do [ -S "$sock" ] && break; sleep 0.25; done
./scripts/qmp_input_probe.py "$sock" mouse --delay 8 > /tmp/qmp_probe_mouse_route.log 2>&1 || true
wait "$pid" || true
rg -n "sexusb\.hid\.report\.nonzero|sexusb\.hid\.report\.idle|sexusb\.hid\.report\.timeout|sexusb\.route\.sexinput|sexusb\.xhci|#PF|#GP|panic|fault.kill|dev\.qmp|qmp" "$log" /tmp/qmp_probe_mouse_route.log || true
```

### Tablet lane
```bash
sock="/tmp/sexos-qmp-tablet-$$.sock"
log="/tmp/usb_hid_qmp_tablet_route.log"
SEXUSB_QEMU_DEVICE=tablet SEXOS_QMP_SOCK="$sock" ./scripts/qemu_harness.sh --timeout 30 --qmp "$sock" > "$log" 2>&1 &
pid=$!
for _ in $(seq 1 80); do [ -S "$sock" ] && break; sleep 0.25; done
./scripts/qmp_input_probe.py "$sock" mouse --delay 8 > /tmp/qmp_probe_tablet_route.log 2>&1 || true
wait "$pid" || true
rg -n "sexusb\.hid\.report\.nonzero|sexusb\.hid\.report\.idle|sexusb\.hid\.report\.timeout|sexusb\.route\.sexinput|sexusb\.xhci|#PF|#GP|panic|fault.kill|dev\.qmp|qmp" "$log" /tmp/qmp_probe_tablet_route.log || true
```

## Result table
| Lane | QMP connect + capabilities | QMP input-send-event | SexOS boot markers | `sexusb.route.sexinput.ready` | `sexusb.hid.report.nonzero` | `sexusb.hid.report.timeout` | Fault markers |
|---|---|---|---|---|---|---|---|
| mouse | yes | yes (`attempted=3 succeeded=3`) | yes (`sexusb.xhci.*` present) | yes | no | yes | none observed (`#PF/#GP/panic/fault.kill` absent in grep) |
| tablet | yes | yes (`attempted=3 succeeded=3`) | yes (`sexusb.xhci.*` present) | yes | no | yes | none observed (`#PF/#GP/panic/fault.kill` absent in grep) |

## Classification
- Final: **SKIP**
- PASS criterion not met: no `sexusb.hid.report.nonzero` in either lane.
- Not FAIL: boot/log capture works; no crash/fault markers in audited grep.

## Root cause category
- **QMP-to-USB-HID delivery mismatch in this QEMU lane**.
- Evidence:
  - QMP transport works (greeting, capabilities, command returns all success).
  - SexOS xHCI + route init works (`sexusb.route.sexinput.ready` appears).
  - HID runtime still times out (`sexusb.hid.report.timeout`) with no nonzero report in both mouse/tablet configurations.
- This points to event delivery not reaching the polled USB interrupt endpoint in a way SexOS consumes, despite successful QMP command acceptance.

## Recommended next path
1. Physical USB proof lane (preferred): run with real host USB HID passthrough and validate `sexusb.hid.report.nonzero`.
2. If QEMU-only path must continue: audit QEMU input event routing semantics for `input-send-event` vs emulated USB endpoint feed in this machine/device combo.

## Artifacts
- `/tmp/usb_hid_qmp_mouse_route.log`
- `/tmp/qmp_probe_mouse_route.log`
- `/tmp/usb_hid_qmp_tablet_route.log`
- `/tmp/qmp_probe_tablet_route.log`
