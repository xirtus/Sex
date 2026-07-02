# M1: Bell Event Real Implementation — STOP FIRST Design

**Status:** STOP FIRST design only — no code changes.
**Date:** 2026-05-05
**Purpose:** Design the smallest safe path from Bell event stubs to a real
bounded Bell event ring buffer in silk-shell. One event kind only
(ObjectLinkedToBuffer). No new PD, no new IPC, no new ABI.

## Verdict

```
╔══════════════════════════════════════════════════════════════════╗
║              SAFE_SHELL_LOCAL_RING                               ║
╠══════════════════════════════════════════════════════════════════╣
║                                                                  ║
║  Shell-local ring:          SAFE (no new PD, no IPC)             ║
║  Existing Bell PD:          NONE (shell-local only today)        ║
║  New PDX opcodes needed:    NO                                   ║
║  New sex-pdx ABI needed:    NO                                   ║
║  New kernel changes:        NO                                   ║
║  Heap allocation:           NO (static ring, 16 entries)         ║
║  Renderer changes:          NO                                   ║
║  Collar authority:          NOT YET (deferred to future phase)   ║
║                                                                  ║
╚══════════════════════════════════════════════════════════════════╝
```

**Chosen approach:** Replace proof-marker-only Bell stubs with a static ring buffer
in silk-shell. Events are recorded in memory, emitted via proof markers, and
rendered on the Bell placeholder surface using the existing L2/L4/L6 multi-rect
pattern. No new PD, no IPC, no ABI changes. Collar integration deferred.

## 1. Current Bell State

### What Exists Today

| Component | Status | Source Reference |
|-----------|--------|-----------------|
| `BellEventKind` enum | ✅ 4 variants | `servers/silk-shell/src/main.rs` line 1194 |
| `bell_emit_object_link_event()` | ✅ Proof-marker-only | `servers/silk-shell/src/main.rs` line 1205 |
| `BELL_PLACEHOLDER` surface (ID 204) | ✅ Frame 6, PageDown toggle | `servers/silk-shell/src/main.rs` line 5764+ |
| Bell panel surface (ID 0x95) | ⚠️ Exists in OS panel tier, not implemented as overlay | `servers/silk-shell/src/main.rs` line 77 |
| G1 event contract doc | ✅ Full 16-field spec | `docs/handoff/G1_BELL_EVENT_CONTRACT_V1.md` |
| J7 object link stub doc | ✅ Handoff doc | `docs/handoff/J7_BELL_OBJECT_LINK_EVENT_V1.md` |
| Bell standalone PD (`servers/bell/`) | ❌ Does not exist | No directory or Cargo.toml |
| `SLOT_BELL` in IPCPKU_MAP | ❌ Not assigned | `IPCPKU_MAP.md` |
| Bell PDX opcodes | ❌ None defined | `crates/sex-pdx/src/lib.rs` |
| Real event queue/ring | ❌ Not implemented | Current code emits markers only |
| Bell surface row rendering | ❌ Not implemented | Bell surface is a fill-rect placeholder only |

### Current `bell_emit_object_link_event()` Flow

```
J4 linen_object_in_quil()
  → collar_check_operation_stub()
  → mesh_emit_linen_quil_links()
  → bell_emit_object_link_event(object_id, buffer_id) ← WE ARE HERE
      → validates object_id exists
      → validates buffer_id exists
      → validates buffer.linen_object_ref == object_id
      → emits [bell.event.stub], [bell.event.object_link], [bell.event.done]
      → NO QUEUE WRITE
      → NO SURFACE RENDER
  → quil_render_buffer_list()
```

The function validates but stores nothing. It emits proof markers as a trace
trail, but no event data persists.

## 2. Minimal Event Schema

### M2 Event Record (V1: One kind only)

```rust
/// A single Bell event record stored in the shell-local ring buffer.
/// V1 supports only ObjectLinkedToBuffer. Fields are fixed-size scalars only.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct BellEvent {
    /// Monotonic event ID (incremented per stored event).
    event_id: u64,
    /// Event kind (V1: only ObjectLinkedToBuffer is actually emitted).
    kind: BellEventKind,
    /// The Linen object_id involved (0 if not applicable).
    object_id: u64,
    /// The Quil buffer_id involved (0 if not applicable).
    buffer_id: u64,
    /// Monotonic counter at time of event (for ordering, not timestamps).
    sequence: u64,
}
```

