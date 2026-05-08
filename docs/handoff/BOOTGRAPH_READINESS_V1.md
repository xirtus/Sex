# BOOTGRAPH_READINESS_V1

## BootGraph gate command

```bash
scripts/check_bootgraph_log.py /tmp/sexos.log
```

Integrated runtime gate path:

```bash
./scripts/master_runtime_gate.sh --probe 25 --keep-log
```

The runtime gate captures serial output and then runs `scripts/check_bootgraph_log.py <serial_log> --allow-fault` as the host-side BootGraph checker.

## Required pass output

For parser-only run:

- `BOOTGRAPH PASS`

For integrated runtime gate:

- `BOOTGRAPH_GATE: PASS`
- `CAP_GRANT_GATE: PASS`
- `ORDER_GATE: PASS`
- `CLOCK_GATE: PASS`
- `FINAL_SCORE: GREEN_MASTER` for full pass.

## Common failures

- `BOOTGRAPH FAIL: BOOTGRAPH_GATE ... missing ...ready`
  - A required `*.ready` marker is missing in the log, or appears before `*.init.start`.
- `BOOTGRAPH FAIL: CAP_GRANT_GATE ...`
  - Missing `[bootgraph.phase25.begin]` / `[bootgraph.phase25.complete]` or invalid ordering, or missing required A2 grant markers.
- `BOOTGRAPH FAIL: ORDER_GATE ...`
  - Sender `bootgraph.edge.send` appears before receiver `*.ready` or before `phase25.complete`.
- `CLOCK_GATE: PASS WARN: ...`
  - Clock chain is partially degraded (for example: send without recv, recv without redraw, repeated clock drops, or fb_live wait without live render).
- `FAULT_GATE: FAIL ...`
  - Fault patterns (`panic`, `fault.kill`, `#PF`, `#GP`) found in serial log (unless explicitly allowed).

## BootGraph V1 Kernel Markers

The kernel emits marker-only observability logs in `kernel/src/init.rs`:

- `[bootgraph.pd.spawn.begin] pd=<name>`
- `[bootgraph.pd.spawn.ok] pd=<name> id=<id> pkey=<pkey>`
- `[bootgraph.pd.spawn.err] pd=<name> reason=<reason>`
- `[bootgraph.phase25.begin]`
- `[bootgraph.cap.grant] from=kernel to=<pd> slot=<slot> target=<target> ok=1`
- `[bootgraph.cap.grant] from=kernel to=<pd> slot=<slot> target=<target> ok=0 optional=1 reason=<reason>`
- `[bootgraph.phase25.complete]`
- `[bootgraph.boot.handoff] target=<pd> id=<id> entry=<entry_addr>`

Proof command:

```bash
rg "bootgraph.pd.spawn|bootgraph.phase25|bootgraph.cap.grant|bootgraph.boot.handoff|fault.kill|#PF|#GP|panic" /tmp/sexos.log
```

## Clock Canary Markers

Added marker names for clock-freeze layer classification:

- `[silkbar.loop.cadence.start] iter=N`
- `[silkbar.loop.cadence.done] iter=N`
- `[sexdisplay.fb.live.wait] iter=N` (hard-budgeted)

Existing related marker names in this checkout:

- `[silkbar.clock.send]`
- `[silkbar.send_update.drop.clock]`
- `[sexdisplay.clock.recv]`
- `[sexdisplay.clock.redraw]`
- `[sexdisplay.render.live.ok]`

Canonical CLOCK_GATE chain (real PASS):

- `[silkbar.clock.send]` count >= 1
- `[sexdisplay.clock.recv]` count >= 1
- `[sexdisplay.clock.redraw]` count >= 1
- `[sexdisplay.render.live.ok]` count >= 1

Boot canary semantics:

- `[silkbar.clock.boot_canary] send=N threshold=T` proves early-boot accelerated cadence.
- Boot canary marker is advisory and not required forever once steady cadence is active.

Tick-based markers:

- Tick-indexed markers remain optional/advisory.
- Parser no longer emits stale warning solely because tick-indexed markers are absent when canonical chain passes.

## V2 Soft Barrier Marker Contract

First rollout edge:

- `silkbar -> sexdisplay` (`slot=5`, `SLOT_DISPLAY`, `op=OP_SILKBAR_UPDATE`)

Soft-barrier defer marker:

- `[bootgraph.edge.defer from=silkbar to=sexdisplay slot=5 reason=missing_cap]`

Rules:

- No separate probe call is allowed; first `pdx_call_checked` remains the real send attempt.
- Emit at most one defer marker per boot per edge/slot.
- Defer before `phase25.complete` is informational/pass.
- Defer after `phase25.complete` is warning.
- Defer followed by normal `bootgraph.edge.send` is pass recovery.

Note: earlier "missing handoff path" note is superseded; `AGENT_HANDOFF_GP_CLOCK.md` may live under `docs/legacy/` in this checkout.
