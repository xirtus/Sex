# BOOTGRAPH_INIT_MARKERS_V1

Date: 2026-05-13

## Goal
Resolve `MASTER_RUNTIME_GATE` `RED_MASTER` caused by `BOOTGRAPH_GATE` missing init markers for:
- sexdrive
- linen
- sexstore
- sexbell
- spindle

## Scope and Rules Followed
- Marker-only patch.
- No kernel edits.
- No sex-pdx/ABI edits.
- No parser changes.
- No startup ordering changes.
- No sleeps/waits/IPC/capability changes.
- No behavior changes.

## Files Changed
- `apps/sexdrive/src/main.rs`
- `servers/linen/src/main.rs`
- `servers/sexstore/src/main.rs`
- `servers/sexbell/src/main.rs`
- `apps/spindle/src/main.rs`

## Marker Additions
- `apps/sexdrive/src/main.rs`
  - Added `[sexdrive.init.start]` at `_start()` entry.
  - Added `[sexdrive.ready]` immediately before steady-state `loop`.

- `servers/linen/src/main.rs`
  - Added `[linen.init.start]` at `_start()` entry.
  - Kept existing `[linen.ready]` in place.

- `servers/sexstore/src/main.rs`
  - Added `[sexstore.init.start]` at `_start()` entry.
  - Added `[sexstore.ready]` immediately before steady-state `loop`.

- `servers/sexbell/src/main.rs`
  - Added `[sexbell.init.start]` at `_start()` entry.
  - Added `[sexbell.ready]` immediately before steady-state `loop`.

- `apps/spindle/src/main.rs`
  - Added `[spindle.init.start]` at `_start()` entry.
  - Kept existing `[spindle.ready]` in place.

## Verification Commands
```bash
./scripts/master_runtime_gate.sh --probe 30 --keep-log
scripts/check_bootgraph_log.py .gate_master/serial.log --allow-fault
```

## Verification Results

### master_runtime_gate
- `BUILD_GATE PASS`
- `SPAWN_GATE PASS`
- `FAULT_GATE PASS`
- `BOOTGRAPH_GATE PASS`
- `CAP_GRANT_GATE PASS`
- `ORDER_GATE PASS`
- `CLOCK_GATE PASS`
- `SCHED_GATE PASS (ADVISORY)`
- `SEXFILES_GATE PASS`
- `FINAL_SCORE GREEN_MASTER`

### check_bootgraph_log
- `BOOTGRAPH PASS`
- `BOOTGRAPH_GATE: PASS`
- `CAP_GRANT_GATE: PASS`
- `ORDER_GATE: PASS`
- `CLOCK_GATE: PASS`
- `FAULT_GATE: PASS`

PD rows now show `OK` for all expected components, including:
- `sexdrive`
- `linen`
- `sexstore`
- `sexbell`
- `spindle`

## Outcome
Success criteria met:
- `BOOTGRAPH_GATE PASS`
- `FINAL_SCORE GREEN_MASTER`
- fault count 0
- handoff written
