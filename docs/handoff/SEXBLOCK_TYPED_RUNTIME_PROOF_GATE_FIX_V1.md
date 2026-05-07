# SEXBLOCK_TYPED_RUNTIME_PROOF_GATE_FIX_V1

## Root cause

`master_runtime_gate.sh` had a numeric parsing bug in `count_marker()`:

- old code: `grep -cE ... || echo 0`
- when zero matches occurred, `grep -cE` printed `0` and returned exit code 1
- `|| echo 0` appended another `0`
- callers received `"0\n0"` and numeric checks like `[ "$count" -ge 1 ]` produced `integer expected`

## Exact fix

File changed: `scripts/master_runtime_gate.sh`

1. Hardened `count_marker()` to always emit a single numeric token.
2. Pre-created `.gate_master/serial.log` before QEMU start so `--keep-log` leaves a deterministic artifact.
3. Added typed-marker stdout emission block when `SEXOS_SEXFILES_REAL_BLOCK_PROOF=1` so the user’s piped grep command can consume script output.

## Final proof command

```bash
SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log \
| grep -E 'sexfiles\.diskfs\.(typed\.(call|reply)|block\.proof\.(typed|bad|unaligned))|sexdrive\.block\.typed|sexblock\.abi'
```

## Result in this host run

- `integer expected` errors: **RESOLVED** (none observed after patch)
- `.gate_master/serial.log` with `--keep-log`: **PRESENT** (`918` lines in run)
- typed markers captured by grep: **MISSING** in this probe
- `ERR_NO_DEVICE` typed status marker: **MISSING** in this probe (no typed route execution observed)

## Non-script blocker (exact)

From gate output and serial log evidence:

- `Spawned PD` markers are present for target PDs.
- Runtime progression stalls at PD1-only `task.running` in probe window.
- `sexfiles.ready` missing and `task.running pd_id=11` missing.
- Therefore typed `sexfiles -> SLOT_BLOCK -> sexdrive` proof path is not executing during this runtime window.

This is not a parsing/capture bug after the script fix; it is a runtime reachability/scheduling/execution-window blocker upstream of typed block proof triggers.

## Safety checks

- No kernel changes
- No sex-pdx ABI changes
- No storage protocol changes
- No fake success path added
- No `#PF/#GP/panic` marker hits in captured log during this run

## Next prompt recommendation

`PD_RUNTIME_REACHABILITY_PROOF_V2`:

- prove why PD11 (sexfiles) and PD3/PD4 do not reach `task.running` in probe window
- keep script fixed; focus on scheduler/runtime reachability only
- stop before any ABI/protocol change