**Total size:** 4 × u64 + 1 × u8 + padding ≈ 40 bytes per event.

**Key design decisions:**
- No strings, no pointers, no heap references.
- `object_id` and `buffer_id` are shell-local IDs (0 = not applicable).
- `sequence` is a monotonic u64 counter (like `LifecycleGeneration`). Not
  a real timestamp — avoids clock dependency.
- `event_id` is the ring-local generation counter (like tombstone events).
- `kind` uses the existing `BellEventKind` enum (repr(u8)).

### Comparison to G1 Full Schema

| Field | G1 (Full Vision) | M1/M2 (V1) | Rationale |
|-------|------------------|------------|-----------|
| event_id | u64 | u64 | Same — needed for ordering |
| sender_pd | u8 | ❌ | Shell-local only; no cross-PD yet |
| sender_identity | u64 | ❌ | No app manifests yet |
| category | enum | ❌ | Only ObjectLinkedToBuffer in V1 |
| urgency | enum | ❌ | Default Normal for V1 |
| lane | enum | ❌ | Default System for V1 |
| title | [u8; 64] | ❌ | No text rendering in V1 |
| body | [u8; 256] | ❌ | No text rendering in V1 |
| target_scene | u8 | ❌ | Implicit from active scene |
| target_frame | u32 | ❌ | Implicit from linen/quil frame |
| target_tab | u8 | ❌ | N/A for object links |
| target_object | u64 | u64 → `object_id` | Same concept, shell-local |
| privacy_class | enum | ❌ | Default Public for V1 |
| expiration | u64 | ❌ | No timeout logic in V1 |
| action_token_id | u64 | ❌ | Deferred to Collar integration |
| action_token_scope | u64 | ❌ | Deferred to Collar integration |
| proof_marker | [u8; 16] | ❌ | Proof markers emitted separately |
| lifecycle_state | enum | ❌ | Events are always "Active" in V1 |

**M1/M2 fields: 5. G1 fields: 16. M1/M2 is ~1/3 the complexity of full Bell.**

## 3. Ring Buffer Design

### Constants

```rust
/// Maximum number of Bell events stored in the shell-local ring.
const BELL_RING_SIZE: usize = 16;

/// Monotonic event counter (starts at 1; 0 = reserved/no event).
static mut BELL_EVENT_SEQUENCE: u64 = 0;

/// Static ring of Bell events. Index = event_id % BELL_RING_SIZE.
static mut BELL_RING: [Option<BellEvent>; BELL_RING_SIZE] = [None; BELL_RING_SIZE];

/// Current write index into the ring (monotonic, wraps).
static mut BELL_RING_NEXT: u64 = 0;
```

### Write Path

```rust
unsafe fn bell_record_event(object_id: u64, buffer_id: u64) {
    let event_id = BELL_EVENT_SEQUENCE;
    BELL_EVENT_SEQUENCE += 1;
    let seq = BELL_RING_NEXT;
    BELL_RING_NEXT += 1;
    let idx = (seq as usize) % BELL_RING_SIZE;
    BELL_RING[idx] = Some(BellEvent {
        event_id,
        kind: BellEventKind::ObjectLinkedToBuffer,
        object_id,
        buffer_id,
        sequence: seq,
    });
    serial_println!("[bell.ring.write] idx={} event_id={} object_id={} buffer_id={}",
        idx, event_id, object_id, buffer_id);
}
```

### Read/Iterate Path

```rust
/// Return the number of events currently in the ring.
unsafe fn bell_event_count() -> usize {
    let total = BELL_RING_NEXT;
    if total == 0 { return 0; }
    core::cmp::min(total as usize, BELL_RING_SIZE)
}

/// Iterate events from oldest to newest, calling a closure for each.
unsafe fn bell_for_each_event<F>(mut f: F) where F: FnMut(&BellEvent) {
    let total = BELL_RING_NEXT;
    let count = bell_event_count();
    let start = if total as usize > BELL_RING_SIZE { total - BELL_RING_SIZE as u64 } else { 0 };
    for i in 0..count {
        let idx = ((start + i as u64) as usize) % BELL_RING_SIZE;
        if let Some(ref ev) = BELL_RING[idx] {
            f(ev);
        }
    }
}
```

### Overflow Policy: Overwrite Oldest

