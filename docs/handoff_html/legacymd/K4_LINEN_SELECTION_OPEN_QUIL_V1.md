# K4: Linen Selection State — Open Selected Object in Quil

**Status:** Handoff (code + docs)
**Commit:** *(to be committed)*
**Source:** User prompt for K4 after K3 Quil buffer list
**Purpose:** Replace hardcoded `open_linen_object_in_quil(3)` with shell-local Linen
selection state, allowing the user to cycle through objects via J/K keys and open
the selected one in Quil via PrintScreen.

## 1. Changes

### 1.1 Selection State (`servers/silk-shell/src/main.rs`)

| Addition | Location | Description |
|----------|----------|-------------|
| `SELECTED_LINEN_OBJECT_ID` | ~line 233 | `static mut u64 = 0` — 0 = unset, repaired on first access |
| `linen_selected_object_id()` | ~line 371 | Returns current selection, repairs if 0 |
| `linen_select_first_valid_object()` | ~line 385 | Scans for first `Some` object |
| `linen_select_next_object()` | ~line 399 | Cycles forward, wraps around |
| `linen_select_prev_object()` | ~line 431 | Cycles backward, wraps around |
| `SurfaceAction::SelectNextLinenObject` | ~line 1683 | New enum variant |
| `SurfaceAction::SelectPrevLinenObject` | ~line 1684 | New enum variant |
| Scancode 0x24 → SelectNextLinenObject | ~line 1842 | J key |
| Scancode 0x25 → SelectPrevLinenObject | ~line 1843 | K key |

### 1.2 Updated Functions

| Function | Change |
|----------|--------|
| `linen_render_object_list()` | Row proof markers now include `selected=true/false`. Emits `[linen.object_select.current]` after rows. |
| `OpenObjectInQuil` handler | Calls `linen_selected_object_id()` instead of hardcoded `3`. Rejects with `[linen.quil.open.reject.no_selection]` if 0. |

### 1.3 Gating

| Action | Gate | Proof Marker |
|--------|------|-------------|
| J (SelectNext) | `FOCUSED_SURFACE_ID == SURFACE_ID_LINEN` | `[linen.object_select.reject] reason=not_focused` |
| K (SelectPrev) | `FOCUSED_SURFACE_ID == SURFACE_ID_LINEN` | `[linen.object_select.reject] reason=not_focused` |
| PrintScreen (OpenInQuil) | Ungated (continues to work globally) | existing `[linen.quil.open.*]` markers |

## 2. Proof Markers

| Marker | When |
|--------|------|
| `[linen.object_select.current] id=N` | Current selection value emitted |
| `[linen.object_select.next] prev=N next=M` | Selection advanced (with or without "wrap") |
| `[linen.object_select.prev] prev=N current=M` | Selection moved backward (with or without "wrap") |
| `[linen.object_select.repair] id=N` | First access repaired unset (0) to valid |
| `[linen.object_select.reject] reason=no_objects` | No objects exist, cannot select |
| `[linen.object_select.reject] reason=single_object` | Only one object, cannot cycle |
| `[linen.object_select.reject] reason=not_focused` | J/K pressed while Linen not focused |
| `[linen.object_list.row] ... selected=true/false` | Object row now includes selected flag |

## 3. Key Behavior

- **Default state:** First valid object is auto-selected on first PrintScreen press (repair).
- **J key (0x24):** Advance selection to next object. Wraps from last to first.
- **K key (0x25):** Move selection to previous object. Wraps from first to last.
- **J/K gated:** Only functional when Linen surface is focused (FOCUSED_SURFACE_ID == SURFACE_ID_LINEN).
  This prevents accidental selection changes while using other surfaces.
  If Linen is not focused, J/K emit `[linen.object_select.reject] reason=not_focused` and are no-ops.
- **PrintScreen (0x59):** Opens the selected Linen object in Quil via existing
  `open_linen_object_in_quil()`. Rejects with `no_selection` if 0 (no objects).
- **Linen list refresh:** Selection change triggers `linen_render_object_list()`.

## 4. Files Changed

- `servers/silk-shell/src/main.rs` — all code changes (additive, ~120 lines)
- `docs/handoff/K4_LINEN_SELECTION_OPEN_QUIL_V1.md` — this document

## 5. Verification

- **Build:** `./scripts/entrypoint_build.sh` passes, ISO produced
- **Proof markers:** All 7 marker types present in source
- **No changes:** kernel/ABI/sex-pdx, sexdisplay, WINDOWS Vec, lifecycle enum, storage,
  editor/parser/compiler, filesystem, behavior of non-Linen surfaces

## 6. Future

- **K5:** Rapid audit K2–K4
- **K6:** Decide next UX: selection visual highlight, command palette, or
  renderer primitive STOP FIRST
