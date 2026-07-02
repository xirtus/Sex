# PERF_LOG_NOISE_ABLATION_V1 — Serial Spam Budgeted, Redraw Cadence Recovered

Date: 2026-07-02
Result: **PASS** — noise down 95.8% total (>99.9% per family), **recv_to_draw 2.91 → 1.10**.

## Root cause confirmed

Serial spam WAS the display redraw bottleneck. Same stimulus, same code paths,
only logging gated:

| Metric | Before | After |
|--------|--------|-------|
| total log lines | 158283 | 6651 |
| boot_frame.alloc lines | 67790 | 4 (+14 summaries) |
| linen.session.reject lines | 61269 | 4 (+summaries) |
| scheduler.yield lines | 22965 | 4 (+summaries) |
| **recv_to_draw** | **2.91 BAD** | **1.10 OK** |
| display recv/draw/present | 32/11/11 | 32/29/29 |
| send_to_recv | 1.03 | 1.03 |
| draw_to_present | 1.00 | 1.00 |
| tick chains | 3 | 5 |
| faults | 0 | 0 |

Serial writes in the yield/reject/alloc hot paths stalled PDs long enough that
sexdisplay coalesced ~3 cursor recvs per draw. With logging budgeted, draws
run near-lockstep with recvs (32 recv → 29 draws).

## Marker classification

| Marker | Site | Class | Frequency source | Treatment |
|--------|------|-------|------------------|-----------|
| `[kernel.mem.boot_frame.alloc]` | `kernel/src/memory/manager.rs` allocate_frame | kernel, AP1 diagnostic, noise (overlap detection lives in `diag_record_boot_frame`, still runs) | one line per boot page-table frame (~67790) | first 4 + power-of-two summary; `self.next` reused as counter |
| `[linen.session.reject] reason=bad_name_len` | `servers/linen/src/main.rs` handle_create_object | server, noise — **spindle (pd=12) hammers create with len=0 continuously** (65536+ per run) | every rejected create call | first 4 + power-of-two summary via `LINEN_REJECT_BAD_NAME_LEN_COUNT` AtomicU64; reject reply still sent every call |
| `scheduler.yield_and_switch.saved` | `kernel/src/scheduler.rs:565` | kernel, log-only, noise | every yield syscall (~22965) | first 4 + power-of-two summary via `SCHED_YIELD_SAVED_COUNT`; follows existing `SCHED_*_LOG_BUDGET` idiom |

All three: logging gated ONLY — allocation, scheduling, reject-reply behavior
untouched. No kernel behavior edit → no STOP FIRST needed. Other 4 linen
reject reasons (bad_kind, wire_name_len, create_failed, get_failed) left
per-event: rare, diagnostic value high.

Summary format (power-of-two counts 8,16,32…, no alloc, atomics only):
```
[perf.noise.summary] name=<family> count=N suppressed=N-4
```

## Files changed

- `kernel/src/scheduler.rs` — SCHED_YIELD_SAVED_COUNT static + gate at yield save.
- `kernel/src/memory/manager.rs` — boot_frame.alloc print gated on self.next.
- `servers/linen/src/main.rs` — AtomicU64 import, counter static, bad_name_len gate.
- `scripts/perf_bisection_gate.sh` — logvolume now parses `[perf.noise.summary]`
  for true event counts (lower bound = last power-of-two summary; falls back to
  raw line count on pre-ablation logs — bisect metrics stay comparable).
- `docs/handoff/PERF_LOG_NOISE_ABLATION_V1.md` — this file.

## Proof (2026-07-02)

- Build: `./scripts/entrypoint_build.sh` PASS.
- Runtime: gate_0_2 QEMU lane (q35, nec-usb-xhci, usb-tablet, headless,
  `-display none`), enum.done-synced injection (+3s), keyboard `a` + pointer
  sweep + drag (runner: `/tmp/pln_v1/run.sh` pattern — see
  INPUT_PRESENT_TICK_TRACE_V1 recurring issues for why not fixed-sleep).
- `input_current_tier_gate.sh`: **PASS** exit 0.
- `input_control_quality_gate.sh`: **PASS** exit 0, chains=5.
- `perf_bisection_gate.sh`: exit 1 — recv_to_draw/send_to_recv/draw_to_present
  all GOOD now; only remaining BAD flag is `input_to_present(12>4)`, which is
  a **measurement artifact, not latency** (see below).
- Fault scan: pf=0 gp=0 panic=0 fault_kill=0 reboot_loop=0 freeze=0 storm=0.

## Known artifact: max_total_logical=12 at seq=1

DISPLAY_DRAW_TICK counts boot-time cursor draws (untagged sends) before the
first USB apply, so seq=1 sees draw_tick=13 vs recv_tick=2 → +11 counter-domain
offset, not queue latency. Later chains: 2, −8, −11, −1. Fix candidates for a
follow-up mission: baseline-offset the first chain in the gate parser, or skip
seq=1. Until then a perf-gate BAD showing ONLY `input_to_present` with clean
ratios should be re-read against tick samples before trusting.

## Remaining bottleneck / next targets

1. **Spindle → linen create spam**: pd=12 issues 65536+ bad create calls
   (len=0) per run — pure CPU/PDX waste even with logging gated. Fix spindle's
   create loop (why len=0? why retry forever?). Likely next perf win.
2. Perf gate `input_to_present` threshold needs boot-offset calibration
   (artifact above).
3. recv_to_draw 1.10 is near-lockstep — display redraw cadence no longer the
   dominant bottleneck at light stimulus. Re-run heavy stimulus before
   declaring Chapter 2 smoothness done.

## Recurring issues

1. Noisy proof markers: prefer first-4 + power-of-two summary pattern
   (`[perf.noise.summary]`) from day one — serial volume measurably throttles
   PD scheduling and display cadence under QEMU `-serial file:`.
2. True event counts from summaries are lower bounds (last power of two).
3. QMP socket path >108 bytes fails — short dirs (`/tmp/pln_v1`).
