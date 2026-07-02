# D5_ACCESSIBILITY_NEGATIVE_TESTS_V1

**Status:** Complete — audit only, no code changes.
**Build:** ISO produced, no errors.
**Track D freeze:** ✅ Ready to freeze.

---

## Summary

Negative test / contradiction audit for Track D accessibility. Verifies that all
D1–D4 accessibility paths reject unsafe targets, use existing lifecycle-safe
helpers, respect renderer ownership, preserve app/editor boundaries, and do not
bypass shell policy.

All 12 negative cases pass. No STOP FIRST conditions triggered. No code changes
needed. Track D is safe to freeze.

---

## Files Inspected

| File | Lines | Role |
|------|-------|------|
| `servers/silk-shell/src/main.rs` | ~7200 | All D accessibility implementation |
| `servers/sexdisplay/src/main.rs` | ~1230 | Renderer (no accessibility semantics) |
| `servers/quil/src/main.rs` | ~59 | Editor stub (no accessibility refs) |
| `docs/D_ACCESSIBILITY_STACK_PLAN_V1.md` | 280 | Negative test spec (section 15) |
| `docs/handoff/D1_D4_*` | — | Track D handoff docs |

---

## Negative Cases

### 1. Tombstoned/destroyed/dead surfaces are skipped by node emission

| Property | Value |
|----------|-------|
| **Guard** | `access_node_is_valid_target()` checks `surface_is_alive()` + `!is_tombstoned()` |
| **Reject marker** | `[access.node.skip_dead]` (line 1601) |
| **Code** | `servers/silk-shell/src/main.rs:1533` |
| **Verdict** | ✅ PASS — dead surfaces excluded from semantic tree |

### 2. Dead targets cannot receive focus/action

| Property | Value |
|----------|-------|
| **Guards** | `try_set_focus()` checks `is_focusable_surface()`, `surface_is_alive()`, `is_tombstoned()`, `surface_is_lifecycle_focusable()` |
| **Reject markers** | `[access.action.reject] action=close reason=dead`, `action=activate reason=dead`, `action=zoom reason=dead` |
| **Code** | Lines 909, 1057-1068, 1197-1199, 1784, 1835, 1862 |
| **Verdict** | ✅ PASS — all D3/D3B actions reject dead targets before dispatch |

### 3. Inactive scene frame actions rejected unless scene switch

| Property | Value |
|----------|-------|
| **Guard** | `surface_in_active_scene()` + `clear_focus_if_wrong_scene()` after scene switch |
| **Code** | `surface_in_active_scene()` line 2710, `try_set_focus()` scene validation at line 2734 |
| **Verdict** | ✅ PASS — `try_set_focus()` checks scene ownership before accepting focus |

### 4. Minimized/visible restore/minimize use existing helpers only

| Property | Value |
|----------|-------|
| **Helpers used** | `minimize_frame()` (line 3847), `restore_minimized_frame()` (line 3763) |
| **Dispatch** | `access_handle_keyboard_action()` → `AccessActivate` → minimize/restore |
| **Verdict** | ✅ PASS — no direct state mutation, all through existing lifecycle helpers |

### 5. Close uses existing lifecycle-safe close path only

| Property | Value |
|----------|-------|
| **Helper used** | `close_surface_from_frame_light()` (line 3986) |
| **Dispatch** | `access_handle_keyboard_action()` → `AccessClose` → close helper |
| **Lifecycle FSM** | Closing → Tombstoned → Destroyed with tombstone recording |
| **Verdict** | ✅ PASS — full lifecycle-safe close path, no shortcuts |

### 6. Zoom uses existing zoom/unzoom helper only

| Property | Value |
|----------|-------|
| **Helper used** | `toggle_zoom_frame()` → `zoom_frame()` / `unzoom_frame()` |
| **Dispatch** | `access_handle_keyboard_action()` → `AccessZoomToggle` → toggle helper |
| **Lifecycle guard** | REJECTS lifecycle states: Closing, Tombstoned, Destroyed |
| **Verdict** | ✅ PASS — full lifecycle-safe zoom toggle, no shortcuts |

