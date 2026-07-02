# INPUT_CHAPTER2_CONTROL_QUALITY_AUDIT_V1

Status: MEASUREMENT_PARTIAL_STOP_FIRST.

## Scope

This pass measured the existing Chapter 1 proof log and added a measurement-only gate:

```text
scripts/input_control_quality_gate.sh
```

No Rust behavior was changed. No kernel, sex-pdx ABI, sexdisplay renderer, sexusb cadence, shell policy, or input normalizer behavior was changed.

Backup made before audit work:

```text
/tmp/input_chapter2_control_quality_start.diff
```

## Measurement Result

Input is real, but product-quality input is not proven.

The latest real QEMU proof log still passes the Chapter 1 chain gate:

```text
scripts/input_current_tier_gate.sh logs/qemu-latest.log
INPUT_100_CURRENT_TIER_V1: PASS
```

The Chapter 2 quality audit gate reports:

```text
scripts/input_control_quality_gate.sh logs/qemu-latest.log
INPUT_CHAPTER2_CONTROL_QUALITY_AUDIT_V1: MEASUREMENT_PARTIAL_STOP_FIRST
```

Quality table from `logs/qemu-latest.log`:

| metric | value |
| --- | ---: |
| sexusb transfer events | 32 |
| sexusb re-arms | 32 |
| sexinput pointer receives | 238 |
| HID pointer emits | 231 |
| shell pointer applies | 64 |
| pointer move ok markers | 8 |
| shell cursor sends | 64 |
| sexdisplay cursor surface updates | 4 |
| sexdisplay cursor draws | 8 |
| drag begin/move/end | 1 / 8 / 1 |
| max drag move line gap | 63 |
| max drag dx/dy | 18 / 5 |
| drag jumps > 24 px | 0 |
| button down/up | 2 / 2 |
| duplicate down / lost release / stuck down | 0 / 0 / 0 |
| keyboard down/up | 1 / 1 |
| shell apply max line gap | 145 |
| cursor update max line gap | 6840 |
| cursor draw max line gap | 6336 |
| fault scan | clean |

## Honest Limits

Several markers are budgeted, so the audit cannot compute exact drop/coalesce rate from the success markers alone:

```text
shell_apply_budget_hit=1
pointer_move_budget_hit=1
cursor_send_budget_hit=1
cursor_draw_budget_hit=0
```

The log proves visible cursor update/draw markers exist, but it does not prove true input-to-pixel latency. Current markers do not carry a shared event sequence or tick from:

```text
USB/HID receive -> shell apply -> cursor surface update -> sexdisplay cursor draw
```

Exact missing markers:

```text
[input.frame.apply] seq=N tick=N
[pointer.present.tick] seq=N tick=N
[input.latency.sample] seq=N recv_tick=N shell_tick=N present_tick=N
```

The current `OP_SURFACE_UPDATE` cursor path carries only `surface_id`, `x`, and `y`; it cannot carry a per-input sequence without an ABI change. A trace-only next pass can still correlate by serial order and coordinates, but that must be labeled as a trace approximation unless a future ABI-safe sequence channel exists.

## First Bottleneck

First bottleneck for Chapter 2 is display-present traceability, not USB transfer cadence.

Observed:

- xHCI transfer/rearm still works.
- Shell apply and cursor send markers hit their bounded budgets.
- Button and drag pairing is clean.
- Display cursor update/draw exists, but is sparse and not tied to input sequence.
- Keyboard passed in this QMP log, but repeat/interactive SDL quality is not measured.

## Proof

Build:

```text
./scripts/entrypoint_build.sh
[SEXOS ENTRYPOINT] success
```

Known host warning remains:

```text
cargo check -p sex-pdx failed in current env
target path `/home/xirtus_arch/x86_64-sex.json` is not a valid file
```

Gate checks:

```text
bash -n scripts/input_control_quality_gate.sh
bash scripts/input_control_quality_gate.sh logs/qemu-latest.log
scripts/input_current_tier_gate.sh logs/qemu-latest.log
```

Fault scan:

```text
[input.faultscan.ok] pf=0 gp=0 panic=0 fault_kill=0 reboot_loop=0 freeze=0 storm=0
```

## Next Smallest Prompt

```text
MISSION: INPUT_CHAPTER2_PRESENT_TRACE_V1 — ADD TRACE-ONLY INPUT-TO-VISIBLE MARKERS.

Goal:
Add bounded trace markers only. Do not optimize smoothing/cadence yet.

Allowed edits:
- servers/silk-shell/src/main.rs
- servers/sexdisplay/src/main.rs
- scripts/input_control_quality_gate.sh
- docs/handoff/INPUT_CHAPTER2_PRESENT_TRACE_V1.md

Do not:
- change kernel
- change sex-pdx ABI
- change OP_SURFACE_UPDATE arguments
- change renderer behavior
- change cursor policy
- change input smoothing

Trace requirements:
- shell emits [input.frame.apply] seq=N x=N y=N tick=N when a real USB pointer event sends cursor update
- sexdisplay emits [pointer.present.tick] x=N y=N tick=N draw=N on cursor surface update/draw
- gate correlates by serial order and x/y, and labels it approximate unless a shared seq reaches display
- report [input.latency.sample] only if correlation is defensible

Stop first if true shared seq requires sex-pdx ABI change.
```
