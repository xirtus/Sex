# D3_ACCESSIBILITY_KEYBOARD_ACTIONS_V1

**Status:** Complete — built, committed.
**Build:** ISO produced, no errors.

---

## Summary

Implements keyboard alternatives for shell navigation using the D2 semantic
node tree. No narrator, no speech, no app/editor input.

**D3 is partial** — focus traversal and minimize/restore activation are
implemented. Close, zoom/unzoom, scene switch, and Atlas keyboard alternatives
are deferred to future phases.

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | +12 lines (3 match arms in EV_KEY dispatch) |
| `docs/handoff/D3_ACCESSIBILITY_KEYBOARD_ACTIONS_V1.md` | New handoff doc |

---

## D3 Implemented

### Keybinding Map

| Key | Scancode | Action | Dispatch |
|-----|----------|--------|----------|
| Tab | 0x0F | `AccessFocusNext` | `access_handle_keyboard_action()` → semantic tree scan → `try_set_focus()` |
| Backspace | 0x0E | `AccessFocusPrev` | Same, reverse scan |
| Enter | 0x1C | `AccessActivate` | `access_handle_keyboard_action()` → `minimize_frame()` / `restore_minimized_frame()` |

### Focus Traversal Algorithm

```
access_handle_keyboard_action(AccessFocusNext/AccessFocusPrev):
  1. Build semantic tree via access_emit_shell_nodes() → [Option<AccessNode>; 64]
  2. Find current FOCUSED_SURFACE_ID position in tree
  3. Scan forward/backward with wrapping (1..len offset)
  4. For each candidate node:
     - Validate sid != 0
     - surface_is_alive(sid)
     - !is_tombstoned(sid)
     - surface_is_lifecycle_focusable(sid)
  5. On first valid candidate: try_set_focus(sid)
  6. Budgeted marker: [access.action.focus_next]
  7. If no candidates: [access.action.reject] reason=no_targets
```

### Activate Dispatch

```
access_handle_keyboard_action(AccessActivate):
  1. Validate FOCUSED_SURFACE_ID != 0
  2. Validate surface_is_alive(sid) && !is_tombstoned(sid)
  3. Find frame via frame_for_surface(sid)
  4. If frame exists:
     - If minimized → restore_minimized_frame(frame_id)
       [access.action.allow] dispatch=restore
     - If visible → minimize_frame(frame_id)
       [access.action.allow] dispatch=minimize
  5. Non-frame surfaces (placeholders, panels): no-op (already focused)
  6. Marker: [access.keyboard.alt] on success
  7. Budgeted reject markers on failure: [access.action.reject] reason=*
```

---

## D3 Deferred (Future Phases)

| Feature | Reason for Deferral |
|---------|---------------------|
| Close focused frame/tab | Requires close confirmation UX (not yet designed) |
| Zoom/unzoom focused frame | Zoom state affects frame geometry in ways not yet safe to trigger from keyboard alone |
| Scene switch alternative | Scene switch currently requires pointer click on scene chip or silkbar workspace |
| Atlas settings keyboard action parity | Atlas keyboard intercept (F10) exists but full action parity deferred |

---

## Lifecycle Safety Verification

All D3 keyboard actions route through existing lifecycle-safe functions:

| Action | Path | Lifecycle guard |
|--------|------|----------------|
| FocusNext | `access_emit_shell_nodes()` → scan → `try_set_focus(sid)` | `surface_is_alive()` + `!is_tombstoned()` + `surface_is_lifecycle_focusable()` |
| FocusPrev | Same with reverse scan | Same |
| Activate (minimized) | `restore_minimized_frame(frame_id)` | Existing lifecycle-safe restore |
| Activate (visible) | `minimize_frame(frame_id)` | Existing lifecycle-safe minimize |

**No new lifecycle transitions.** No direct state mutation. All paths go through
existing helper functions that are already lifecycle-safe.

---

## Proof Markers Added

| Marker | Budget | Location | When |
|--------|--------|----------|------|
| `[access.action.focus_next]` | 8 | `access_handle_keyboard_action()` | Successful focus next/prev, logs from→to sid + role + label |
| `[access.action.reject]` | 4 | Same | Action rejected (no focus, dead target, no candidates) |
| `[access.action.allow]` | 4 | Same | Activate dispatched (restore or minimize) |
| `[access.keyboard.alt]` | 8 | Same | Activate succeeded on non-frame surface (no-op) |

---

## Behavior Changes

**Minimal.** When Tab/Backspace/Enter is pressed:

- Tab: shifts focus to next alive surface in the semantic tree (wrapping)
- Backspace: shifts focus to previous alive surface (wrapping)
- Enter: minimizes a visible frame, restores a minimized frame

Previous behavior:
- Tab was `FocusToggle`: hardcoded cycle through 5 surface IDs (100→101→102→103→200→100)
- Backspace was unmapped (no action)
- Enter was unmapped (no action)

---

## STOP FIRST Findings

| Condition | Finding |
|-----------|---------|
| Requires heap/String/broad refactor | ✅ Not needed — reuses D2 stack-allocated tree |
| Requires app/editor input | ✅ Not added — shell chrome only |
| Requires narrator/speech | ✅ Not added — keyboard dispatch only |
| Requires kernel/ABI change | ✅ Not needed |
| Requires sexdisplay semantics ownership | ✅ Not needed — dispatch through existing shell functions |
| Bypasses lifecycle-safe paths | ✅ No — all paths validated in lifecycle safety table |
| Requires persistence/storage | ✅ Not needed |
| Directly mutates lifecycle/focus state | ✅ No — always through `try_set_focus()`, `minimize_frame()`, `restore_minimized_frame()` |

**No STOP FIRST conditions triggered.**

---

## Ready for D4+

**Partially.** The focus traversal and activate actions are wired. Future D
phases can add close, zoom/unzoom, scene switch, and Atlas keyboard
alternatives by extending the `access_handle_keyboard_action()` dispatch
table at the same match point.

---

## References

- `docs/handoff/D2_ACCESSIBILITY_SEMANTIC_NODE_EMITTER_V1.md` — D2 node model
- `docs/handoff/D1_ACCESSIBILITY_SHELL_SEMANTICS_AUDIT_V1.md` — D1 audit
- `docs/D_ACCESSIBILITY_STACK_PLAN_V1.md` — Track D plan
- `servers/silk-shell/src/main.rs` — implementation (~12 lines added)
