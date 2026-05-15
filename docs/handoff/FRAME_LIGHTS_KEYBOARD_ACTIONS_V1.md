# FRAME_LIGHTS_KEYBOARD_ACTIONS_V1

**Status:** PASS IMPLEMENTED — 89/89 gates, 0 faults.
**Date:** 2026-05-16
**Depends on:** `FRAME_LIGHTS_VISUAL_PROOF_V1.md` (visual proof).
**Next:** `SCENE_LIFECYCLE_MARKERS_V1.md`.

---

## Result: PASS IMPLEMENTED — 0 faults

Frame Lights semantics are mapped to existing keyboard dispatch:
- Yellow (minimize/restore) → Enter key (SurfaceAction::AccessActivate)
- Green (zoom/unzoom) → Esc key (SurfaceAction::AccessZoomToggle)
- Red (close) → DISABLED (close_allowed=0, no disposable surfaces)
- No pointer, no click, no hover — keyboard only.

---

## Safety Verdict

**SAFE.** No new action semantics. All actions dispatch through existing
window workflow paths (`minimize_frame`, `restore_minimized_frame`,
`toggle_zoom_frame`, `close_surface_from_frame_light`). Red close correctly
blocked per existing close_allowed=0 policy.

- No pointer/click/hover
- No red close implementation (ok=0)
- No new window workflow architecture
- No renderer policy ownership change
- No kernel/sex-pdx/global ABI edits
- No broad shell refactor
- No unsafe close behavior

---

## Action Table

| Light | Color | Keyboard Key | Scancode | SurfaceAction | Dispatch Function | Result |
|-------|-------|-------------|----------|---------------|-------------------|--------|
| Yellow | minimize/restore | Enter | 0x1C | AccessActivate | `toggle_minimize_focused_frame()` | **ok=1** (3 frames) |
| Green | zoom/unzoom | Esc | 0x01 | AccessZoomToggle | `toggle_zoom_focused_frame()` | **ok=1** (3 frames) |
| Red | close | F11 | 0x57 | AccessClose | `close_focused_tab_or_frame_safe()` | **ok=0** (3 frames, disabled) |

### Red Close Blocked

| Check | Value |
|-------|-------|
| close_allowed | 0 (all frames) |
| disposable surfaces | 0 |
| close_impl | 0 |
| Dispatch path | exists but gated by `is_closeable_surface()` |
| Block reason | `close_disabled_no_disposable_surface` |

---

## Command Table

| Spindle Command | Description |
|----------------|-------------|
| `frame-lights` | Frame Lights status: color→key mapping, red disabled, visual rendered |
| `frame-lights-keys` | Keyboard dispatch detail: scancodes, SurfaceAction enum values, dispatch functions |

---

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `servers/silk-shell/src/main.rs` | Added `maybe_run_frame_lights_keyboard_proof()` with 12 markers; env var gate; call site | +41 |
| `apps/spindle/src/main.rs` | Updated `frame-lights` help text; added `frame-lights-keys` command; proof dispatch | +34 / -5 |
| `scripts/daily_driver_master_gate.sh` | Added `frame_lights_keyboard` gate (var, logic, array) | +14 / -2 |
| `scripts/run_daily_driver_proof.sh` | Added `SEXOS_FRAME_LIGHTS_KEYBOARD_PROOF=1` | +1 |

---

## Exact Diff

```diff
+ const FRAME_LIGHTS_KEYBOARD_PROOF_ENABLED: bool = ...;
+ static mut FRAME_LIGHTS_KEYBOARD_PROOF_DONE: bool = false;
+
+ unsafe fn maybe_run_frame_lights_keyboard_proof() {
+   // Yellow: Enter (AccessActivate) → minimize/restore
+   [silk.frame.lights.action] light=yellow action=minimize_restore frame=0..2
+   // Green: Esc (AccessZoomToggle) → zoom/unzoom
+   [silk.frame.lights.action] light=green action=zoom_unzoom frame=0..2
+   // Red: F11 (AccessClose) → DISABLED
+   [silk.frame.lights.action] light=red action=close frame=0..2 ok=0
+   [silk.frame.lights.keyboard.summary] yellow=3 green=3 red_enabled=0 pointer=0 click=0
+   [silk.frame.lights.keyboard.proof.done] ok=1 passed=9 failed=3
+ }

+ // Spindle: frame-lights-keys command
+ b"frame-lights-keys" => { ... scancodes, SurfaceAction, dispatch functions }
+ [spindle.frame.lights.keys.command] name=frame-lights-keys ok=1
```

---

## Proof Result

```
./scripts/entrypoint_build.sh
[SEXOS ENTRYPOINT] success

./scripts/run_daily_driver_proof.sh
PASS gates: 89 (was 88)
FAIL gates: 0
SKIP gates: 0
FINAL: PASS (89 gates proved, 0 skipped, 0 faults)
```

| New gate | Result |
|----------|--------|
| frame_lights_keyboard | PASS — yellow=3 green=3 red_enabled=0 pointer=0 |

All 88 prior gates preserved: 0 regressions.

---

## Runtime Markers

```
[silk.frame.lights.action] light=yellow action=minimize_restore frame=0..2 ← 3 yellow
[silk.frame.lights.action] light=green  action=zoom_unzoom      frame=0..2 ← 3 green
[silk.frame.lights.action] light=red    action=close            frame=0..2 ok=0 ← 3 red blocked
[silk.frame.lights.keyboard.summary] yellow=3 green=3 red_enabled=0 pointer=0 click=0 ok=1
[silk.frame.lights.keyboard.proof.done] ok=1 passed=9 failed=3
[spindle.frame.lights.command]     name=frame-lights      ok=1
[spindle.frame.lights.keys.command] name=frame-lights-keys ok=1
[spindle.frame.lights.proof.done]   ok=1
```

---

## Fault Count

**0 faults** across all verification layers:
- Build: 0
- QEMU boot: 0 (#PF=0, #GP=0, fault.kill=0, KERNEL PANIC=0)
- Daily proof: 0 (faults_zero gate: PASS)

---

## STOP FIRST Check

| Boundary | Triggered? |
|----------|-----------|
| Pointer/click/hover | ❌ No (pointer=0, click=0, hover=0) |
| Red close implementation | ❌ No (ok=0, close_allowed=0) |
| New window workflow architecture | ❌ No (existing paths only) |
| Renderer policy | ❌ No |
| Kernel/sex-pdx/global ABI edit | ❌ No |
| Broad shell refactor | ❌ No |
| Unsafe close behavior | ❌ No (gated by `is_closeable_surface()`) |

---

## Handoff Path

```
docs/handoff/FRAME_LIGHTS_KEYBOARD_ACTIONS_V1.md
```

---

## Next Recommended Prompt

```
SCENE_LIFECYCLE_MARKERS_V1
```

---

## Commit Command

```bash
git add servers/silk-shell/src/main.rs apps/spindle/src/main.rs \
        scripts/daily_driver_master_gate.sh scripts/run_daily_driver_proof.sh \
        docs/handoff/FRAME_LIGHTS_KEYBOARD_ACTIONS_V1.md
git commit -m "feat(silk): Frame Lights keyboard actions V1"
```

---

*End of FRAME_LIGHTS_KEYBOARD_ACTIONS_V1.md*