### 7. Focus description never logs app text/user content/document names

| Property | Value |
|----------|-------|
| **D4 description** | Numeric tokens only: role ID (u8), state flags (u16 hex), action flags (u16 hex), target IDs, label hash (u32) |
| **Label privacy invariant** | Only shell-owned static labels hashed. No plaintext app text. |
| **Verdict** | ✅ PASS — D4 handoff doc explicitly states label privacy invariant. No app content logged. |

### 8. Label hash is not treated as a secrecy boundary

| Property | Value |
|----------|-------|
| **Wording** | D4 handoff corrected: "The label hash is deterministic and avoids printing raw labels, but it is not a secrecy boundary. Small/static label spaces may be guessed offline. Do not hash private app/user content." |
| **Verdict** | ✅ PASS — corrected from earlier "cannot be reversed" claim. |

### 9. sexdisplay owns zero accessibility policy

| Property | Value |
|----------|-------|
| **Evidence** | `grep -c "accessibil\|semantic\|narrat\|focus" servers/sexdisplay/src/main.rs` returns 19 |
| **Matches** | All 19 are compositor rendering references (composite order, focus surface color). Zero accessibility semantics. |
| **Verdict** | ✅ PASS — sexdisplay only renders shell state. No semantic inference from pixels. |

### 10. Quil editor/app input remains untouched

| Property | Value |
|----------|-------|
| **Evidence** | Quil imports zero `SLOT_*` constants. Only listens on slot 0. Handles only `OP_QUIL_PING`. |
| **Caps** | No `SLOT_DISPLAY`, no `SLOT_INPUT`, no `SLOT_SHELL`. Shell→Quil one-way cap only. |
| **Verdict** | ✅ PASS — Quil has no keyboard input path, no display access, no surface lifecycle authority. |

### 11. Esc zoom binding gated by normal shell mode; Atlas intercept wins first

| Property | Value |
|----------|-------|
| **Dispatch order** | Atlas intercept (line 7111): `if ATLAS_MODE_ENABLED && scancode != 0x44` fires BEFORE `scancode_to_action()` (line 7114) |
| **Atlas mode** | Esc (0x01) → `handle_atlas_keyboard()` → cancel/exit Atlas |
| **Normal mode** | Esc → `scancode_to_action()` → `AccessZoomToggle` → `toggle_zoom_frame()` |
| **Esc binding invariant** | Documented in D3B handoff: "If future app/editor input receives Esc, this binding must move behind a shell modifier or mode gate." |
| **Verdict** | ✅ PASS — Atlas intercept always wins first. Normal-mode Esc zoom only fires when Atlas is closed. |

### 12. SceneNext/ScenePrev helpers remain unbound until safe modifier/key model exists

| Property | Value |
|----------|-------|
| **SurfaceAction** | `AccessSceneNext`, `AccessScenePrev` defined (lines 722-723) |
| **Dispatch** | Implemented in `access_handle_keyboard_action()` (lines 1880-1894) |
| **Binding** | Unbound — no scancode maps to these variants |
| **Rationale** | No safe single-key scancode available. All F-keys used. Number keys conflict with Focus100-Focus200. Letter keys conflict with future editor input. Requires modifier tracking (Ctrl+Tab) which doesn't exist in scancode-only model. |
| **Verdict** | ✅ PASS — dispatch helpers exist but unbound. Safe by design. |

---

## Pass/Fail Matrix