When the ring wraps (event count > `BELL_RING_SIZE`), the oldest entry is
silently overwritten. This is deterministic, no-allocation, and matches the
existing `TOMBSTONE_RING` pattern (A6).

```rust
// No special overflow check needed: the ring naturally overwrites.
// Oldest entries are at (seq - BELL_RING_SIZE) % BELL_RING_SIZE when
// seq >= BELL_RING_SIZE.
```

### Why 16?

- Matches `TOMBSTONE_RING` size (also 16).
- Matches `LINEN_MAX_OBJECTS` (16) and `QUIL_MAX_BUFFERS` (16).
- Sufficient for last 16 object-link events visible in the Bell surface.
- 16 × 40 bytes = 640 bytes total. Negligible.

### Comparison to Existing TOMBSTONE_RING

| Property | TOMBSTONE_RING (A6) | BELL_RING (M1/M2) |
|----------|---------------------|-------------------|
| Size | 16 | 16 |
| Element type | `Option<TombstoneEvent>` | `Option<BellEvent>` |
| Write | `record_tombstone_event()` | `bell_record_event()` |
| Iterate | Scan for `is_some()` | Sequential from oldest |
| Overflow | Overwrite oldest (same ring pattern) | Overwrite oldest |
| Proof marker | `[tombstone.event.record]` | `[bell.ring.write]` |

## 4. Ownership Split

| Responsibility | silk-shell (PKEY 3) | sexdisplay (PKEY 1) |
|---------------|---------------------|---------------------|
| Event queue storage | ✅ Ring buffer in silk-shell | ❌ |
| Event kind definitions | ✅ BellEventKind enum | ❌ |
| Event validation | ✅ Object/buffer existence checks | ❌ |
| Event proof markers | ✅ [bell.ring.write] etc. | ❌ |
| Bell surface row rendering | ✅ Uses multi-rect (L2 pattern) | ❌ |
| Fill rect compositing | ❌ Sends 0xEF calls | ✅ Dumb rect renderer |
| Event authority | ✅ Shell-local only (no PD yet) | ❌ |
| Collar grant checks | ❌ Deferred (stub-only today) | ❌ |

**Bell events are entirely shell-local in V1.** No cross-PD communication.
No new renderer involvement. Collar stub remains the gate for the
Linen→Quil link operation (J5), not for Bell storage.

## 5. Implementation Path: A. Shell-Local Ring

**Chosen: Option A (SAFE_SHELL_LOCAL_RING)**

### Why Not Options B/C

| Option | Description | Verdict |
|--------|-------------|---------|
| **A. Shell-local ring** | Static ring in silk-shell, no new PD | ✅ SAFE — matches TOMBSTONE_RING pattern |
| **B. Real Bell PD** | New `servers/bell/` with IPC slot | ❌ BLOCKED — requires new PDX opcodes, IPCPKU_MAP slot assignment, kernel route table edit |
| **C. ABI/opcode required** | New sex-pdx constants for Bell IPC | ❌ BLOCKED — STOP FIRST territory |

### Why Shell-Local First

1. **No authority boundary to design yet.** Bell events in V1 are produced by and
   consumed within the shell. No app or external PD produces events.
2. **TOMBSTONE_RING pattern is proven.** A6 demonstrated the same ring design
   works well for lifecycle events.
3. **IPC complexity not justified.** A new Bell PD (Option B) requires slot
   assignment in IPCPKU_MAP, kernel route table entries, new PDX opcodes,
   and a separate binary — all for storing 16 events that silk-shell already
   has all the data for.
4. **Future PD migration is easy.** When real Bell authority is needed (collaboration
   apps, cross-PD events), a real Bell PD can subscribe to the shell's ring or
   receive events via a new IPC. The ring design does not prevent migration.

## 6. M2 Implementation Prompt

