# Bell Event Detail Seed V1

## Status: PASS
Date: 2026-05-14
Attempts: 2

## Root Cause
The Bell keyboard detail proof (`maybe_run_bell_keyboard_detail_proof`) failed at stage 3
with `event_id=0 ok=0 reason=no_event` because the silk-shell local `BELL_EVENTS` ring
was empty when the proof ran.

The Bell server (`sexbell`) seeds a demo event at boot (`[bell.demo.boot] event_id=1`),
but this event lives in the Bell server's own PD queue. Silk-shell's local `BELL_EVENTS`
ring is a separate in-memory ring buffer populated only through J4/J7 Linen→Quil object
link events via `bell_record_event()` → `bell_emit_object_link_event()`. There is no
sync mechanism between silk-shell's local ring and the Bell server's queue.

No J4 link occurs before the proof runs (proofs run at the very start of the main loop,
before any user-initiated Linen→Quil links). The ring is therefore empty, and
`bell_selected_event_snapshot()` returns `None`, causing the `no_event` reject.

## Fix
Added a new proof gate `SEXOS_BELL_DETAIL_SEED_PROOF=1` with `maybe_run_bell_detail_seed_proof()`
that seeds two synthetic Bell events into the local ring before exercising the full
detail open/close path:

1. `bell_record_event(1000, 1000)` — dummy event, gets event_id=0
2. `bell_record_event(1001, 1001)` — target event, gets event_id=1, newest (row=0)

This ensures `bell_ring_count() > 0` and `bell_selected_event_snapshot()` returns a
valid nonzero event_id. The existing keyboard handlers (`bell_select_next_row`,
`bell_select_prev_row`, `bell_emit_selected_event_detail_proof`, `bell_close_detail`,
`bell_cycle_lane`) are exercised as-is — no changes to their logic.

## Event/Detail Proof Table

| Stage | Action | event_id | ok | Reason |
|-------|--------|----------|----|--------|
| 0 | open_focus | — | 1 | ok |
| 1 | seed_event (×2) | 0,1 | 1 | seeded |
| 2 | next_event | 0→1 | 1 | ok (navigated between events) |
| 3 | prev_event | 1→0 | 1 | ok (wrapped back) |
| 4 | open_detail | 1 | 1 | ok (nonzero event_id) |
| 5 | close_detail | — | 1 | ok |
| 6 | lane_cycle | — | 1 | ok |

## Runtime Proof Counts

```
[bell.detail.seed.proof] stage=0 action=open_focus ok=1 reason=ok
[bell.ring.write] idx=0 event_id=0 object_id=1000 buffer_id=1000
[bell.ring.write] idx=1 event_id=1 object_id=1001 buffer_id=1001
[bell.event.seed.visible] event_id=1 total=2 ok=1
[bell.detail.seed.proof] stage=1 action=seed_event ok=1 reason=seeded
[bell.nav.move] old=0 new=1 total=2
[bell.detail.seed.proof] stage=2 action=next_event ok=1 reason=ok
[bell.nav.move] old=1 new=0 total=2
[bell.detail.seed.proof] stage=3 action=prev_event ok=1 reason=ok
[bell.detail.open] event_id=1 kind=ObjectLinkedToBuffer
[bell.detail.open] event_id=1 ok=1 reason=ok
[bell.detail.target] idx=0 event_id=1 total=2 ok=1 reason=detail_open_ok
[bell.detail.seed.proof] stage=4 action=open_detail ok=1 reason=ok
[bell.detail.seed.proof] stage=5 action=close_detail ok=1 reason=ok
[bell.detail.seed.proof] stage=6 action=lane_cycle ok=1 reason=ok
[bell.detail.seed.proof.done] ok=1
```

- `bell.event.seed.visible`: 1
- `bell.detail.target`: 1
- `bell.detail.open`: 2 (kind + ok=1)
- `bell.detail.seed.proof` stages: 7 (0-6)
- `bell.detail.seed.proof.done`: 1
- `bell.nav.move`: 2
- faults: 0

## Files Changed

`servers/silk-shell/src/main.rs`
- Added `BELL_DETAIL_SEED_PROOF_ENABLED` const (gated on `SEXOS_BELL_DETAIL_SEED_PROOF`)
- Added `BELL_DETAIL_SEED_PROOF_DONE` static flag
- Added `maybe_run_bell_detail_seed_proof()` (~80 lines) with 7 proof stages
- Added call site in main loop proof chain (after `maybe_run_bell_keyboard_detail_proof`)
- Added diagnostic skip marker for disabled case

## Build Results
```
SEXOS_BELL_DETAIL_SEED_PROOF=1 ./scripts/entrypoint_build.sh → PASS
./scripts/entrypoint_build.sh → PASS
```

## Additional Markers Added
- `[bell.event.seed.visible]` event_id=N total=N ok=N — seeded ring state
- `[bell.detail.target]` idx=N event_id=N total=N ok=N reason=... — selected target before open
- `[bell.detail.seed.proof]` stage=N action=NAME ok=N reason=... — proof stages
- `[bell.detail.seed.proof.done]` ok=N — final proof gate

## Notes
- No Bell server changes. No sex-pdx/ABI edits.
- No Quil, USB, pointer, kernel, or sexdisplay edits.
- The synthetic events use object_id=1000/1001 buffer_id=1000/1001 to clearly
  mark them as proof-seeded (distinct from real object IDs).
- Bell server demo event (event_id=1 in sexbell queue) is independent of
  silk-shell's local ring; no cross-PD sync attempted in this V1 fix.
