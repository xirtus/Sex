# USB_QMP_INPUT_LANE_FIX_V1

Status: HOST-QMP PATH FIXED (OS untouched)

## Scope
- Host script and docs only.
- No `kernel/*`, `servers/*`, `crates/*`, `apps/*` edits.

## Old blocker
- Fixed QMP path in `dev.sh`:
  - `-qmp unix:/tmp/sexos-qmp.sock,server=on,wait=off`
- Prior failure:
  - `Failed to bind socket to /tmp/sexos-qmp.sock: Operation not permitted`

## New QMP socket behavior
- `dev.sh` now uses per-run socket path when QMP is enabled:
  - default: `/tmp/sexos-qmp-${USER:-sexos}-$$.sock`
  - override: `SEXOS_QMP_SOCK=/path/to.sock`
- Cleanup:
  - remove stale socket before QEMU launch
  - trap cleanup on `EXIT INT TERM`
  - remove socket after QEMU exits
- `scripts/qemu_harness.sh` now accepts:
  - `--qmp /path/to.sock`
  - exports `SEXOS_QEMU_QMP=1` + `SEXOS_QMP_SOCK`
- `scripts/qmp_input_probe.py` now accepts socket from:
  - positional path arg, or
  - `SEXOS_QMP_SOCK` env var

## Validation run
- Syntax checks:
  - `bash -n scripts/qemu_harness.sh` PASS
  - `python3 -m py_compile scripts/qmp_input_probe.py` PASS
- QMP harness proof attempt:
  - `sock="/tmp/sexos-qmp-test-$$.sock"`
  - `SEXOS_QMP_SOCK="$sock" ./scripts/qemu_harness.sh --timeout 10 --qmp "$sock"`
- Observed:
  - banner includes `QMP sock: /tmp/sexos-qmp-test-<pid>.sock`
  - runtime includes `QMP monitor: /tmp/sexos-qmp-test-<pid>.sock`
  - no bind-permission error for `/tmp/sexos-qmp.sock`
  - post-exit socket cleanup verified (`cleaned`)

## Host-only proof command (QMP lane)
```bash
sock="/tmp/sexos-qmp-${USER:-sexos}-$$.sock"
log="/tmp/usb_hid_real_report_proof_v1_qmp.log"

SEXUSB_QEMU_DEVICE=mouse \
SEXOS_QMP_SOCK="$sock" \
./scripts/qemu_harness.sh --timeout 30 --qmp "$sock" > "$log" 2>&1 &
qpid=$!

# wait for QMP socket publish
for _ in $(seq 1 40); do [ -S "$sock" ] && break; sleep 0.25; done

# inject events if QMP command path is ready
SEXOS_QMP_SOCK="$sock" ./scripts/qmp_input_probe.py "$sock" w a s d || true

wait "$qpid" || true
rg -n "sexusb\.hid\.report\.nonzero|sexusb\.hid\.report\.timeout|sexusb\.route\.sexinput\.ready" "$log"
```

## Physical-operator fallback (if QMP injection path still does not trigger USB nonzero)
```bash
log="/tmp/usb_hid_real_report_proof_v1_manual.log"
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run > "$log" 2>&1
```
- Operator action: move mouse and click repeatedly in QEMU window during first 30s.
- Expected markers in log:
  - `[sexusb.route.sexinput.ready]`
  - `[sexusb.hid.report.nonzero] ... ok=1`
  - optional idle/timeout markers before first motion.

## Stop-first outcome check
- QMP socket path was findable and patchable in allowed files.
- No broad harness rewrite required.
- No OS code changes required.

## 2026-05-20 - QMP_ARG_NOT_PASSED_TO_QEMU_FIX_V1

- Root cause: `dev.sh run-nographic` launched a separate hardcoded QEMU argv that did not include `QMP_ARG`, so `QMP monitor: ...` was printed but no `-qmp unix:...` reached QEMU.
- Fix: added QMP visibility marker in `dev.sh`:
  - `[dev.qmp.arg] sock=PATH ok=1`
- Fix: added `QMP_ARG` to the actual `run-nographic` QEMU invocation by switching to `QEMU_NOGRAPHIC_ARGS` array and launching via `"${QEMU_NOGRAPHIC_ARGS[@]}"`.
- Fix: print-cmd path in `scripts/qemu_harness.sh` now prints the `-qmp unix:...` argument whenever `--qmp`/`SEXOS_QMP_SOCK` is set.
- Marker extraction typo check: verified `"scene.*render" "$LOG_FILE"` form is now correct.

## 2026-05-20 - QMP_HANDSHAKE_TIMEOUT_FIX_V1

- Observed root cause in probe path: `scripts/qmp_input_probe.py` read raw `recv(4096)` blobs and attempted whole-buffer JSON decode, which is fragile for QMP line-delimited multi-message streams and produced greeting/capability timeouts.
- Probe fix in allowed file:
  - switched to robust line-delimited JSON receive with explicit timeout diagnostics
  - explicit greeting wait (`QMP` key required), then `qmp_capabilities`
  - added command/response logging (`[qmp.send]`, `[qmp.recv]`, `[qmp.event]`, `[qmp.summary]`)
  - default mode changed to `mouse` (relative move + left click press/release)
  - preserved keyboard mode via `key`/legacy key-list forms (`... key w a s d` or `... w a s d`)
- Dev-side visibility improvement:
  - `dev.sh` now emits `[dev.qmp.cmd] -qmp unix:PATH,server=on,wait=off` when QMP is enabled.