| # | Test | Verdict | Evidence |
|---|------|---------|----------|
| 1 | Tombstoned/dead skip node emission | ✅ PASS | `[access.node.skip_dead]` marker present |
| 2 | Dead targets reject focus/action | ✅ PASS | `[access.action.reject] reason=dead` markers (3 paths) |
| 3 | Inactive scene reject | ✅ PASS | `surface_in_active_scene()` + `clear_focus_if_wrong_scene()` |
| 4 | Minimize/restore existing helpers | ✅ PASS | `minimize_frame()` / `restore_minimized_frame()` only |
| 5 | Close lifecycle-safe path | ✅ PASS | `close_surface_from_frame_light()` only |
| 6 | Zoom lifecycle-safe helper | ✅ PASS | `toggle_zoom_frame()` only |
| 7 | No app text in description | ✅ PASS | Numeric tokens only, label privacy invariant |
| 8 | Label hash not secrecy boundary | ✅ PASS | Handoff corrected, explicit warning added |
| 9 | sexdisplay no accessibility policy | ✅ PASS | Zero accessibility semantics, only compositor rendering |
| 10 | Quil app input untouched | ✅ PASS | No SLOT_* caps, no keyboard path, one-way OK |
| 11 | Esc binding gated by normal mode | ✅ PASS | Atlas intercept fires before scancode dispatch |
| 12 | SceneNext/ScenePrev unbound | ✅ PASS | Dispatch helpers exist, no bindings |

**Result:** 12/12 PASS. No contradictions found.

---

## Docs Wording Corrections

One correction applied to D4 handoff doc during this audit:

| Doc | Before | After |
|-----|--------|-------|
| `D4_ACCESSIBILITY_FOCUS_DESCRIPTION_PROOF_V1.md` | "cannot be reversed to recover the original label" | "not a secrecy boundary. Small/static label spaces may be guessed offline. Do not hash private app/user content." |

This was the only wording correction needed across all D1–D5 docs.

---

## Build Verification

```sh
$ ./scripts/entrypoint_build.sh
ISO image produced: 1575 sectors
[SEXOS ENTRYPOINT] success
```

---

## Track D Freeze Verdict

**✅ Track D is safe to freeze.**

| Phase | Status | Notes |
|-------|--------|-------|
| D1 audit/spec | ✅ Complete | Shell semantics inventory |
| D2 semantic node emitter | ✅ Complete | Bounded, no-heap AccessNode tree |
| D3 keyboard focus/minimize/restore | ✅ Complete | Tab/Backspace/Enter via semantic tree |
| D3B close/zoom completion | ✅ Complete | F11 close, Esc zoom, SceneNext/Prev unbound |
| D4 focus description proof | ✅ Complete | Numeric tokens, label hash, privacy invariant |
| D5 negative tests | ✅ Complete | 12/12 pass, no contradictions |

**Remaining deferred items (no freeze blocker):**

| Item | Reason | Future track |
|------|--------|-------------|
| SceneNext/ScenePrev bindings | Requires modifier tracking (Ctrl+Tab) | Future keyboard model |
| Atlas settings cycle accent / toggle pin | Already handled by `handle_atlas_keyboard()` | Already covered |
| SurfaceAction::FocusToggle dead code | No scancode maps to it | Remove in cleanup |
| Full AT-SPI/DBus parity | Out of scope for SexOS | Never |
| Narrator/speech/audio | Out of scope for V1 | Future Bell phase |

---

## References

- `docs/D_ACCESSIBILITY_STACK_PLAN_V1.md` — §15 negative test spec
- `docs/handoff/D1_ACCESSIBILITY_SHELL_SEMANTICS_AUDIT_V1.md` — D1 audit
- `docs/handoff/D4_ACCESSIBILITY_FOCUS_DESCRIPTION_PROOF_V1.md` — D4 (corrected)
- `docs/handoff/D3B_ACCESSIBILITY_KEYBOARD_ACTIONS_COMPLETE_V1.md` — Esc binding invariant
- `docs/handoff/A8_LIFECYCLE_PROOF_SCENARIOS_V1.md` — Lifecycle FSM proof
