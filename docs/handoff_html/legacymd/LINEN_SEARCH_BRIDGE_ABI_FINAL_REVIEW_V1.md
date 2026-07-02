# LINEN_SEARCH_BRIDGE_ABI_FINAL_REVIEW_V1 — STOP-FIRST Final Review

## Goal
Docs-only final review of OP_LINEN_SEARCH_OBJECTS=0x47 before any future
implementation.  No source changes.

## Current Blocker
Spindle has SLOT_LINEN but no OP_LINEN_SEARCH_OBJECTS.  Linen's search
(`linen_search_by_token`) is local in-memory — not exposed via PDX.

## Exact Proposed Opcode
```
OP_LINEN_SEARCH_OBJECTS = 0x47

Request:
  arg0: token bytes 0-7 (packed LE)
  arg1: token bytes 8-15 (packed LE, 0 if token ≤ 8 bytes)
  arg2: flags
    bits 0-7 = max_results (u8, 0=unlimited up to 16)
    bit 8    = search_tags (include tag table)
    bit 9    = search_names (include object names)

Reply (packed u64):
  bits 0-15   = match_count (u16, 0..16)
  bits 16-23  = first_match_kind (u8)
  bits 24-63  = first_match_object_id (u40)
  If match_count == 0, entire value is 0.
```

## Nonblocking / Fire-and-Forget Limits
- Spindle sends `pdx_call(SLOT_LINEN, 0x47, ...)` — fire-and-forget
- Linen processes, calls `linen_search_by_token()`, replies with packed result
- Spindle consumes reply via `pdx_listen_raw(0)` type=0x1 in main loop
- ✅ No blocking: Spindle continues event loop after send
- ✅ Reply arrives asynchronously (existing PDX reply pattern)

## Why Sync Readback Remains Unavailable
- Single reply carries only first match (packed u64 limit)
- Subsequent matches require additional PDX calls (no cursor/state across calls)
- Client must iterate: search → get first → list next → repeat
- No streaming/generator protocol in current PDX

## Implementation Phases
1. **Linen handler**: Add `OP_LINEN_SEARCH_OBJECTS = 0x47` match arm, unpack, search, reply
2. **Spindle client**: Update `object-search` to use new opcode, capture reply
3. **Proof**: E2e test — `object-search work` → Linen reply → Spindle display
4. **Gate**: `linen_search_bridge` in daily_driver_master_gate.sh

## STOP-FIRST Boundaries
- ❌ Requires new Linen opcode (0x47) — ABI change
- ✅ Reuses existing `linen_search_by_token()` — zero new search logic
- ✅ Fire-and-forget model — no blocking
- ❌ Not implemented — STOP FIRST review only

## Decision
**Ready for implementation** when ABI freeze allows new opcodes.
Low risk: reuses proven local search, fire-and-forget, bounded 16-object table.
