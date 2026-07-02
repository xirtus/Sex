# M8: Bell Event Detail Proof Stub

**Status:** Complete
**Date:** 2026-05-05
**Purpose:** Add Enter-on-selected-Bell-row proof stub. No action, no ack,
no delete, no event mutation, no Bell PD.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║               PASS_M8_BELL_DETAIL_PROOF_STUB                ║
╠══════════════════════════════════════════════════════════════╣
║ Event lookup:           READ-ONLY COPY from ring             ║
║ Enter gating:           Bell focused + palette/atlas pass    ║
║ Proof markers:          6 success + 3 reject                 ║
║ Boundaries:             INTAKT                               ║
║ Build:                  PASS                                 ║
╚══════════════════════════════════════════════════════════════╝
```

## Changes

**Files:** `servers/silk-shell/src/main.rs` (64 insertions, 8 deletions)

### 1. `bell_selected_event_snapshot()` — Read-only event lookup

```rust
unsafe fn bell_selected_event_snapshot() -> Option<BellEvent>
```

Maps `BELL_SELECTED_ROW` (visible row index, 0 = newest) to the corresponding
`BellEvent` in the ring. Iterates newest-first using the same index arithmetic
as `bell_for_each_event()`. Returns a `Copy` of the event. No ring mutation,
no allocation, no side effects.

Returns `None` when:
- Ring is empty (`bell_ring_count() == 0`)
- Selected index points to a `None` slot (should not occur in practice after wrapping)

### 2. `bell_emit_selected_event_detail_proof()` — Proof-marker-only stub

```rust
unsafe fn bell_emit_selected_event_detail_proof()
```

Three-guard validation, then proof markers:

| Guard | Marker | Condition |
|-------|--------|-----------|
| 1 | `[bell.detail.reject] reason=not_focused` | `FOCUSED_SURFACE_ID != SURFACE_ID_BELL_PLACEHOLDER` |
| 2 | `[bell.detail.reject] reason=no_event` | `bell_selected_event_snapshot()` returns None |
| 3 (per kind) | `[bell.detail.reject] reason=unsupported_kind` | Event kind is not `ObjectLinkedToBuffer` |

On success (ObjectLinkedToBuffer):
```
[bell.detail.open] event_id=N kind=ObjectLinkedToBuffer
[bell.detail.event] event_id=N kind=ObjectLinkedToBuffer object_id=M buffer_id=K
[bell.detail.object_link] object_id=M buffer_id=K
[bell.detail.done] event_id=N
```

### 3. Keyboard handler — Enter (0x1C) for Bell

Updated the Bell focused-surface intercept (originally just J/K) to also
handle Enter:

```rust
} else if FOCUSED_SURFACE_ID == SURFACE_ID_BELL_PLACEHOLDER
    && (scancode == 0x24 || scancode == 0x25 || scancode == 0x1C)
{
    match scancode {
        0x24 => { bell_select_next_row(); }
        0x25 => { bell_select_prev_row(); }
        0x1C => { bell_emit_selected_event_detail_proof(); }
        _ => {}
    }
    mutated = true;
```

## Keyboard Precedence

Enter (0x1C) dispatch chain:

| Context | Handler | Precedence | Behavior |
|---------|---------|------------|----------|
| Command palette open | `palette_execute_selected()` | 1 (highest) | Executes command, closes palette |
| Atlas active | `handle_atlas_keyboard(0x1C)` → scene confirm | 2 | Confirms scene selection |
| Bell focused | `bell_emit_selected_event_detail_proof()` | 3 | Proof markers only |
| Any other focus | `scancode_to_action` → `AccessActivate` | 4 | Default enter action |

The Bell intercept only fires when no higher-priority consumer (palette, atlas)
took the key first.

## Proof Marker Inventory

| Marker | Type | Description |
|--------|------|-------------|
| `[bell.detail.open]` | Success | Detail proof started, event_id + kind |
| `[bell.detail.event]` | Success | Event detail for ObjectLinkedToBuffer |
| `[bell.detail.object_link]` | Success | Object-to-buffer link detail |
| `[bell.detail.done]` | Success | Detail proof complete, event_id |
| `[bell.detail.reject]` | Reject | Not focused, no event, or unsupported kind |
| `[bell.keyboard.enter]` | Input | Enter key consumed for Bell |

## Boundaries

| Area | Status |
|------|--------|
| kernel/ | ✅ CLEAN |
| crates/sex-pdx/ | ✅ CLEAN |
| servers/sexdisplay/ | ✅ CLEAN |
| servers/bell/ | ✅ CLEAN |
| servers/linen/ | ✅ CLEAN |
| servers/quil/ | ✅ CLEAN |
| PDX ABI/opcodes | ✅ CLEAN |
| Ring mutation | ✅ NONE (read-only copy) |
| Ack/delete/action | ✅ NONE (proof markers only) |
| Bell PD creation | ✅ NONE |
| Text rendering | ✅ NONE |
| Collar/Mesh authority | ✅ NONE |

## Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| Proof stub does not act on the event | MEDIUM | By design — M8 is proof-marker-only. Next step adds real dispatch. |
| Only ObjectLinkedToBuffer kind supported | LOW | Only kind currently emitted. Other kinds will be added when implemented. |
| No visual feedback for Enter press | LOW | Proof markers only — no render change, no detail pane. |

## Build Result

```
[SEXOS TRACE] stage=package_iso
ISO image produced: 1610 sectors
[SEXOS ENTRYPOINT] success
```

**Build: PASS** — ISO produced successfully.

## Next Steps

**M9: Rapid audit of M8** — close the Bell detail proof stub milestone.
After M9: evaluate wiring selected event to existing handler chains
(view linked object in Linen/Quil via `open_linen_object_in_quil()`),
or move to other subsystem work.
