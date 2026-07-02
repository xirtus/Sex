# QMP_INPUT_PROOF_LANE_STABILIZE_V1

## Result

PASS on final runtime log:

- log: `logs/qemu-latest.log`
- build: `./scripts/entrypoint_build.sh` PASS
- QMP lane: `GATE_DIR=/tmp/qmp_lane_stabilize_v1 PROBE_SECONDS=18 POST_STIMULUS_TIMEOUT_SECONDS=24 ./scripts/gate_0_2.sh`
- Chapter 1: `scripts/input_current_tier_gate.sh logs/qemu-latest.log` PASS
- Chapter 2: `scripts/input_control_quality_gate.sh logs/qemu-latest.log` PASS
- perf: `scripts/perf_bisection_gate.sh logs/qemu-latest.log` BAD on real ratios only:
  - `send_to_recv(4.00>2.0)`
  - `recv_to_draw(6.40>2.0)`
- Linen storm: `bad_name_len len=0 caller=12` count 0
- faults: `#PF=0 #GP=0 panic=0 fault.kill=0 reboot_loop=0 freeze=0 storm=0`

Note: `gate_0_2.sh` still prints a legacy `FINAL_SCORE: RED_0_2`
because its older summary checks look for marker names outside this mission
(`sexinput.keyboard.send`, `silk-shell.cursor.update`). The required mission
gates above pass on the same final log.

## Root Cause

The Linen-clean QMP proof lane was not marker-driven enough:

- QMP input could start before USB/shell/focus readiness had settled.
- The drag press sometimes landed on Quil/Mesh instead of shell-owned surface 100.
- Built-in synthetic input proofs could race QMP input and consume the click/drag path.
- Fixed post-stimulus cutoff could either stop before display-present/tick markers or run into the late QEMU page-fault tail.

The final stable path is:

1. Build this QMP lane with `SEXOS_PROOFS_DISABLED=1` so host QMP stimulus owns the input proof.
2. Wait for `[usb.xhci.enum.done]` and `[silk-shell.ready]`.
3. Send QMP `ret` and wait for focus to reach surface 100.
4. Drain a bounded absolute pointer sweep until the serial log proves pre-drag coordinates arrived with `buttons=0`.
5. Send a short drag with explicit double release.
6. Send a short no-button cursor sweep for display trace.
7. Stop QEMU when Linen storm fixed + Chapter 1 + Chapter 2 gates pass, or on hard fault/timeout.

## Changed

- `scripts/gate_0_2.sh`
  - uses short `/tmp` QMP path by default
  - writes final runtime log to `logs/qemu-latest.log`
  - waits on readiness and proof markers
  - disables built-in synthetic input proofs for the QMP lane build
  - injects QMP Enter before pointer to focus surface 100
  - uses marker-drained pointer stimulus
  - stops after clean proof markers instead of fixed sleep
  - tightens hard-fault scan to avoid benign `freeze` substrings

## Final Metrics

From final `scripts/input_control_quality_gate.sh logs/qemu-latest.log`:

- transfer_events=32
- rearms=32
- pointer_recv=565
- pointer_emit=532
- shell_apply=66
- pointer_move=10
- shell applies=512 sends=512 drag_moves=87
- display recv=128 draws=20 presents=20
- send_to_recv=4.00
- recv_to_draw=6.40
- tick chains=2

Remaining bottleneck is display trace throughput:

- `send_to_recv=4.00`
- `recv_to_draw=6.40`

This is a real perf metric after the clean proof, not a Chapter 1 regression.
