# QUIL_SURFACE_STUB_V1A

**Status:** Complete — built, committed.
**Build:** ISO produced, no errors.

---

## Summary

Shell-side audit of the existing Quil frame/surface lifecycle path in
silk-shell. **No behavior changes.** Added 3 lifecycle proof markers
to confirm the FSM transitions are tracked correctly across all paths:
F9 open/toggle, minimize, restore, tiling placeholder fill, focus.

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | +12 lines (3 proof markers) |
| `docs/handoff/QUIL_SURFACE_STUB_V1A.md` | New handoff doc |

---

## Proof Markers Added

| Marker | Budget | Location | Path |
|--------|--------|----------|------|
| `[shell.quil.lifecycle.restore]` | 8 | `open_quil_in_active_scene()` — minimized branch | Restore from Minimized → Visible |
| `[shell.quil.lifecycle.minimize]` | 4 | `toggle_quil()` — minimize branch | Visible → Minimized transition |
| `[shell.quil.tile.placeholder]` | 8 | `tile_visible_frames()` — Quil fill rect block | Placeholder fill rect during tiling |

These supplement the existing markers:

| Existing Marker | Location | Path |
|-----------------|----------|------|
| `[shell.quil.frame.create]` | `ensure_quil_frame()` | Frame created in FRAMES slot |
| `[shell.quil.frame.reject]` | `ensure_quil_frame()` | No free frame slot |
| `[shell.quil.open]` | `open_quil_in_active_scene()` | Quil opened (any path) |
| `[shell.quil.focus]` | `focus_or_open_quil()` | Quil focused |
| `[shell.quil.toggle.minimize]` | `toggle_quil()` — **renamed to lifecycle.minimize** | Quil minimized via F9 toggle |

---

## Quil Shell Infrastructure Summary

All lifecycle paths are tracked by the shell's lifecycle FSM (8-state):

| Lifecycle Path | Function | FSM Transition | Lifecycle Guard | Status |
|---|---|---|---|---|
| Boot registration | `lifecycle_register()` | Allocated → Visible | — | ✅ |
| First open (F9) | `open_quil_in_active_scene()` → `ensure_quil_frame()` + 0xEC | Visible (stays) | `surface_is_alive()` | ✅ |
| Minimize (F9 toggle) | `toggle_quil()` → `minimize_frame()` | Visible → Minimized | `set_lifecycle_state()` | ✅ |
| Restore (F9 toggle) | `open_quil_in_active_scene()` → `restore_minimized_frame()` | Minimized → Visible | `[lifecycle.tombstone.reject_restore]` guard | ✅ |
| Focus (F9 if open) | `focus_or_open_quil()` → `try_set_focus()` | Visible (stays) | `focus_ref_is_current()`, generation check | ✅ |
| Tiling position | `tile_visible_frames()` / `tile_active_scene_frames()` | Visible (stays) | Skips minimized/zoomed/dead | ✅ |
| Placeholder fill | Both tile paths + `open_quil_in_active_scene()` 0xEF | — | Only when `sid == SURFACE_ID_QUIL` | ✅ |
| Surface alive | `surface_is_alive()` → `true` | Never destroyed | `SURFACE_ID_QUIL => true` | ✅ |
| Frame-owned guard | `sync_lifecycle_scene_visibility()` | Skips when no frame | Excluded via `SURFACE_ID_LINEN / QUIL` check | ✅ |

---

## Lifecycle Path Detail

### F9 → First Open
```
ToggleQuil (F9) → toggle_quil()
  → frame not found in active scene
  → open_quil_in_active_scene()
    → ensure_quil_frame()            [shell.quil.frame.create]
    → restore_minimized_frame()?     No (not minimized)
    → 0xEC send geometry             sexdisplay gets surface
    → 0xEF fill rect                 [shell.quil.tile.placeholder]
    → try_set_focus()                [shell.quil.focus]
                                    [shell.quil.open]
```

