# M2: Shell-Local Bell Event Ring

**Status:** Handoff (code + docs)
**Date:** 2026-05-05
**Purpose:** Replace proof-marker-only Bell event stubs with a real bounded
shell-local ring buffer. One event kind (ObjectLinkedToBuffer). No new PD,
no IPC, no ABI changes.

## Verdict

```
╔══════════════════════════════════════════════════════════════════╗
║                    PASS_M2                                      ║
╠══════════════════════════════════════════════════════════════════╣
║ Build:                 PASSES (ISO produced)                     ║
║ Forbidden areas:       CLEAN                                    ║
║ Ring capacity:         16                                       ║
║ Event kinds:           1 (ObjectLinkedToBuffer)                 ║
║ Overflow policy:       Overwrite oldest (deterministic)         ║
║ New PD/IPC/ABI:        NONE                                    ║
║ Existing markers:      PRESERVED                                ║
╚══════════════════════════════════════════════════════════════════╝
```

## Changes

### BellEvent Struct

```rust
struct BellEvent {
    event_id: u64,      // Monotonic event ID
    kind: BellEventKind, // V1: ObjectLinkedToBuffer only
    object_id: u64,      // Linen object involved
    buffer_id: u64,      // Quil buffer involved
    sequence: u64,       // Ring write order
}
```

**Total size:** 40 bytes per event (5 u64 fields). `BELL_RING_CAP=16` → 640 bytes.

### Static Ring State

| Variable | Type | Initial | Purpose |
|----------|------|---------|---------|
| `BELL_RING_CAP` | `const usize` | `16` | Ring capacity |
| `BELL_EVENTS` | `[Option<BellEvent>; 16]` | `[None; 16]` | Ring buffer storage |
| `BELL_RING_WRITE_INDEX` | `u64` | `0` | Monotonic write index (wraps via modulo) |
| `BELL_EVENT_SEQUENCE` | `u64` | `0` | Global event sequence counter |

The ring follows the exact same pattern as `TOMBSTONE_RING` (A6): fixed-size
array, `replace()` for atomic slot write, modulo for wrap, monotonic counters
for ordering.

### Ring Write: `bell_record_event()`

```
Write path:
  idx = BELL_RING_WRITE_INDEX % BELL_RING_CAP
  seq = BELL_EVENT_SEQUENCE++
  BELL_EVENTS[idx].replace(BellEvent { ... })
  BELL_RING_WRITE_INDEX++
  if slot was occupied: [bell.ring.overwrite]
  [bell.ring.write] idx=N event_id=N object_id=N buffer_id=N
```

### Updated `bell_emit_object_link_event()`

Existing flow preserved:
```
[bell.event.stub]                    ← existing (unchanged)
→ validation checks                  ← existing (unchanged)
→ [bell.event.reject.missing] if bad ← existing (unchanged)
→ [bell.event.object_link]           ← existing (unchanged)
→ bell_record_event()                 ← NEW (ring write)
  → [bell.ring.write]                 ← NEW
  → [bell.ring.overwrite] if wrap     ← NEW (conditional)
→ [bell.ring.done] count=N event_id=M ← NEW
→ [bell.event.done]                   ← existing (unchanged)
```

### Proof Markers

| Marker | Type | When |
|--------|------|------|
| `[bell.event.stub]` | Existing | Event kind and IDs |
| `[bell.event.object_link]` | Existing | Full link details |
| `[bell.event.reject.missing]` | Existing | Validation failure |
| `[bell.event.done]` | Existing | Completion |
| `[bell.ring.write]` | **New** | Ring slot written |
| `[bell.ring.overwrite]` | **New** | Previous entry existed |
| `[bell.ring.done]` | **New** | Post-write summary |

All existing K15 action-trace markers preserved. No breaks in the proof chain.

## Overflow Policy

When `BELL_RING_WRITE_INDEX >= BELL_RING_CAP`, the ring wraps and overwrites
the oldest entry. Deterministic. No allocation. Matches TOMBSTONE_RING.

```
[bell.ring.overwrite] idx=0 prev_event_id=8  ← entry 8 was overwritten by entry 16
```

## Files Changed

- `servers/silk-shell/src/main.rs` — struct, ring state, helpers, updated emitter
- `docs/handoff/M2_BELL_SHELL_LOCAL_RING_V1.md` — this document

## Verification

- **Build:** `./scripts/entrypoint_build.sh` → `[SEXOS ENTRYPOINT] success`
- **No changes:** kernel/, sex-pdx/, sexdisplay/, linen/, quil/
- **All existing events preserved:** ObjectLink emitted from J4→J5→J6→J7→K3 chain
- **Ring capacity 16** matches LINEN_MAX_OBJECTS (16) and QUIL_MAX_BUFFERS (16)
- **No new allocation:** Static array, no heap
- **Bell PD not created:** Shell-local only
