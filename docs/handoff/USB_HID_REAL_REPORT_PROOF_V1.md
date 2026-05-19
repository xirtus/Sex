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
