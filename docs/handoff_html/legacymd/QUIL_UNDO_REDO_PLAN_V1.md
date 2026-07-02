# QUIL_UNDO_REDO_PLAN_V1 — STOP-FIRST Design

## Goal
Design a bounded static undo/redo ring for Quil's 512-byte text buffer.
Docs-only.  No source changes.

## Memory Budget
Quil already uses:
- `QUIL_BUFFER[512]` — 512 bytes
- `QUIL_CURSOR_POS` — usize
- `QUIL_SEL_START`, `QUIL_SEL_END` — 2 × usize

Proposed undo ring:
- **Ring size**: 16 entries (power of 2 for wrap-around indexing)
- **Per entry**: 512 bytes (full buffer snapshot) + 1 byte cursor + 1 byte op_type
- **Total**: 16 × (512 + 1 + 1) = 16 × 514 = 8,224 bytes
- **Total Quil BSS**: ~512 + 8,224 + ~200 (existing) ≈ 8,936 bytes

## Static Ring Design
```
static mut UNDO_RING: [[u8; 512]; UNDO_DEPTH];  // 16 buffer snapshots
static mut UNDO_CURSOR: [usize; UNDO_DEPTH];       // cursor position per snapshot
static mut UNDO_OP: [UndoOp; UNDO_DEPTH];          // operation type per snapshot
static mut UNDO_HEAD: usize;                       // next write position
static mut UNDO_TAIL: usize;                       // oldest entry (for redo)
static mut UNDO_COUNT: usize;                      // entries available for undo
```

## Operations Captured
| Operation | Undo Action | Op Type |
|-----------|------------|---------|
| Append char | Delete char at cursor | OpAppend |
| Backspace | Restore deleted char | OpBackspace |
| Delete char | Restore deleted char at position | OpDelete |
| Newline | Remove \n | OpNewline |
| Delete to EOL | Restore deleted segment | OpDeleteEOL |
| Delete line | Restore deleted line | OpDeleteLine |

## Redo
- When user undoes, the undone state is pushed to a redo stack
- Redo replays the operation forward
- Redo cleared on new edit (not undo/redo)

## STOP-FIRST Boundaries
- ❌ No heap allocation — static BSS only
- ❌ No unbounded growth — fixed 16-entry ring
- ❌ No kernel/ABI/USB/input/pointer changes
- ✅ Full buffer snapshot approach (simple, correct)
- ❌ Snapshot-based: 8KB memory cost for 16 undo levels

## Alternatives Considered
1. **Delta-based**: Store only the diff from previous state. Saves memory (~64 bytes per entry) but complex merge logic.
   - Rejected: complexity risk, correctness hard to prove.
2. **Command-based**: Store the inverse operation (e.g., "delete char at pos 3").
   - Rejected: requires position tracking, fragile with multi-step undo.
3. **Snapshot-based** (chosen): Store full 512-byte buffer per undo level.
   - Accepted: simple, correct, bounded. 8KB is acceptable in no_std BSS.

## Implementation Phases
1. **Phase A**: Add undo ring static arrays + UndoOp enum
2. **Phase B**: Instrument text_buffer_append/backspace/delete_char/newline/delete_to_eol/delete_line to push snapshots
3. **Phase C**: Add `text_buffer_undo()` and `text_buffer_redo()` functions
4. **Phase D**: Wire Ctrl+Z (undo) and Ctrl+Y (redo) scancodes in dispatch
5. **Phase E**: Proof gate + env var + daily driver gate

## Proof Markers (planned)
```
[quil.undo.push] op=NAME depth=N ok=N
[quil.undo.pop] op=NAME depth=N ok=N
[quil.undo.redo] op=NAME depth=N ok=N
[quil.undo.proof.done] ok=N
```

## Risk Assessment
- **Memory**: 8,224 bytes additional BSS — well within typical kernel module limits
- **Performance**: 512-byte memcpy per operation — negligible
- **Correctness**: Snapshot approach is trivially correct (restore = full state)
- **Interference**: Must snapshot AFTER each operation, before any redraw

## Decision
**Deferred** — design complete, implementation pending.  Low risk, high user value.
Recommended as Tier 2 mission (QUIL_UNDO_RING_V1) after delete keybindings.
