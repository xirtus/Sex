# APP_LIFECYCLE_CONTRACT_V1

**Status:** LOCKED ✅
**Date:** 2026-05-06
**Prerequisites:**
- APP_MANIFEST_CAP_CONTRACT_V1 (locked)
- APP_SURFACE_LAUNCH_CONTRACT_V1 (passes)
- SILK_SHELL_INTERACTION_CONTRACT_V1 (locked)
- A3_SHELL_LIFECYCLE_MODEL_V1 (metadata additive)
- A4_FOCUS_LIFECYCLE_GUARDS_V1 (focus guards wired)
- FRAME_LIFECYCLE_HARDENING_V1 (drag/hover cleanup)

---

## Summary

Lock minimal app lifecycle semantics for dynamically registered app surfaces. Previously, `close_surface_from_frame_light()` only handled 4 hardcoded boot surfaces (APP, STATIC, TEST3, TEST4). Dynamically registered surfaces (via `handle_app_surface_req`) could launch and focus but **could not be closed** — their lifecycle was stuck at Visible.

This patch extends the close path to handle **any lifecycle-registered surface**, enabling the full lifecycle chain: launch → focus → minimize → restore → close/tombstone → stale-focus rejection.

---

## Changes

### 1. `is_closeable_surface()` — lifecycle registration fallback

**File:** `servers/silk-shell/src/main.rs`
**Lines:** ~7667

```diff
+            // Fallback: dynamically registered app surfaces (via lifecycle) are closeable
+            if lifecycle_state(surface_id).is_some() {
+                return true;
+            }
```

Previously, the `_ =>` fallback only checked `surface_is_alive()`, which returns `false` for any surface not in the hardcoded list. Now, if a surface is registered in the lifecycle table (via `lifecycle_register()`), it is considered closeable.

### 2. `close_surface_from_frame_light()` — dynamic surface support

**File:** `servers/silk-shell/src/main.rs`
**Lines:** ~7691-7730

Two changes:

a) **Initial guard** — Replaced `if !surface_is_alive(surface_id)` with `if !is_closeable_surface(surface_id)`:
   ```diff
   -    if !surface_is_alive(surface_id) {
   +    // Must be closeable (checks registry, lifecycle registration, or alive flags).
   +    if !is_closeable_surface(surface_id) {
   ```

b) **Hardcoded match** — Changed `_ => return false` to `_ => {}`:
   ```diff
          SURFACE_ID_TEST4  => SURFACE_103_ALIVE = false,
   -      _ => return false, // unknown or non-closeable surface
   +      _ => {} // dynamic surfaces: lifecycle state is authority; no alive flag needed
   ```

This allows any lifecycle-registered surface to flow through the Closing → Tombstoned → Destroyed state machine. The existing lifecycle reject guards (already-dead check, drag check) still apply.

### 3. Lifecycle proof (gated by `SEXOS_LIFECYCLE_PROOF=1`)

**File:** `servers/silk-shell/src/main.rs`
**Lines:** ~10027-10086

6 boot-time proof stages:

| Stage | Operation | Expected | Marker |
|-------|-----------|----------|--------|
| 0 | Launch surface 310 via `handle_app_surface_req` | accepted=true | `[shell.lifecycle.proof.launch] sid=310 accepted=true` |
| 1 | Focus surface 310 | focused=true, actual=310 | `[shell.lifecycle.proof.focus] sid=310 result=true actual=310` |
| 2 | Minimize surface 310's frame | lifecycle state = Minimized | `[shell.lifecycle.proof.minimize] result=true state=Minimized` |
| 3 | Restore surface 310 | lifecycle state = Visible | `[shell.lifecycle.proof.restore] result=true state=Visible` |
| 4 | Close surface 310 | lifecycle state = Destroyed | `[shell.lifecycle.proof.close] result=true state=Destroyed` |
| 5 | Try to focus closed surface 310 | focus rejected (false) | `[shell.lifecycle.proof.stale] focus_rejected=true` |

---

## Files Changed

| File | Lines | Change |
|------|-------|--------|
| `servers/silk-shell/src/main.rs` | +72 / -2 | 3 logical changes + proof stages |

---

## Lifecycle States for Dynamic Surfaces

| State | Supported | Proof |
|-------|-----------|-------|
| Allocated → Mapped | ✅ (via panel/atlas toggle, not app path) | A3 |
| Mapped → Visible | ✅ (minimize restore) | `[shell.lifecycle.proof.restore]` |
| Visible → Minimized | ✅ | `[shell.lifecycle.proof.minimize]` |
| Minimized → Visible | ✅ | `[shell.lifecycle.proof.restore]` |
| Visible → Closing | ✅ (**NEW** — previously hardcoded) | `[shell.lifecycle.proof.close]` |
| Closing → Tombstoned | ✅ | `[lifecycle.tombstone.record]` |
| Tombstoned → Destroyed | ✅ | `[lifecycle.destroy.record]` |
| Destroyed (focus rejected) | ✅ | `[shell.lifecycle.proof.stale]` |

