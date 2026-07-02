# Linen Object Detail Keyboard V1

## Status: PASS
Date: 2026-05-14
Attempts: 1

## Summary
Added a non-blocking Linen object detail panel that shows object metadata
(object_id, kind, name, state, parent_id) via local LINEN_OBJECTS reads only.
No PDX calls, no linen_sync_reply blocking, no Quil integration.

## Design

The detail panel reads directly from silk-shell's local LINEN_OBJECTS table —
the same table populated by the remote snapshot fetch. This avoids the
blocking open path (linen_sync_reply → OP_LINEN_GET_PUBLIC_SNAPSHOT).

Functions added:
- `linen_object_detail_open()` — reads local object metadata, sets OPEN flag
- `linen_object_detail_close()` — clears OPEN flag
- `linen_selected_index()` — helper to get selected object index

State added:
- `LINEN_OBJECT_DETAIL_OPEN: bool` — detail panel open/closed

## Proof Table

| Stage | Action | ok | Result |
|-------|--------|----|--------|
| 0 | Focus Linen | 1 | ok |
| 1 | Next object (J) | 1 | object_id selected |
| 2 | Open detail | 1 | metadata shown (kind=Project, state=Loaded) |
| 3 | Prev object (K) | 1 | nav while detail open |
| 4 | Close detail | 1 | ok |
| 5 | Safety audit | 1 | local only, no blocking |

## Runtime Proof Counts

```
[linen.detail.open]           idx=0 object_id=1 ok=1 reason=ok
[linen.detail.metadata]       object_id=1 kind=Project state=Loaded parent_id=0 grant_ref=0
[linen.detail.close]          ok=1 reason=ok
[linen.object.detail.proof]   stage=0-5 all ok=1
[linen.object.detail.proof.done] ok=1
faults: 0
```

## Files Changed

`servers/silk-shell/src/main.rs`
- Added LINEN_OBJECT_DETAIL_PROOF_ENABLED const + DONE flag
- Added LINEN_OBJECT_DETAIL_OPEN state flag
- Added linen_object_detail_open() function (~30 lines)
- Added linen_object_detail_close() function (~8 lines)
- Added linen_selected_index() helper (~10 lines)
- Added maybe_run_linen_object_detail_proof() proof function (~55 lines)
- Added proof call site in main loop

`docs/handoff/LINEN_OBJECT_DETAIL_KEYBOARD_V1.md` (created)

## Build Results
```
SEXOS_LINEN_OBJECT_DETAIL_PROOF=1 ./scripts/entrypoint_build.sh -> PASS
./scripts/entrypoint_build.sh -> PASS
```

## Notes
- No kernel, ABI, USB, display, Quil, or pointer edits.
- No blocking — all reads are from local LINEN_OBJECTS table.
- Does NOT call linen_sync_reply() or any PDX call.
- The blocking open path (Enter → OP_LINEN_OPEN_INTENT → linen_sync_reply)
  is unchanged; the detail panel is an alternative non-blocking view.
- Future: can be extended to show name bytes, linked surfaces, flags, etc.
