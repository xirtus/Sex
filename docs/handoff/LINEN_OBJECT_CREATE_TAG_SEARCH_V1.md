# LINEN_OBJECT_CREATE_TAG_SEARCH_V1 — Handoff

## Goal
Prove local Linen object workflow: create objects with names, tag them, search
by token, show detail.  No blocking PDX wait, no storage sync, no destructive
delete.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/linen/src/main.rs` | Tag table + 4 helper fns + 9-stage proof | +241 |

## Architecture
- **Tag table**: `LINEN_TAG_TABLE` — 16 slots, each holds (tag_bytes, tag_len, object_id)
- **Create**: Uses existing `SESSION.create()` — no new opcode
- **Tag**: `linen_tag_object()` writes (object_id, tag) into static table
- **Search**: `linen_search_by_token()` scans object names + tag table for substring
- **Detail**: `linen_object_detail()` prints name/kind/owner/tag_count for one object

## Proof Stages (9 stages, burst loop)
0. Create Document "work-doc-alpha", tag="work"
1. Create Session "session-beta-tag", tags="beta"+"work"
2. Create Document "team-work-gamma" (token in name, no explicit tag)
3. Search token="work" → expects ≥2 matches
4. Search token="beta" → expects ≥1 match
5. Detail last created object
6. Search nonexistent token="zzznope" → expects 0
7. Detail nonexistent id=0xFFFF → graceful fail
8. Done marker

## Markers (serial)
```
[linen.object.create] object_id=N kind=N ok=N reason=...
[linen.object.tag] object_id=N tag=NAME ok=N reason=...
[linen.search.query] token=NAME count=N ok=N
[linen.search.result] object_id=N selected=N ok=N
[linen.object.workflow.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_LINEN_OBJECT_WORKFLOW_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `linen_object_workflow`: PASS (3 creates, 3 searches)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer changes
- ❌ No blocking PDX pdx_listen_raw loops — proof runs in bounded burst before main loop
- ❌ No storage sync / RamFS / DiskFS writes — purely local session+tag table
- ❌ No destructive delete
- ✅ Tag table is static BSS, no heap, bounded to 16 entries × 16 bytes each

## Known Limitations
- Tag table is separate from LinenObject struct (not persisted)
- Search is linear O(n) substring scan, no index
- Object IDs truncated to u64 (session model unchanged)
- No wildcard/regex search

## Future Follow-up
- Integrate tags into LinenObject struct or SexFiles metadata record
- Add search-by-kind filter
- Persistent tag store via SexFiles
- PDX opcode for remote tag queries