```
MISSION: M2_BELL_EVENT_RING_QUEUE

Goal:
Replace proof-marker-only Bell event stubs with a real static ring buffer
in silk-shell. Store ObjectLinkedToBuffer events with validation. Add
proof markers for ring operations. No new PD, no IPC, no ABI.

Changes to servers/silk-shell/src/main.rs:

1. BellEvent struct (after BellEventKind enum, line ~1199):
   - event_id: u64
   - kind: BellEventKind
   - object_id: u64
   - buffer_id: u64
   - sequence: u64

2. Static ring state (near BELL_ACTIVE / existing Bell state):
   - const BELL_RING_SIZE: usize = 16
   - static mut BELL_RING: [Option<BellEvent>; BELL_RING_SIZE]
   - static mut BELL_RING_NEXT: u64 = 0
   - static mut BELL_EVENT_SEQUENCE: u64 = 0

3. Helper functions:
   - bell_record_event(object_id, buffer_id)
   - bell_event_count()
   - bell_for_each_event(closure)  -- for future render use

4. Update bell_emit_object_link_event():
   - Call bell_record_event() after validation succeeds
   - Keep existing validation and proof markers
   - Add [bell.ring.write] after record
   - Add [bell.ring.count] N after done

5. Add ring render to Bell surface (future M3):
   - NOT in M2 — just queue storage + proof markers

6. Do NOT:
   - Change any existing behavior
   - Add new surface rendering
   - Change Collar/Mesh stubs
   - Add new PDX or ABI
   - Change linen/quil render functions
   - Add text rendering
   - Add event kinds beyond ObjectLinkedToBuffer

Proof markers added:
- [bell.ring.write] idx=N event_id=N object_id=N buffer_id=N
- [bell.ring.count] count=N
- [bell.ring.overflow] (if wrap detected, optional)

Keep existing:
- [bell.event.stub]
- [bell.event.object_link]
- [bell.event.reject.missing]
- [bell.event.done]

Build:
./scripts/entrypoint_build.sh

Output:
docs/handoff/M2_BELL_EVENT_RING_V1.md
Code changes to servers/silk-shell/src/main.rs only.
```

## 7. STOP FIRST Table

| Item | Why STOP FIRST |
|------|----------------|
| New Bell PD (`servers/bell/`) | New IPC slot, kernel route table, PDX opcodes — full STOP FIRST |
| New PDX opcodes for Bell events | sex-pdx ABI edit — STOP FIRST |
| SLOT_BELL in IPCPKU_MAP | Cross-PD slot assignment — STOP FIRST |
| App-supplied Bell events | Requires app identity, Collar policy, cross-PD protocol — STOP FIRST |
| Real Collar grant enforcement in Bell | Bell + Collar integration — STOP FIRST |
| Persistent Bell event storage | Filesystem/storage code — STOP FIRST |
| Text/rich notification rendering | sexdisplay text primitive — STOP FIRST |
| Bell event queue in separate PD | Architectural decision for later — STOP FIRST |
| Real timestamps (clock-dependent) | Avoid clock dependency in V1 — STOP FIRST |

## 8. Forbidden Approaches

| Approach | Reason |
|----------|--------|
| Heap-backed event storage | Static ring only — no allocator dependency |
| Dynamic event kinds via registry | All event kinds hardcoded as enum variants |
| Cross-PD event sending in V1 | Shell-local only until Bell PD exists |
| Full G1 schema in V1 | 512-byte records not justified for single event kind |
| Event persistence to disk | No filesystem dependency for event queue |
| App event injection | No app manifests, no authority boundary yet |
| Bell surface chrome/lifecycle changes | Existing Bell placeholder is sufficient for V1 |

## 9. Remaining Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Ring wraps silently | LOW | 16 entries is large; overflow overwrites oldest (deterministic) |
| No event deduplication | LOW | V1 emits one event per object-link; duplicates are valid |
| No event dismissal/ack | LOW | V1 events persist in ring until overwritten |
| No Bell surface rendering in M2 | LOW | M2 only adds queue; M3 adds surface row rendering |
| G1 schema mismatch | LOW | M1/M2 fields are a subset of G1; forward-compatible |

## Proof

**Document complete:** `docs/handoff/M1_BELL_EVENT_REAL_IMPL_STOP_FIRST_DESIGN_V1.md`

**All required sections present:**
- ✅ Verdict: SAFE_SHELL_LOCAL_RING
- ✅ Current Bell state (stub status, no PD, no queue)
- ✅ Minimal event schema (5 fields, 40 bytes)
- ✅ Ring buffer design (16 entries, static, overwrite-oldest)
- ✅ Overflow policy (deterministic wrap, same as TOMBSTONE_RING)
- ✅ Ownership split (shell = queue, sexdisplay = rects only)
- ✅ STOP FIRST table (11 items)
- ✅ Forbidden approaches (7 items)
- ✅ Exact M2 implementation prompt
