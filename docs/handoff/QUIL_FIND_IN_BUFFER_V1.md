# QUIL_FIND_IN_BUFFER_V1 — Handoff

## Goal
Add local in-memory find/search in Quil's text buffer.  Bounded 32-byte query,
linear scan, returns (first_index, count).  Static only, no heap, no storage.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/quil/src/main.rs` | `text_buffer_find()` function, find proof gate + function | +54 |

## Architecture
- **`text_buffer_find(query: &[u8]) → (usize, u8)`**: linear scan of buffer
- **Returns**: (first_match_index, total_count).  first_index = 0xFFFF if not found
- **Bounds**: query ≤ 32 bytes, query ≤ buffer length
- **Complexity**: O(n) single pass, no allocation

## Proof (3-stage exercise)
| Query | Buffer | Expected |
|-------|--------|----------|
| "HELLO" | "HELLO WORLD HELLO" | idx=0, count=2 |
| "WORLD" | "HELLO WORLD HELLO" | idx=6, count=1 |
| "XYZ" | "HELLO WORLD HELLO" | idx=0xFFFF, count=0 |

## Markers
```
[quil.find.query] len=N ok=N reason=...
[quil.find.result] idx=N count=N ok=N reason=...
[quil.find.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_QUIL_FIND_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `quil_find`: PASS (3 queries)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No heap — pure stack/register local variables
- ✅ Bounded query (32 bytes max), bounded scan (512 bytes max)
- ✅ Existing buffer and undo ring unchanged
