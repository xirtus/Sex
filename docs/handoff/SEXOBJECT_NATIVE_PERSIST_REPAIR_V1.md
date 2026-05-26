# SEXOBJECT_NATIVE_PERSIST_REPAIR_V1

## Current classification
Previous blocker class was: `init.start` observed without `init.ready` during Linen native persist lane.

Current run (`/tmp/sexfiles_ready_blocker_v1.log`) shows readiness and route completion:
- `[sexfiles.init.ready] slot=1 ok=1`
- `[sexfiles.trampoline.loop.enter] ok=1`
- `[sexfiles.route.dispatch] op=0x40 ok=1 caller=7`
- `[sexfiles.route.reply] op=0x40 status=S ok=1 caller=7 object_id=1`
- Linen receives success reply and emits done markers.

## Root cause (current understanding)
Startup readiness was previously under-instrumented for precise classification. With added startup markers in code, the 0x40 route now proves end-to-end once SexFiles reaches loop state.

## Marker contract
SexFiles:
- `[sexfiles.init.start] ok=1`
- `[sexfiles.init.before_trampoline] ok=1`
- `[sexfiles.trampoline.enter] ok=1`
- `[sexfiles.trampoline.before_register] slot=1 ok=1`
- `[sexfiles.init.ready] slot=1 ok=1`
- `[sexfiles.trampoline.loop.enter] ok=1`
- `[sexfiles.route.dispatch] op=0x40 ok=1 ...`
- `[sexfiles.route.reply] op=0x40 status=S ok=1 ...` (or `status=E ok=0`)

Linen (0x40 native proof path):
- `[linen.sexobject.native.call] slot=1 op=0x40 ok=1`
- `[linen.sexobject.native.reply.wait] attempts=N ok=0|1`
- `[linen.sexobject.native.reply] status=S ok=1`
- `[linen.sexobject.native.timeout] attempts=N ok=0` (bounded failure path)

Existing success markers preserved for both sides (`sexfiles.sexobject.*`, `linen.sexobject.native.*`).

## Gate behavior update
`linen_sexobject_native_persist` classification includes startup stage detail:
- `sexfiles_started=0|1`
- `sexfiles_before_trampoline=0|1`
- `sexfiles_trampoline_enter=0|1`
- `sexfiles_ready=0|1`
- `dispatch_op40=0|1`

Explicit FAIL modes include:
- `sexfiles_not_ready`
- `no_dispatch_op40`
- `dispatch_but_no_create_marker`
- `linen_no_reply`
- `read mismatch`

PASS requires:
- Linen call+reply success markers
- SexFiles dispatch/reply success for op 0x40
- SexFiles create/write/read/done markers
- Linen create/write/read/done markers

## Proof commands
```bash
./scripts/entrypoint_build.sh

SEXOS_PROOF_QMP=0 DAILY_DRIVER_PROBE_SECONDS=240 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexfiles_ready_blocker_v1.log

./scripts/daily_driver_master_gate.sh /tmp/sexfiles_ready_blocker_v1.log

rg -a -n "sexfiles.init|sexfiles.trampoline|sexfiles.route|sexfiles.sexobject|linen.sexobject|sexobject_write_read_persist|linen_sexobject_native_persist|FINAL:|#PF|#GP|panic|fault.kill" \
  /tmp/sexfiles_ready_blocker_v1.log | tail -500
```

## Current-tier limitation
- This run still fails global daily gate on unrelated `clock_visible_seconds` lane.
- Early startup markers (`init.start`, `init.before_trampoline`, `trampoline.enter`, `trampoline.before_register`) did not appear in the captured serial log, while later startup markers did. Treat this as remaining observability gap in earliest startup window.

## Do-not-regress
- Do not change kernel/ABI/`crates/sex-pdx` for this lane.
- Do not alter scheduler/display/clock/shell lanes.
- Keep bounded retry only for Linen opcode `0x40` proof path.
- Preserve existing DiskFS/RamFS proof behavior outside native 0x40 observability/sequencing markers.

## SEXFILES_READY_BLOCKER_V1 status (2026-05-26)
Current run classification moved from "init.start without init.ready" to "ready reached, route not yet exercised in this boot window":
- observed: `[sexfiles.init.start] ok=1`
- observed: `[sexfiles.init.before_trampoline] ok=1`
- observed: `[sexfiles.trampoline.enter] ok=1`
- observed: `[sexfiles.init.ready] slot=1 ok=1`
- observed: `[sexfiles.trampoline.loop.enter] ok=1`
- not observed in this run log: `sexfiles.route.dispatch op=0x40`, `sexfiles.sexobject.*`, `linen.sexobject.*`

Run blocker in this attempt was environmental:
- QEMU failed to acquire write lock for `.gate_master/nvme.img`:
  `Failed to get "write" lock ... Is another process using the image`
- this prevented a clean Linen/SexFiles route exercise for op `0x40` in the same pass.

Gate instrumentation now reports startup stage bits in `linen_sexobject_native_persist` row detail:
- `sexfiles_started=0|1`
- `sexfiles_before_trampoline=0|1`
- `sexfiles_trampoline_enter=0|1`
- `sexfiles_ready=0|1`
- `dispatch_op40=0|1`
