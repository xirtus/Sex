# USB_HID_REAL_REPORT_PROOF_V1

Status: SKIP

## Scope
- Allowed-file mission only.
- Runtime marker insertion in existing `sexusb` interrupt-IN polling path.
- No kernel/ABI/scheduler/pointer-producer/shell/display edits.

## Marker contract added
- `[sexusb.hid.report.nonzero] len=N b0=XX b1=XX b2=XX ok=1`
- `[sexusb.hid.report.idle] len=N ok=1`
- `[sexusb.hid.report.timeout] polls=N ok=0`

## Exact marker insertion point
- File: `servers/sexusb/src/main.rs`
- Location: continuous interrupt-IN poll loop after Transfer Event completion (`intr_actual` and `b0..b2` read), and timeout path when `intr_ok` is false.
- Path reused: existing xHCI interrupt-IN TRB queue -> poll Transfer Event -> decode/forward flow.

## Runtime log
- Log path: `/tmp/usb_hid_real_report_proof_v1.log`
- Command used: `timeout 30 ./dev.sh run-nographic > /tmp/usb_hid_real_report_proof_v1.log 2>&1`

## Observed markers (excerpt)
- `[sexusb.route.sexinput.ready] slot=9 ok=1`
- `[sexusb.xhci.enum.timeout] phase=RING polls=2 ok=0`
- `[sexusb.hid.report.timeout] polls=2 ok=0`
- Continued bounded timeout markers (`sexusb.xhci.enum.timeout`) through runtime window.

## QEMU/QMP or physical input limitation
- QMP injection lane was attempted with `SEXOS_QEMU_QMP=1`, but host runtime refused socket bind:
  - `Failed to bind socket to /tmp/sexos-qmp.sock: Operation not permitted`
- In this lane, no injected HID activity was possible, and no physical GUI interaction lane was used.

## Fault summary
- No `#PF`, `#GP`, `panic`, or `fault.kill` markers observed in the runtime log.

## STOP FIRST boundaries
- STOP FIRST if proving nonzero requires:
  - kernel edits
  - `sex-pdx` ABI edits
  - USB ring redesign
  - descriptor parser rewrite
  - broad path rewrite to locate report decode path

## Result rationale
- Build: PASS
- Runtime safety (fault-free): PASS
- Real/QMP-injected nonzero HID report proof: not provable in this host lane due to QMP socket permission block.
- Final classification for this mission run: SKIP.

## 2026-05-20 host update (USB_QMP_INPUT_LANE_FIX_V1)
- QMP socket blocker in host launcher fixed.
- `dev.sh` no longer binds fixed `/tmp/sexos-qmp.sock` when QMP is enabled.
- Per-run socket now used (`/tmp/sexos-qmp-${USER:-sexos}-$$.sock`) with pre/post cleanup and trap cleanup.
- `scripts/qemu_harness.sh` supports `--qmp PATH` and forwards `SEXOS_QMP_SOCK` + `SEXOS_QEMU_QMP=1`.
- `scripts/qmp_input_probe.py` now accepts socket from argv or `SEXOS_QMP_SOCK`.

### Verified host behavior
- QMP bind permission error for fixed `/tmp/sexos-qmp.sock` is no longer on active path.
- Harness run with explicit per-run socket showed `QMP monitor: /tmp/sexos-qmp-test-<pid>.sock`.
- Socket cleanup verified after exit.

### Next proof command
```bash
sock="/tmp/sexos-qmp-${USER:-sexos}-$$.sock"
log="/tmp/usb_hid_real_report_proof_v1_qmp.log"

SEXUSB_QEMU_DEVICE=mouse \
SEXOS_QMP_SOCK="$sock" \
./scripts/qemu_harness.sh --timeout 30 --qmp "$sock" > "$log" 2>&1 &
qpid=$!

for _ in $(seq 1 40); do [ -S "$sock" ] && break; sleep 0.25; done
SEXOS_QMP_SOCK="$sock" ./scripts/qmp_input_probe.py "$sock" w a s d || true

wait "$qpid" || true
rg -n "sexusb\.hid\.report\.nonzero|sexusb\.hid\.report\.timeout|sexusb\.route\.sexinput\.ready" "$log"
```

## 2026-05-20 route audit update (USB_HID_QEMU_DEVICE_ROUTE_AUDIT_V1)
- Added infra visibility and timing controls:
  - `scripts/qemu_harness.sh`: explicit resolved USB arg + QMP arg in output.
  - `scripts/qmp_input_probe.py`: `--delay SECONDS` support.
- Ran two lanes with delayed injection (`--delay 8`):
  - `SEXUSB_QEMU_DEVICE=mouse` + QMP mouse events
  - `SEXUSB_QEMU_DEVICE=tablet` + QMP mouse events
- In both lanes:
  - QMP greeting/capabilities/input-send-event all succeeded.
  - SexOS boot + xHCI markers and `sexusb.route.sexinput.ready` appeared.
  - No `sexusb.hid.report.nonzero`; repeated `sexusb.hid.report.timeout` observed.
- Classification for this route-audit run: **SKIP** (QMP accepted, but no proof of real HID report delivery to SexOS endpoint path).

## 2026-05-20 physical operator lane (USB_PHYSICAL_HID_OPERATOR_PROOF_V1)
- Added operator-required proof doc: `docs/handoff/USB_PHYSICAL_HID_OPERATOR_PROOF_V1.md`.
- Added host-only runner: `scripts/usb_physical_hid_operator_probe.sh`.
- Runner behavior:
  - defaults to `mouse` mode, supports `tablet`
  - uses GTK display + xHCI via `qemu_harness.sh`
  - logs to `/tmp/usb_physical_hid_operator_probe.log`
  - asks operator to move/click mouse for at least 10 seconds
  - classifies:
    - PASS -> `sexusb.hid.report.nonzero` and no faults (exit 0)
    - SKIP -> no nonzero and no faults (exit 2)
    - FAIL -> build/harness error or fault markers (exit 1)
