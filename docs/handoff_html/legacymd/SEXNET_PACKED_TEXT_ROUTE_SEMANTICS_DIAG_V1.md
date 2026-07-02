# SEXNET_PACKED_TEXT_ROUTE_SEMANTICS_DIAG_V1

## Outcome
STOP

## Route semantics found
C) `pdx_call` return convention blocks synchronous value transfer on this lane.

## Evidence
1. `pdx_call` userspace ABI returns `(status, value)` from syscall 0.
2. Kernel syscall dispatch for non-slot-0 calls enters `safe_pdx_call(slot, num, arg0, arg1, arg2)`.
3. For `CapabilityData::Domain`, kernel resolves edge as `AsyncEnqueue` and `traverse_edge` returns `Ok(0)` on enqueue success.
4. Therefore userland receives `status=0,value=0` for successful send, not remote function return.
5. `sexnet` side uses `pdx_listen_raw(0)` + `pdx_reply(caller_pd, result)` (async reply model).

## Why packed-text len was zero
Kaleidoscope expected synchronous `ret1=value` from `pdx_call`, but this lane is async enqueue; returned value is enqueue success (`0`), so `len` observed as 0 even when send succeeded.

## Implication
Under current kernel+sex-pdx semantics and constraints (no kernel/sex-pdx edits, no `pdx_call_sync`), direct scalar request/response via immediate `pdx_call` return is not achievable on `SLOT_NET` domain route.

## Safe next step
If continuing under same constraints, switch Kaleidoscope flow to async request + `pdx_listen_raw(0)` reply collection with bounded waits and markers, or explicitly allow kernel/ABI change for sync return semantics.