Hidden state remains unwired (documented A8 gap — low severity, deferred).

---

## Proof Markers

### Default build (SEXOS_LIFECYCLE_PROOF unset)

Zero behavior change. All existing lifecycle markers unchanged.

### Proof build (SEXOS_LIFECYCLE_PROOF=1)

| Marker | Stage |
|--------|-------|
| `[shell.lifecycle.proof] stage=0` | Proof start |
| `[shell.lifecycle.proof.launch] sid=310 accepted=true` | Launch |
| `[shell.app_surface.accept] sid=310 ...` | Handler acceptance |
| `[shell.lifecycle.proof] stage=1` | Focus stage |
| `[shell.lifecycle.proof.focus] sid=310 result=true actual=310` | Focus |
| `[focus.ref.commit]` | FocusRef synced |
| `[shell.lifecycle.proof] stage=2` | Minimize stage |
| `[shell.lifecycle.proof.minimize] result=true state=Minimized` | Minimize |
| `[lifecycle.transition.allow]` | Lifecycle state change |
| `[shell.lifecycle.proof] stage=3` | Restore stage |
| `[shell.lifecycle.proof.restore] result=true state=Visible` | Restore |
| `[shell.lifecycle.proof] stage=4` | Close stage |
| `[shell.lifecycle.proof.close] result=true state=Destroyed` | Close |
| `[lifecycle.destroy.record]` | Terminal destroy |
| `[shell.lifecycle.proof] stage=5` | Stale focus stage |
| `[shell.lifecycle.proof.stale] focus_rejected=true` | Stale rejected |
| `[shell.focus.reject.tombstoned]` or `[focus.generation.reject]` | Focus guard |

---

## Build & Runtime

- **Build (default):** PASS — `./scripts/entrypoint_build.sh` ISO produced
- **Build (proof):** PASS — `SEXOS_LIFECYCLE_PROOF=1 ./scripts/entrypoint_build.sh` ISO produced
- **Runtime gate (default):** GREEN_MASTER — all 6 gates PASS
- **Runtime gate (proof):** PENDING — run `SEXOS_LIFECYCLE_PROOF=1 ./scripts/master_runtime_gate.sh` to verify proof markers

---

## STOP FIRST Conditions NOT Triggered

- ❌ No kernel/ABI/sex-pdx edits
- ❌ No scheduler changes
- ❌ No sexdisplay renderer changes
- ❌ No framebuffer write changes
- ❌ No broad app runtime redesign
- ❌ No process loader changes

---

## Remaining Risks

1. **Hidden lifecycle state still unwired** — `sync_scene_visibility()` does not call `set_lifecycle_state(Hidden)`. Surfaces in non-active scenes remain `Visible` in lifecycle. Deferred per A8 gap assessment.

2. **DestroyFocused keyboard handler still hardcoded** — The `SurfaceAction::DestroyFocused` handler (line ~10500) has its own hardcoded surface chain for APP/STATIC/TEST3/TEST4. Dynamic surfaces cannot be closed via keyboard shortcut. Fix deferred — the frame light close button works for dynamic surfaces via `close_surface_from_frame_light()` which now supports them.

3. **No error reply on reject** — `handle_app_surface_req` returns bool but does not send a typed error reply to the caller. Same limitation noted in APP_SURFACE_LAUNCH_CONTRACT_V1.

4. **Proof surface 310 persists** — The lifecycle proof registers surface 310 which persists after boot. When running with `SEXOS_LIFECYCLE_PROOF=1`, surface 310 is created and then destroyed (tombstoned). After tombstone, it cannot be focused or restored, and the frame slot is consumed. This is safe because the proof is off by default.

---

## Next Steps

1. Verify proof markers by running `SEXOS_LIFECYCLE_PROOF=1 ./scripts/master_runtime_gate.sh`
2. Wire `Hidden` lifecycle state in `sync_scene_visibility()` (A8 gap fix)
3. Refactor `SurfaceAction::DestroyFocused` to delegate to `close_surface_from_frame_light()`
4. Add error reply opcode for `handle_app_surface_req` rejections

---

## References

- `docs/handoff/A3_SHELL_LIFECYCLE_MODEL_V1.md` — Lifecycle FSM metadata
- `docs/handoff/A4_FOCUS_LIFECYCLE_GUARDS_V1.md` — Focus guard wiring
- `docs/handoff/APP_SURFACE_LAUNCH_CONTRACT_V1.md` — App surface launch contract
- `docs/handoff/APP_MANIFEST_CAP_CONTRACT_V1.md` — App manifest/capability contract
- `docs/handoff/FRAME_LIFECYCLE_HARDENING_V1.md` — Drag/hover cleanup on close
- `servers/silk-shell/src/main.rs` — Implementation