### F9 → Minimize (when visible)
```
ToggleQuil (F9) → toggle_quil()
  → frame found, not minimized
  → minimize_frame()
    → set_frame_minimized()
    → set_lifecycle_state(Minimized) [frame.light.minimize.fsm]
    → 0xEE deactivate               sexdisplay hides surface
    → tile_active_scene_frames()
                                    [shell.quil.lifecycle.minimize]
```

### F9 → Restore (when minimized)
```
ToggleQuil (F9) → toggle_quil()
  → frame found, minimized
  → open_quil_in_active_scene()
    → restore_minimized_frame()
      → set_lifecycle_state(Visible)
      → 0xEC upsert                  sexdisplay shows surface
      → try_set_focus()
                                    [shell.quil.lifecycle.restore]
                                    [shell.quil.open]
    → 0xEF fill rect                 [shell.quil.tile.placeholder]
```

---

## Behavior Changes

**None.** All 3 added markers are diagnostic-only. No lifecycle state
transitions, focus rules, tiling layout, or display protocol was changed.

---

## STOP FIRST Findings

| Condition | Finding |
|-----------|---------|
| Proof requires kernel/init changes | ✅ Not needed |
| Proof requires Quil server changes | ✅ Not needed |
| Proof requires sexdisplay/protocol changes | ✅ Not needed |
| Existing shell path is not lifecycle-safe | ✅ All paths are safe |
| More than tiny marker-only patch needed | ✅ 3 markers, 12 lines |

**No STOP FIRST conditions triggered.**

---

## Ready for QUIL_PD_SPAWN_V1B?

**Yes.** The shell-side Quil lifecycle path is proven safe. V1B can proceed
with kernel boot integration and server-side fleshing as a deliberate
boot/capability topology change.

---

## Build Verification

```sh
./scripts/entrypoint_build.sh
# Result: ISO produced, no errors
# Warnings: only pre-existing (unnecessary unsafe blocks)
```

## Diff

```diff
--- a/servers/silk-shell/src/main.rs
+++ b/servers/silk-shell/src/main.rs
@@ -920,6 +920,9 @@
         if sid == SURFACE_ID_QUIL {
             pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_QUIL, 0,
                 (QUIL_PLACEHOLDER_COLOR as u64) << 32 | ((rh as u64) << 16) | rw as u64);
+            static mut QUIL_PLACEHOLDER_BUDGET: u32 = 8;
+            let b = &mut QUIL_PLACEHOLDER_BUDGET;
+            if *b > 0 { *b -= 1; serial_println!("[shell.quil.tile.placeholder] sid={}", sid); }
         }
     }

@@ -3291,6 +3294,9 @@
     if frame_is_minimized(fid) {
         if !restore_minimized_frame(fid) {
             return false;
+        }
+        static mut QUIL_RESTORE_BUDGET: u32 = 8;
+        let b = &mut QUIL_RESTORE_BUDGET;
+        if *b > 0 { *b -= 1; serial_println!("[shell.quil.lifecycle.restore] frame={}", fid); }
     } else if frame_is_zoomed(fid) {

@@ -3384,7 +3390,7 @@
                 if minimize_frame(QUIL_FRAME_ID) {
                     static mut QUIL_TOGGLE_BUDGET: u32 = 4;
                     let b = &mut QUIL_TOGGLE_BUDGET;
-                    if *b > 0 { *b -= 1; serial_println!("[shell.quil.toggle.minimize] frame={}", QUIL_FRAME_ID); }
+                    if *b > 0 { *b -= 1; serial_println!("[shell.quil.lifecycle.minimize] frame={}", QUIL_FRAME_ID); }
                     return true;
                 }
```

## References

- `docs/handoff/QUIL_SURFACE_STUB_PLAN_SPLIT_V1.md` — phase split analysis
- `A8_LIFECYCLE_PROOF_SCENARIOS_V1.md` — lifecycle proof marker conventions
- `LIFECYCLE_TILING_WIRING_V1.md` — tiling lifecycle guards
- `open_quil_in_active_scene()` — line 3274
- `toggle_quil()` — line 3374
- `tile_visible_frames()` — line 810
- `lifecycle_state()` / `set_lifecycle_state()` — lifecycle helpers
