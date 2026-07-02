# ATLAS_SCENE_SETTINGS_UI_V1

**Status:** Complete — built, committed.
**Build:** ISO produced, no errors.

---

## Summary

Adds minimal keyboard controls in Atlas mode for the SceneSettings model
fields added by ATLAS_SCENE_SETTINGS_MODEL_V1. Two keys:

| Key | Scancode | Action | Marker |
|-----|----------|--------|--------|
| A | `0x1E` | Cycle accent token (Clear → Warm → Cool → Coral → Gold → Clear) | `[atlas.scene.settings.accent]` |
| P | `0x19` | Toggle pinned flag | `[atlas.scene.settings.pin]` |

No visual feedback (accent/pinned cannot be rendered without sexdisplay
protocol changes). Metadata-only mutation with proof markers.

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | +24 lines in `handle_atlas_keyboard()` |
| `docs/handoff/ATLAS_SCENE_SETTINGS_UI_V1.md` | New handoff doc |

---

## Key Handling

Both keys are handled inside the existing `handle_atlas_keyboard()` match
block, after Escape and before the `_` fallthrough arm. They only fire while
`ATLAS_MODE_ENABLED` is true (guaranteed by the caller in the EV_KEY handler).

### Accent cycle (`0x1E` — 'A')

```rust
0x1E => {
    let sel = ATLAS_SELECTED_SCENE;
    if validate_scene_id(sel) {
        let idx = sel as usize;
        let new_accent = (SCENES[idx].accent + 1) % ACCENT_COUNT;
        SCENES[idx].accent = new_accent;
        // emits [atlas.scene.settings.accent] scene=N accent=M
    } else {
        // emits [atlas.scene.settings.ui.reject] fn=accent scene=N
    }
}
```

Cycles `0 → 1 → 2 → 3 → 4 → 0` (Clear → Warm → Cool → Coral → Gold).
Wraps via modulo; always produces a valid `ACCENT_DEFAULT..ACCENT_COUNT` value
regardless of prior state. No clamping needed beyond the modulo.

### Pin toggle (`0x19` — 'P')

```rust
0x19 => {
    let sel = ATLAS_SELECTED_SCENE;
    if validate_scene_id(sel) {
        let idx = sel as usize;
        let new_pinned = !SCENES[idx].pinned;
        SCENES[idx].pinned = new_pinned;
        // emits [atlas.scene.settings.pin] scene=N pinned=true/false
    } else {
        // emits [atlas.scene.settings.ui.reject] fn=pin scene=N
    }
}
```

Simple boolean flip. No side effects on scene ordering, lifecycle, or
frame operations — pinning semantics are reserved for future phases.

---

## Conflict Analysis

| Existing Key | Scancode | Conflict? |
|-------------|----------|-----------|
| Left arrow | `0x4B` | No — different scancode |
| Right arrow | `0x4D` | No |
| Up arrow | `0x48` | No |
| Down arrow | `0x50` | No |
| Enter | `0x1C` | No |
| Escape | `0x01` | No |
| 1-5 (number keys) | `0x02..0x06` | No |
| F10 (ToggleAtlas) | `0x44` | Falls through before Atlas intercept |
| **A (accent)** | **`0x1E`** | ✅ Unused in scancode table |
| **P (pin)** | **`0x19`** | ✅ Unused in scancode table |

Both scancodes are free — not present in `scancode_to_action()` or used
elsewhere in Atlas handling.

---

## Proof Markers

| Marker | Budget | Location | Condition |
|--------|--------|----------|-----------|
| `[atlas.scene.settings.accent]` | 16 | `handle_atlas_keyboard()` `0x1E` arm | Accent cycled on valid scene |
| `[atlas.scene.settings.pin]` | 16 | `handle_atlas_keyboard()` `0x19` arm | Pinned flag toggled on valid scene |
| `[atlas.scene.settings.ui.reject]` | 8 (shared) | Both arms' else branches | Invalid scene_id or mutation failure |

The `ui.reject` marker is emitted separately for accent and pin (distinguished
by `fn=` field) but both draw from the same `ATLAS_UI_REJECT_BUDGET` counter.

---

## Visual Behavior

**None.** The existing `atlas_render_stub()` uses hardcoded card colors
(`ATLAS_COLOR_CARD_ACTIVE`, `ATLAS_COLOR_CARD_SCENE`, `ATLAS_COLOR_CARD_EMPTY`)
and does not consume the `accent` or `pinned` fields. Rendering accent/pinned
would require either:

1. A sexdisplay protocol change (new opcode or extended color tokens)
2. A shell-side color resolution pass that selects card tints from
   `CUSTOM_TINT_BUNDLES` based on each scene's `accent` field

Both are deferred. The metadata is mutated correctly; the UI can consume it
when a render path exists.

---

## Persistence

**Explicitly deferred.** No storage protocol, no sexstore calls, no
serialization. All settings are ephemeral — reset on reboot.

---

## Build Verification

```sh
./scripts/entrypoint_build.sh
# Result: ISO produced, no errors
# Warnings: only pre-existing (unused import in sexstore, etc.)
```

---

## STOP FIRST Findings

| Condition | Finding |
|-----------|---------|
| Visualizing settings requires sexdisplay protocol change | ✅ No change attempted — metadata-only |
| Key handling conflicts with existing Atlas navigation | ✅ No conflict — `0x1E` and `0x19` are unused |
| Pinned semantics would require scene ordering/layout rewrite | ✅ Not wired to any behavior — flag only |
| Settings mutation needs storage | ✅ Not added |

**No STOP FIRST conditions triggered.**

---

## Diff

```diff
--- a/servers/silk-shell/src/main.rs
+++ b/servers/silk-shell/src/main.rs
         0x01 => { // Escape
             ...
         }
+        0x1E => { // 'A' — cycle accent token
+            let sel = ATLAS_SELECTED_SCENE;
+            if validate_scene_id(sel) {
+                let idx = sel as usize;
+                let new_accent = (SCENES[idx].accent + 1) % ACCENT_COUNT;
+                SCENES[idx].accent = new_accent;
+                // [atlas.scene.settings.accent]
+            } else {
+                // [atlas.scene.settings.ui.reject] fn=accent
+            }
+        }
+        0x19 => { // 'P' — toggle pinned flag
+            let sel = ATLAS_SELECTED_SCENE;
+            if validate_scene_id(sel) {
+                let idx = sel as usize;
+                let new_pinned = !SCENES[idx].pinned;
+                SCENES[idx].pinned = new_pinned;
+                // [atlas.scene.settings.pin]
+            } else {
+                // [atlas.scene.settings.ui.reject] fn=pin
+            }
+        }
         _ => {
             // fallthrough
         }
```

---

## References

- `ATLAS_SCENE_SETTINGS_MODEL_V1.md` — model definition (accent, pinned, helpers)
- `ATLAS_KEYBOARD_SELECT_V1.md` — existing Atlas keyboard navigation
- `handle_atlas_keyboard()` — line ~2485, where UI keys are added
- `ATLAS_SELECTED_SCENE` — static u8 at line ~1495, reused for settings target
- `validate_scene_id()` — bounds check helper from model phase
- `ACCENT_COUNT` — constant at line ~1368, used for modulo wrap
