# PDX_RING_OVERFLOW_DIAGNOSTIC_V1

**Date:** 2026-05-03
**Status:** COMPLETE (refinement of existing diagnostics)

## Context

CLOCK_FREEZE_FALLBACK_GATE_V1 discovered that SilkBar update messages (SetClock) are silently lost under MPSC load — the kernel's SPSC `RingBuffer<MessageType, 256>` drops messages when multiple producers (shell + silkbar) enqueue concurrently without atomic CAS.

M2 audit item F1 tracked this as a reliability hole: non-clock SilkBar updates (workspace switch, chip visibility, urgent, chip kind) are also subject to silent drops. The stale-clock bug was fixed by the fallback gate, but other update types still need sender-side drop observability.

## What Existed Before

- `send_update()` in silkbar called `pdx_call_checked()` but discarded the return value
- No sender-side drop logging
- No way to know if a workspace/chip/urgent update silently failed
- Receiver-side: `[silkde.m2.assert.bad]` fires when `apply_update()` returns `false` (invalid kind/bounds)

## What Changed

### Phase 1 (CLOCK_FREEZE_FALLBACK_GATE_V1 — already merged)

Added `[silkbar.send_update.drop]` bounded diagnostic (budget 16) in `send_update()`:

```
[silkbar.send_update.drop] kind=N count=N
```

### Phase 2 (this handoff — refined marker)

Added `index` and `err` code to the drop marker for precision:

```
[silkbar.send_update.drop] kind=4 idx=0 err=0xfffffffffffffffe count=7
```

- `kind`: UpdateKind discriminant (0=workspace, 2=chip, 3=chipkind, 4=clock, etc.)
- `idx`: slot index (workspace index, chip index, or 0 for clock)
- `err`: PDX error code — `0xFFFF_FFFF_FFFF_FFFE` = ring full (`ERR_SERVICE_NOT_READY`), `0xFFFF_FFFF_FFFF_FFFC` = cap invalid (`ERR_CAP_INVALID`)
- `count`: sequential drop counter (wrapping u64)

## Current Diagnostic Coverage

| Layer | Marker | Budget | When |
|-------|--------|--------|------|
| Sender (silkbar) | `[silkbar.send_update.drop]` | 16 | AsyncEnqueue returns error |
| Receiver (sexdisplay) | `[silkde.m2.assert.bad]` | unbudgeted | `apply_update()` returns `false` |

## Known Limits (design handoff for future)

1. **Sender-side only.** The drop marker fires on the silkbar side when `pdx_call_checked` returns `Err`. It does NOT detect messages that were enqueued but silently lost in the SPSC ring under MPSC collision (the enqueue returns Ok but the write is overwritten by another producer). These are invisible to both sender and receiver.

2. **No ACK protocol.** Receiver does not acknowledge updates. Silkbar cannot distinguish "delivered" from "dropped" without a round-trip ACK, which would require PDX ABI changes and a new IPC opcode.

3. **No retry.** Drops are logged but not retried. A retry loop could fix transient ring-full drops but would add complexity and could livelock under sustained load.

4. **SPSC ring is the root cause.** The kernel `RingBuffer` uses load/store without CAS. Under MPSC load (shell + silkbar both sending to sexdisplay), the `write_idx` increment is racy. Fix requires either: (a) atomic CAS for write_idx, (b) per-producer slots, or (c) a mpsc queue.

5. **Drop count does not indicate severity.** A single drop under transient load is harmless. A growing counter indicates sustained ring pressure that may require queue sizing or rate limiting.

## Files Changed

- `servers/silkbar/src/main.rs` — refined `[silkbar.send_update.drop]` marker with `idx` and `err` fields

## Verification

```bash
# Build
./scripts/entrypoint_build.sh

# Run (12s nographic)
SEXUSB_XHCI_TRACE=0 timeout 12 ./dev.sh run-nographic \
  2>/tmp/pdx-ring-diag.trace | tee /tmp/pdx-ring-diag.log

# Verify
grep -c 'silkbar.send_update.drop' /tmp/pdx-ring-diag.log   # expect 0 in normal boot
grep -cE 'fault|panic' /tmp/pdx-ring-diag.log                # expect 0
```

## Verified Results (2026-05-03)

```
silkbar.send_update.drop: 0 (no drops in normal boot)
fault/panic/PF/GP:         0 (no errors)
All proof markers:        PRESERVED (launcher, status, clock, bell, workspace)
```

## STOP FIRST Conditions

1. Adding ACK/retry protocol — requires PDX ABI change, new IPC opcode, sexdisplay state machine
2. Fixing SPSC ring to MPSC — requires kernel ring buffer change with CAS
3. Adding queue sizing or backpressure — requires kernel/PDX coordination
4. Removing the drop diagnostic to save serial bandwidth
5. Making the diagnostic unbudgeted (spam)
