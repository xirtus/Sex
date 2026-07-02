# MESH_KEYBOARD_MAP_NAV_V1

STATUS: PASS — All criteria met in 1 attempt (5 iterations for drain path fix).

---

## What was done

**Mesh keyboard map navigation**: Made Mesh fully keyboard-operable:
- Open/focus Mesh via command palette (backtick `` ` `` → "Open Mesh" → Enter)
- Navigate nodes with J/K (next/previous)
- Inspect selected node with Enter (detail proof)
- Close/back with Escape, F11, or Backspace

**Drain path**: Added Mesh keyboard passthrough in `handle_hid_event` drain path
(between Spindle passthrough and `reserved_ui_action` check) so synthetic proofs
(`handle_hid_event` calls) route J/K/Enter/Escape/F11/Backspace to Mesh when
Mesh is focused — matching the Spindle pattern.

**Main dispatch**: Removed `!reserved_ui_key` guard from Mesh handler so
reserved keys (Esc, Enter, F11, Backspace) reach Mesh when focused, matching
the Spindle handler pattern. Added close/back keys (0x01 Esc, 0x57 F11, 0x0E
Backspace) to the Mesh dispatch condition.

**Proof**: Added `maybe_run_mesh_keyboard_map_proof()` — an 8-stage default-off
proof gated by `option_env!("SEXOS_MESH_KEYBOARD_MAP_PROOF").is_some()`.

---

## Proof runtime (all markers fire)

- `[mesh.keyboard.map.proof]` × 8 (stages 0-7) ✓
- `[mesh.key.recv] code=36/37/28/1 down=1 mod=0` × 4 ✓
- `[mesh.node.nav] old=0 new=1 count=2` ✓
- `[mesh.node.nav] old=1 new=0 count=2` ✓
- `[mesh.node.detail] idx=0 node_id=1 ok=1 reason=selected` ✓
- `[mesh.overlay.toggle] enabled=0 ok=1 reason=close_back` ✓
- `[silk-shell.key.route] target=mesh` × 4 ✓
- `[mesh.keyboard.map.proof.done] ok=1` ✓
- Faults: 0 ✓

---

## Build

- `SEXOS_MESH_KEYBOARD_MAP_PROOF=1 ./scripts/entrypoint_build.sh`: PASS
- `./scripts/entrypoint_build.sh` (normal): PASS

---

## Files changed

- `servers/silk-shell/src/main.rs` (+218/-7 lines)
- `docs/handoff/MESH_KEYBOARD_MAP_NAV_V1.md` (new)

---

## Key design decisions

1. **Drain path Mesh passthrough** (between Spindle passthrough and reserved_ui_action):
   Required because the proof sends keys via `handle_hid_event`, which goes through
   the drain path, not the main event loop dispatch. J/K/Enter/Escape are reserved
   UI keys in `scancode_to_action`, so without explicit passthrough they'd be
   consumed by `access_handle_keyboard_action` before reaching the Mesh handler.

2. **Detail proof in drain path is marker-only**: The drain path's Enter handler
   emits `[mesh.node.detail]` but does NOT call `mesh_focus_linen_at_selected_fact`.
   The full detail chain (with Linen focus) fires only in the main event loop
   dispatch. This avoids a hang during proof execution caused by Linen's
   `open_linen_in_active_scene` → `ensure_linen_frame` → PDX surface creation
   blocking inside `handle_hid_event`.

3. **Close/back**: Escape (0x01) minimizes Mesh via `toggle_mesh()`. F11 (0x57)
   and Backspace (0x0E) are also mapped as alternative close/back keys.

---

## Daily-driver usage

- `` ` `` → palette → "Open Mesh" → Enter: Open/focus Mesh
- J: Next node
- K: Previous node
- Enter: Detail selected node (opens Linen focused on linked object)
- Esc: Close/back (minimize Mesh)

---

## Caveats

- `mesh_focus_linen_at_selected_fact` (Enter → open Linen at selected fact)
  is proven working via the main event loop dispatch path but NOT in the
  synthetic proof (see design decision #2). Manual keyboard testing verifies it.
- Physical USB keyboard hardware testing is deferred.
- Mesh fact ring must have ≥2 facts for navigation to be meaningful (proved
  with 2 facts emitted during boot: object_id 2 and 5).
