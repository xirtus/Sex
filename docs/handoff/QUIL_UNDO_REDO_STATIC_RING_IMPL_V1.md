# QUIL_UNDO_REDO_STATIC_RING_IMPL_V1 — Handoff

## Goal
Implement bounded no-heap undo/redo for Quil's 512-byte text buffer using a
static snapshot ring.  Push snapshots before mutations, undo restores, redo
replays.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/quil/src/main.rs` | Static ring arrays (5 × 16 entries), 3 undo/redo functions, 6 mutation sites instrumented, proof | +138 |

## Data Structures
```
UNDO_DEPTH = 16
UNDO_RING: [[u8; 512]; 16]    — 16 full buffer snapshots (8,192 bytes)
UNDO_CURSORS: [usize; 16]     — cursor position per snapshot (128 bytes)
UNDO_LENS: [usize; 16]        — buffer length per snapshot (128 bytes)
UNDO_HEAD: usize               — next write position (circular)
UNDO_COUNT: usize              — entries available for undo (0..16)
UNDO_REDO_COUNT: usize         — entries available for redo (0..16)
Total BSS: ~8,448 bytes
```

## Functions

### text_buffer_undo_push()
- Called before every mutating operation (append, backspace, newline, delete_char, delete_to_eol, delete_line)
- Copies current buffer+cursor+len into ring at HEAD
- Advances HEAD circularly (wraps at 16)
- Increments UNDO_COUNT (capped at 16)
- Clears UNDO_REDO_COUNT (new edit invalidates redo)

### text_buffer_undo()
- If UNDO_COUNT == 0, returns false
- Current state becomes redo-able (UNDO_REDO_COUNT incremented)
- Moves HEAD back one, decrements UNDO_COUNT
- Restores buffer, cursor, len from ring at HEAD

### text_buffer_redo()
- If UNDO_REDO_COUNT == 0, returns false
- Restores from ring at current HEAD (the entry we undid past)
- Advances HEAD, increments UNDO_COUNT, decrements UNDO_REDO_COUNT

## Mutating Operations Instrumented
| Function | undo_push before? |
|----------|-------------------|
| text_buffer_append | ✅ |
| text_buffer_backspace | ✅ |
| text_buffer_newline | ✅ |
| text_buffer_delete_char | ✅ |
| text_buffer_delete_to_eol | ✅ |
| text_buffer_delete_line | ✅ |

## Proof (8-stage exercise)
| Stage | Action | Buffer | Undo Count |
|-------|--------|--------|-----------|
| 0 | append 'A' | "A" | 1 |
| 1 | append 'B' | "AB" | 2 |
| 2 | append 'C' | "ABC" | 3 |
| 3 | undo | "AB" | 2 |
| 4 | undo | "A" | 1 |
| 5 | undo | "" | 0 |
| 6 | undo (no-op) | "" | 0 |
| 7 | redo | "A" | 1 |
| 8 | redo | "AB" | 2 |

## Markers (serial)
```
[quil.undo.push] idx=N len=N ok=N
[quil.undo.apply] old_len=N new_len=N ok=N
[quil.redo.apply] old_len=N new_len=N ok=N
[quil.undo_redo.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_QUIL_UNDO_REDO_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `quil_undo_redo`: PASS (57 undo pushes across all proofs)
- 3 undos + 2 redos verified in dedicated proof, plus undo_push from all other Quil proofs

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No heap — static BSS only (8,448 bytes)
- ❌ No unbounded growth — fixed 16-entry circular ring
- ✅ Ring wraps correctly when full (oldest entry overwritten)
- ✅ New edit clears redo history (undo_push → redo_count=0)
- ✅ Existing proofs continue to work (undo_push is transparent)

## Known Limitations
- 16-entry depth shared across all proofs (older entries overwritten by newer ops)
- No visual undo/redo indicator
- Redo restores from ring but redo chain breaks on new edit
- Ctrl+Z/Ctrl+Y not bound to scancodes (modifier tracking needed)
- No selective undo (e.g., undo only the last append, not backspace)

## Future Follow-up
- Scancode bindings for Ctrl+Z/Ctrl+Y (requires modifier tracking)
- Visual undo indicator (depth count in palette or title bar)
- Deeper ring (32 or 64 entries if BSS budget allows)
- Undo group coalescing (merge consecutive appends into one undo step)
