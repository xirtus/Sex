# LINEN_SEARCH_BRIDGE_ABI_PLAN_V1 — STOP-FIRST Design

## Goal
Design a safe, nonblocking Spindle→Linen search bridge ABI without implementing
it (STOP FIRST — requires new Linen opcode, violates current "no ABI" hard rule).

## Current Blocker (from V5 audit)
```
Spindle has SLOT_LINEN (PD 7: linen) but no OP_LINEN_SEARCH_OBJECTS opcode.
Linen's search (linen_search_by_token) is local in-memory — not exposed via PDX.
Existing opcodes: CREATE(0x41), LIST(0x42), GET(0x43),
  PUBLIC_SNAPSHOT(0x44), PUBLIC_NAME(0x45), OPEN_INTENT(0x46).
Spindle cannot trigger remote search without a new ABI opcode.
```

## Proposed Opcode
```
OP_LINEN_SEARCH_OBJECTS = 0x47

arg0: token bytes (packed LE, up to 8 bytes)
arg1: token bytes 8-15 (packed LE, up to 8 bytes)
arg2: flags (bits 0-7 = max_results, bit 8 = search_tags, bit 9 = search_names)

Reply: packed u64
  bits 0-15   = match_count (u16)
  bits 16-23  = first_match_kind (u8)
  bits 24-63  = first_match_object_id (u40)
  If match_count == 0, entire value is 0.
  Subsequent matches retrieved via OP_LINEN_LIST_OBJECTS iteration.
```

## Nonblocking Fire-and-Forget Design
- Spindle sends `pdx_call(SLOT_LINEN, 0x47, token_lo, token_hi, flags)` — fire-and-forget
- Linen receives, calls `linen_search_by_token()`, replies with packed result
- Spindle consumes reply via `pdx_listen_raw(0)` type=0x1 in its main loop
- No blocking wait: Spindle continues its event loop after sending
- Reply arrives asynchronously (same pattern as existing `pdx_listen_raw`)

## Reply / Readback Limitation
- Single reply carries only first match. Subsequent matches require:
  - Second PDX call (no stateful cursor across calls)
  - Or client-side filtering via OP_LINEN_LIST_OBJECTS loop
- No streaming/generator pattern (AsyncEnqueue is one-shot request/reply)
- Client must poll iteratively: search → get first → list next → repeat

## Implementation Phases

### Phase A: Linen Server Handler (servers/linen/src/main.rs)
1. Define `OP_LINEN_SEARCH_OBJECTS = 0x47`
2. Add match arm in main loop
3. Unpack token from arg0/arg1, flags from arg2
4. Call `linen_search_by_token()` — reuse existing local search
5. Pack result (count + first match) into u64 reply
6. Call `pdx_reply(caller_pd, packed_result)`

### Phase B: Spindle Client Command (apps/spindle/src/main.rs)
1. Update `object-search` command to use `OP_LINEN_SEARCH_OBJECTS`
2. Send fire-and-forget `pdx_call(SLOT_LINEN, 0x47, ...)`
3. Capture reply in main loop via `pdx_listen_raw(0)` type=0x1
4. Render results to scrollback
5. If match_count > 1, offer `object-search-next` command

### Phase C: Proof + Gate
1. Add `SEXOS_LINEN_SEARCH_BRIDGE_PROOF=1` env var
2. Proof: auto-execute `object-search work` at boot
3. Gate: `linen_search_bridge` in daily_driver_master_gate.sh

## Proof Markers (planned)
```
[linen.search.recv] token=NAME ok=N reason=...
[linen.search.result] count=N first_id=N ok=N
[spindle.linen.search.send] token=NAME status=N err=N
[spindle.linen.search.recv] count=N first_id=N ok=N
[linen.search.bridge.proof.done] ok=N
```

## Safety / STOP FIRST
- ❌ Requires new Linen opcode (0x47) — ABI change, violates current hard rule
- ✅ Fire-and-forget design — no blocking waits
- ✅ Reuses existing `linen_search_by_token()` — no new search logic
- ❌ No implementation in this batch — STOP FIRST design doc only

## Decisions Deferred
- Whether 0x47 is the right opcode number (could collide with future Linen ops)
- Whether search should be case-sensitive (current: exact byte match)
- Whether to support regex/wildcard (current: substring only)
- Multi-match cursor protocol (stateful vs stateless)

## Future Follow-up
- Promote to implementation when ABI freeze window opens
- Consider unified search opcode (search by name + kind + tag in one call)
- Client-side result cache for multi-match iteration
- PDX streaming reply protocol for large result sets
