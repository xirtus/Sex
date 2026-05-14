# QUIL_STATUS_UNBLOCK_UPDATE_V1

Status: **PASS** — Quil status unblocked in palette and Spindle
Date: 2026-05-14
Attempts: 1

## Summary

After QUIL_HID_STASH_REPLAY_V1 and QUIL_KEYBOARD_BUFFER_NAV_FINISH_V1
proved Quil keyboard delivery and buffer navigation, the command palette
and Spindle status panels were still labeling Quil as delivery_blocked/BLOCK.
This updates both to reflect the proven state.

## Status Deltas

### Command Palette (`palette_item_status`)
| Field | Before | After |
|-------|--------|-------|
| `available` | `false` | **`true`** |
| `status_label` | `delivery_blocked` | **`keyboard_nav_ready`** |
| `reason` | `quil_keyboard_delivery_blocker` | **`quil_hid_stash_replay_buffer_nav_proven`** |
| Statusbar count | 8 | **9** (+1 for Quil) |

### Spindle Status Panel
| Section | Before | After |
|---------|--------|-------|
| `status` | `Quil BLOCK delivery deferred` | **`Quil PASS keyboard nav ready`** |
| `status.item` | `status=BLOCK reason=delivery_deferred` | **`status=PASS reason=keyboard_nav_ready`** |
| `apps` | `Quil BLOCK app delivery deferred` | **`Quil PASS keyboard nav ready`** |
| `blockers` | `Quil delivery STOP FIRST (deferred)` | **`Quil delivery PROVEN (stash/replay done)`** |

## Changes (2 files, 6 hunks)

### `servers/silk-shell/src/main.rs`
1. `palette_item_status()`: FocusQuil → `available=true, status=keyboard_nav_ready`
2. Proof function: `maybe_run_quil_status_unblock_proof()`
3. Proof gate: `SEXOS_QUIL_STATUS_UNBLOCK_PROOF=1`

### `apps/spindle/src/main.rs`
4. Status panel text: `Quil BLOCK` → `Quil PASS`
5. Status item marker: `status=BLOCK` → `status=PASS`
6. Apps panel text: `Quil BLOCK` → `Quil PASS`
7. Blockers text: `STOP FIRST (deferred)` → `PROVEN (stash/replay done)`

## Proof Markers
```
[quil.status.unblock.proof] stage=0 action=start ok=1
[quil.status.unblock.proof] stage=1 action=status_check available=1 status=keyboard_nav_ready
[shell.palette.statusbar] open=1 selected=0 available=9       ← Quil now counted
[shell.palette.status] idx=1 action=OpenQuil available=1 status=keyboard_nav_ready
[quil.status.unblock.proof] stage=2 action=status_emitted ok=1
[quil.status.unblock.proof.done] ok=1
```

## Test Results
- Build with flag: ✅ PASS
- Baseline: ✅ zero behavior change (Spindle text is static, proof is gated)
- Faults: ✅ 0
- No ABI/kernel changes

## Handoffs
- `docs/handoff/QUIL_STATUS_UNBLOCK_UPDATE_V1.md` — created
- `docs/handoff/QUIL_HID_STASH_REPLAY_V1.md` — prerequisite
- `docs/handoff/QUIL_KEYBOARD_BUFFER_NAV_FINISH_V1.md` — prerequisite
