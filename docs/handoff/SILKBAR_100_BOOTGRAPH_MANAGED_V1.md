# SILKBAR_100_BOOTGRAPH_MANAGED_V1

## Scope
- `servers/silkbar/src/main.rs`
- `crates/silkbar-model/src/lib.rs` (verified existing contract gates)

## Startup liveness fix
- SilkBar now logs `[silkbar.boot.begin]` and `[bootgraph.silkbar.spawned]` at start.
- Contract is checked before update emission.
- On pass: emits `[silkbar.boot.contract.ok]` and `[bootgraph.silkbar.contract_ready]`.
- On failure: emits `[silkbar.boot.contract.fail] code=C` and degrades safely (`[bootgraph.silkbar.degraded]`) without panic/spin crash.
- Initial workspace/chip updates are deferred into a bounded local queue; no pre-loop update sends.
- Deferred init queue marker: `[silkbar.boot.init.defer] count=N`.

## Deferred init behavior
- At most one deferred init update is flushed per loop iteration.
- Flush markers:
  - `[silkbar.boot.init.flush] idx=I remaining=N`
  - `[silkbar.boot.init.flush.done]`

## BootGraph readiness markers
- Added local readiness markers in order:
  - `[bootgraph.silkbar.spawned]`
  - `[bootgraph.silkbar.contract_ready]` (non-degraded)
  - `[bootgraph.silkbar.loop_ready]`
  - `[bootgraph.silkbar.clock_ready]` (non-degraded)
- Degraded path emits `[bootgraph.silkbar.degraded] reason=...`.

## Security/liveness invariants
- SilkBar remains aggregation/status producer only.
- No framebuffer writes are introduced.
- No shell focus/session ownership changes introduced.
- Upstream index bounds now reject invalid values with:
  - `[silkbar.update.reject] kind=K idx=I reason=out_of_bounds`

## Clock behavior
- No init `SetClock(ss=0)` send.
- `get_ticks()` remains diagnostic-only.
- Clock send marker now includes result:
  - `[silkbar.clock.send] hh=H mm=M ss=S status=R`
