# A2_COMPOSITOR_LIFECYCLE_FSM_SPEC_V1

**Status:** Handoff/spec only. No code changed.
**Date:** 2026-05-04
**Purpose:** Define the exact 8-state lifecycle FSM that A3/A4 will implement in silk-shell. Resolve all A1 audit findings into concrete FSM design decisions.

---

## 1. Executive Summary

This spec defines the canonical 8-state lifecycle FSM (Allocated → Mapped → Visible → Hidden/Minimized → Closing → Tombstoned → Destroyed) that A3 will implement. Focus is separate from lifecycle state as a `focused_surface: Option<FocusRef>` with a 5-guard validation chain. LifecycleGeneration is required as a monotonic u64 counter to detect stale references. SurfaceId/FrameId hardcoded constants are preserved temporarily; A3 must add generation safety before any dynamic allocation. The 0xEE opcode collision (destroy vs hide) must be resolved as a protocol audit in A5/A7, not in A2. All 20 proof markers from the A doc are specified with transition mappings.

---

## 2. Inputs from A1

| Finding | A2 Resolution |
|---------|---------------|
| No explicit FSM | Canonical 8-state enum defined in §3 |
| Close skips Closing→Tombstoned→Destroyed | Transition table enforces exact chain (§4) |
| No LifecycleGeneration | Required monotonic u64 counter (§7) |
| No FocusRef | `Option<(u64, u64)>` = (SurfaceId, LifecycleGeneration) (§6) |
| No caller identity check | To be implemented in A4 with shell-internal flag (§8) |
| try_set_focus() lacks minimized check | A4 must add `frame_accepts_input()` guard (§8) |
| 0xEE used for both destroy and minimize | Protocol audit deferred to A5/A7; A2 only specifies semantics (§14) |
| WINDOWS: Vec is heap-backed | Preserve temporarily; A3 adds static tracking alongside Vec, migrate in A5 |
| No lifecycle proof markers | All 20 markers specified with transition mapping (§15) |
| No generation safety for SurfaceId reuse | Hardcoded IDs preserved; A6 adds generation to tombstone records |

---

## 3. Canonical FSM State Definitions

```rust
/// Lifecycle state for a surface managed by the compositor.
/// Focus is NOT a lifecycle state — it is tracked separately.
enum LifecycleState {
    /// SurfaceId reserved but no frame mapped. No display state.
    Allocated,
    /// Surface attached to a Frame. Has geometry but may not be visible
    /// (behind other surfaces, off-screen, non-active scene).
    Mapped,
    /// Surface in active scene, frame not minimized, z-order includes it.
    /// Receives input events if focused.
    Visible,
    /// Surface's frame is in a non-active scene.
    /// Geometry exists but no input routing.
    Hidden,
    /// Frame collapsed to Atlas/SilkBar. Surface hidden via 0xEE hide.
    /// No pointer focus.
    Minimized,
    /// Close requested. Guards check closeable, alive, non-tombstoned.
    /// Transition to Tombstoned is the only allowed outgoing transition.
    Closing,
    /// Surface dead but record exists. Cannot receive focus.
    /// Holds LifecycleGeneration, timestamp, reason for debugging.
    /// Not live content — cannot be rendered or interacted with.
    Tombstoned,
    /// Terminal. SurfaceId eligible for reuse only after generation safety.
    /// No transition out.
    Destroyed,
}
```

### State Semantics

| State | Displayed | Focusable | Accepts Input | Frame Required | Notes |
|-------|-----------|-----------|---------------|----------------|-------|
| Allocated | No | No | No | No | Reserved ID only |
| Mapped | Maybe (z-order) | Only if Visible | No | Yes | Behind other surfaces |
| Visible | Yes | Yes | Yes | Yes | Active scene, not minimized |
| Hidden | No (wrong scene) | No | No | Yes | Scene switch |
| Minimized | No | No | No | Yes | 0xEE hide sent |
| Closing | No | No | No | Yes | Irreversible |
| Tombstoned | No (record only) | No | No | No | Debug/atlas record |
| Destroyed | No | No | No | No | Terminal |

---

## 4. Allowed Transition Table

| From | To | Trigger | Guard | Proof Marker |
|------|----|---------|-------|-------------|
| Allocated | Mapped | Frame attach | SurfaceId valid, frame slot available | `[comp.surface.map]` |
| Mapped | Visible | Scene activation, un-minimize | Frame in active scene, not minimized | `[comp.surface.visible]` |
| Mapped | Closing | Close request | Surface closeable, alive, non-tombstoned | `[comp.surface.close]` |
| Visible | Hidden | Scene switch | New scene active, surface's scene inactive | `[comp.surface.hide]` |
| Visible | Minimized | Minimize action | Frame not already minimized | `[comp.surface.minimize]` |
| Visible | Closing | Close request | Same as Mapped→Closing | `[comp.surface.close]` |
| Hidden | Visible | Scene switch back | Surface's scene becomes active | `[comp.surface.visible]` |
| Hidden | Closing | Close request | Same as Mapped→Closing | `[comp.surface.close]` |
| Minimized | Visible | Restore | Frame un-minimized, alive, non-tombstoned | `[comp.surface.visible]` |
| Minimized | Closing | Close request | Same | `[comp.surface.close]` |
| Closing | Tombstoned | Tombstone record created | Generation incremented, drag cancelled | `[comp.surface.tombstone]` |
| Tombstoned | Destroyed | Reclamation timeout | Generation safety period elapsed | `[comp.surface.destroy]` |

**Total: 12 allowed transitions.**

---

## 5. Forbidden Transition Table

| From | To | Why |
|------|----|-----|
| Allocated | Visible/Hidden/Minimized | Must map to a frame first |
| Allocated | Tombstoned/Destroyed | Surface has no content to tombstone |
| Mapped | Minimized | Must be Visible first |
| Mapped | Tombstoned/Destroyed | Must go through Closing |
| Visible | Tombstoned/Destroyed | Must go through Closing |
| Hidden | Minimized | Must be Visible first |
| Minimized | Hidden | Must go through Visible first |
| Closing | Visible/Hidden/Minimized | Close is irreversible |
| Closing | Allocated/Mapped | Cannot re-enter live states |
| Tombstoned | Any (except Destroyed) | Cannot resurrect |
| Destroyed | Any | Terminal state |
| Any (with active drag) | Closing/Tombstoned/Destroyed | Drag must cancel first |

**Total: 12 forbidden transitions.**

---

## 6. FocusRef Specification

```rust
/// A validated reference to a surface that may be focused.
/// The LifecycleGeneration prevents stale references from
/// earlier lifecycle epochs.
struct FocusRef {
    surface_id: u64,
    generation: u64,  // LifecycleGeneration at time of reference
}
```

### FocusRef Rules

1. `focused_surface: Option<FocusRef>` — shell-wide current focus.
2. FocusRef is created on successful `try_set_focus()` and stored in `FOCUSED_SURFACE`.
3. On focus validation (A4): compare FocusRef.generation against the target surface's current LifecycleGeneration. If mismatch → reject with `[comp.surface.focus.reject]` reason=stale_generation.
4. On Tombstoned/Destroyed of focused surface: generation increments, FocusRef becomes stale, `clear_focus_if_dead()` selects next valid surface.
5. FocusRef is internal-only — never sent to sexdisplay or apps.
6. FocusRef is not persisted — lost on shell restart (E-gate deferred).

### Decision: Generation Required ✅

LifecycleGeneration is required in A3. Rationale:
- Prevents stale focus after surface destroy + new surface with same ID
- Enables clear_focus_if_dead() to detect stale references by generation, not just alive flag
- Cost: one u64 per surface + one u64 in FocusRef. Acceptable for static array model.
- If generation cannot be added without broad refactor: STOP FIRST and defer to A6.

---

## 7. LifecycleGeneration Specification

```rust
/// Monotonic counter incremented on transitions that invalidate stale
/// references: entering Closing, Closing→Tombstoned, Tombstoned→Destroyed.
/// Used to detect stale references (FocusRef, hover, drag).
/// Starting value: 1. Never decrements. Wraparound checked but assumed improbable.
static mut LIFECYCLE_GENERATION: u64 = 1;
```

### Increment Rules

1. Incremented once when a live state (Visible/Hidden/Minimized) enters Closing.
2. Incremented once on Closing→Tombstoned.
3. Incremented once on Tombstoned→Destroyed.
4. If a surface transitions directly from a live state to Destroyed without going through Closing→Tombstoned (e.g., panel surface toggle), increment once at the Destroyed transition.
5. Never decremented.
6. Wraparound: checked at each increment. If generation would wrap to 0, STOP FIRST (wraparound requires audit of all FocusRef references).
7. Generation 0 is reserved for "no surface" / uninitialized state.

### Stale Reference Detection

| Reference Type | Detection Mechanism |
|---------------|-------------------|
| FocusRef | generation mismatch on focus validation |
| Hovered surface | alive check sufficient (hover is transient) |
| Drag target | alive check sufficient (drag is transient) |
| Tab surface_id | surface_is_alive() + is_tombstoned() |
| Frame flags | FRAME_FLAG_MINIMIZED/FRAME_FLAG_ZOOMED (frame-level, not surface) |

---

## 8. SurfaceId/FrameId Reuse Policy

### Decision: Hardcoded IDs Preserved Temporarily ✅

Current hardcoded SurfaceId constants (100-103, 0x90-0x96, 200-201) are preserved. No dynamic allocation in V1.

### Rules

1. Destroyed SurfaceIds must not be reused without generation safety. Since IDs are hardcoded constants, this is trivially enforced (no new surfaces get old IDs).
2. If future dynamic allocation is added, it must use LifecycleGeneration + slot-based allocation (not sequential ID reuse).
3. FrameId allocation follows same policy — hardcoded constants, no dynamic reuse in V1.
4. The compile-time `APP_SURFACES` registry (lines 89-125 of main.rs) is the canonical allocation source for managed surfaces.
5. Tombstone ring buffer is unchanged in V1 (8 entries, circular). A6 may extend with generation.

---

## 9. Frame/Tab/Scene Interaction Rules

1. Frame owns the lifecycle of its active tab's surface. Surface state transitions update Frame state (e.g., Visible → Minimized sets FRAME_FLAG_MINIMIZED).
2. Tab switch within a Frame does not change lifecycle state of the previously active surface — it remains in its current state (Visible/Hidden).
3. Frame minimized/maximized applies to all tabs in the frame. Individual tab lifecycle is unaffected.
4. Scene membership is per-frame, not per-surface. Surface lifecycle state is independent of scene (Visible vs Hidden depends on active scene).
5. Frame close destroys the active tab's surface through Closing→Tombstoned. Inactive tabs are tombstoned without Closing state (no user-facing close action).
6. Scene switch: all surfaces in the old scene transition Visible→Hidden; all surfaces in the new scene transition Hidden→Visible (subject to minimized check).
7. Atlas has no lifecycle authority. It displays shell-provided state only.

---

## 10. Frame Light Semantics

| Light | Action | FSM Transition | Guard | Proof Marker |
|-------|--------|---------------|-------|-------------|
| Red (close) | Close active tab/surface | Visible/Minimized/Hidden → Closing → Tombstoned | `is_closeable_surface()`, drag not active | `[comp.surface.close]` or `.close.reject` |
| Yellow (minimize) | Minimize frame | Visible → Minimized | Not already minimized | `[comp.surface.minimize]` or `.minimize.reject` |
| Green (zoom) | Toggle frame zoom | No lifecycle transition (flag toggle only) | Frame supports zoom | `[comp.surface.zoom]` or `.zoom.reject` |

### Specific Rules

1. Red light close is idempotent: second close on Closing/Tombstoned state returns false and fires `[comp.surface.close.reject]` reason=already_closing|already_dead.
2. Yellow light minimize on already-minimized frame: no-op with `[comp.surface.minimize.reject]` reason=already_minimized.
3. Green light zoom does not change lifecycle state — only FRAME_FLAG_ZOOMED flag. Geometry bounds are preserved via existing clamp_position/clamp_surface_size.
4. Red light action must check for active drag before transition. If drag active on target surface, cancel drag first (A5 implementation detail).
5. Frame lights are visual only — sexdisplay renders lights but shell decides action.

---

## 11. Tombstone Semantics

1. Tombstoned is not live content — cannot be focused, rendered, or interacted with.
2. Tombstone record holds: SurfaceId, LifecycleGeneration at time of tombstone, destroy timestamp (shell tick count), caller identity (A6 addition).
3. Tombstone reclamation: Deferred to A6. Current 8-entry ring buffer is sufficient for V1. Generation safety period is not yet defined.
4. Tombstone visibility: Tombstoned surfaces may appear in Atlas/debug view as dead entries, never as interactive surfaces.
5. Tombstone does NOT prevent new surfaces with different IDs — only prevents focus/drag on the tombstoned ID.
6. Tombstone + LifecycleGeneration together prevent stale reference reuse. Both are required for full safety.

---

## 12. Drag/Resize Cancellation Rules

1. Before any lifecycle transition that would invalidate the drag target (close → Closing/Tombstoned, minimize, destroy), the implementation MUST check for active drag.
2. If drag active on the target surface:
   - Cancel drag: `try_transition(InteractionState::Idle)`
   - Log: `[comp.surface.cancel.drag]` reason=lifecycle_transition target=sid
   - Then proceed with lifecycle transition
3. If drag active on a DIFFERENT surface:
   - Do NOT cancel — the lifecycle transition affects a different target
4. Zoom toggle does NOT require drag cancellation (zoom preserves surface identity)
5. A5 implementation detail: `clear_drag_if_dead()` must be renamed/converted to `cancel_drag_before(target_surface_id)` and called BEFORE lifecycle transition, not after.

---

## 13. Unknown/Stale ID Handling

1. Unknown SurfaceId → deterministic no-op, return false, fire `[comp.error]` reason=unknown_surface.
2. Destroyed SurfaceId in any operation → deterministic no-op, return false, fire reason=destroyed.
3. Tombstoned SurfaceId in focus/drag/input → reject with reason=tombstoned.
4. Stale FocusRef (generation mismatch) → reject with reason=stale_generation.
5. All reject paths must be deterministic — no panic, no spin, no error flood.
6. Panel/os-owned surface IDs (0x90-0x96) use their own alive flags and toggle mechanisms — closed/disabled panels are not lifecycle-managed (they toggle between alloc/visible and destroyed directly).

---

## 14. Sexdisplay Conformance Rules

1. sexdisplay never decides lifecycle meaning, focus policy, tab/frame/scene policy.
2. sexdisplay renders only bounded pixels from shell-provided model.
3. All surface geometry is bounds-checked via `clamp_position()`/`clamp_surface_size()` before sexdisplay receives it.
4. sexdisplay receives standard geometry/create/focus/update opcodes — no lifecycle-specific protocol.
5. **0xEE collision handling (from A1 finding):**
   - A2 does not invent new opcodes or protocol changes.
   - A3/A4 implementation must preserve current 0xEE behavior (destroy + hide use same opcode).
   - A5/A7 must audit protocol constants and decide: split into separate opcodes, add lifecycle state field to existing opcode payload, or accept the collision with lifecycle state in snapshot.
   - STOP FIRST for any sexdisplay protocol extension before A5 audit.
6. sexdisplay has no authority to refuse or modify shell state commands — it renders what it receives.

---

## 15. Proof Markers Required by Transition

```
// Lifecycle transitions
[comp.audit.start]                        // A1: audit begins
[comp.surface.map]                        // Allocated → Mapped
[comp.surface.map.reject]                 // Map failed (no slot)
[comp.surface.visible]                    // → Visible (from Mapped/Hidden/Minimized)
[comp.surface.hide]                       // Visible → Hidden
[comp.surface.minimize]                   // Visible → Minimized
[comp.surface.minimize.reject]            // Minimize on already-minimized
[comp.surface.zoom]                       // Zoom flag toggle
[comp.surface.zoom.reject]                // Zoom on non-zoomable frame
[comp.surface.close]                      // → Closing (from Mapped/Visible/Hidden/Minimized)
[comp.surface.close.reject]               // Close on already-Closing/Tombstoned
[comp.surface.tombstone]                  // Closing → Tombstoned
[comp.surface.destroy]                    // Tombstoned → Destroyed

// Focus
[comp.surface.focus.set]                  // Focus set on valid target
[comp.surface.focus.reject]               // Focus rejected (reason: dead/tombstoned/minimized/stale_generation/nonfocusable/wrong_scene/not_shell_caller)
[comp.surface.focus.clear]                // Focus cleared (reason: dead/stale/scene_switch/empty)

// Scene
[comp.scene.switch]                       // Active scene changed

// Geometry
[comp.surface.geometry.update]            // Surface geometry changed

// Drag
[comp.surface.cancel.drag]                // Drag cancelled (reason: lifecycle_transition/target_dead)

// Error
[comp.error]                              // Unknown/stale surface ID, unexpected condition

// Pref lifecycle (Scan 8)
[comp.pref.load]          [comp.pref.validate.ok]     [comp.pref.validate.reject]
[comp.pref.apply]         [comp.pref.reset]           [comp.pref.redact]
```

**Total: 20 core markers + 6 pref lifecycle markers = 26.**

### Existing Marker Migration (from A1)

| Current Marker | Target Marker | When |
|---------------|---------------|------|
| `[shell.focus.set]` | `[comp.surface.focus.set]` | A8 rename pass |
| `[shell.focus.reject.*]` | `[comp.surface.focus.reject]` | A8 rename pass |
| `[shell.surface.focus.clear.dead]` | `[comp.surface.focus.clear]` | A8 rename pass |
| `[shell.frame.minimize]` | `[comp.surface.minimize]` | A8 rename pass |
| `[shell.interaction.transition]` | `[comp.surface.cancel.drag]` | A8 rename pass |
| `[shell.surface.unknown.reject]` | `[comp.error]` | A8 rename pass |

---

## 16. Negative Proof Scenarios

| # | Scenario | FSM Guard | Expected Result | Proof Marker |
|---|----------|-----------|----------------|--------------|
| 1 | Focus Destroyed surface | Generation check | `try_set_focus()`=false | `[comp.surface.focus.reject]` reason=destroyed |
| 2 | Focus Tombstoned surface | `is_tombstoned()` | `try_set_focus()`=false | `[comp.surface.focus.reject]` reason=tombstoned |
| 3 | Focus Minimized surface | `frame_accepts_input()`=false | `try_set_focus()`=false | `[comp.surface.focus.reject]` reason=minimized |
| 4 | Close already-Closing surface | Idempotent guard | `close_surface()`=false | `[comp.surface.close.reject]` reason=already_closing |
| 5 | Close Tombstoned surface | `surface_is_alive()`=false | `close_surface()`=false | `[comp.surface.close.reject]` reason=already_dead |
| 6 | Minimize already-minimized frame | `frame_is_minimized()` | no-op | `[comp.surface.minimize.reject]` reason=already_minimized |
| 7 | Close during active drag | Drag check | Drag cancelled, then close | `[comp.surface.cancel.drag]` then `[comp.surface.close]` |
| 8 | Unknown SurfaceId | ID range check | no-op, return false | `[comp.error]` reason=unknown_surface |
| 9 | App PD sends focus request | Caller identity check (A4) | Rejected if not shell | `[comp.surface.focus.reject]` reason=not_shell_caller |
| 10 | Focus with stale generation | FocusRef.generation mismatch | Rejected | `[comp.surface.focus.reject]` reason=stale_generation |
| 11 | Scene switch, no Visible surface | clear_focus_if_dead() fallback | Focus cleared, no crash | `[comp.surface.focus.clear]` reason=no_surface_in_scene |
| 12 | Destroy during drag | Drag check | Drag cancelled, surface destroyed | `[comp.surface.cancel.drag]` then `[comp.surface.destroy]` |

---

## 17. A3 Implementation Requirements

1. **LifecycleState enum** — Add `LifecycleState` with 8 variants as defined in §3.
2. **State tracking** — Add `lifecycle: LifecycleState` field to each surface's tracking struct (which struct depends on Vec resolution).
3. **WINDOWS Vec resolution** — A2 decision: preserve temporarily. A3 adds lifecycle state alongside existing alive booleans without removing Vec. Migration to static array happens in A5.
4. **LifecycleGeneration counter** — Add `static mut LIFECYCLE_GENERATION: u64 = 1` and increment rules from §7.
5. **FocusRef** — Replace `FOCUSED_SURFACE_ID: u64` with `FOCUSED_SURFACE: Option<FocusRef>`.
6. **Transition dispatch** — No behavior change in A3. State tracking is additive — existing close/minimize/focus paths still work.
7. **No proof marker changes in A3** — Proof marker renaming is A8. A3 adds lifecycle state tracking only.
8. **Sexdisplay opcodes unchanged** — A3 preserves current 0xEC/0xEE/0xED usage. No protocol audit in A3.

---

## 18. A4 Focus Guard Requirements

1. **Caller identity validation** — `try_set_focus()` must distinguish shell-internal focus changes from PD-originated focus requests. Mechanism: add `caller: FocusSource` enum (`ShellInternal`, `PdxRequest`). `PdxRequest` focus requests must pass additional validation (surface must be in active scene, frame must accept input, surface must be focusable).
2. **Generation safety** — `try_set_focus()` must validate FocusRef.generation against the target surface's current LifecycleGeneration.
3. **Minimized check** — `try_set_focus()` must call `frame_accepts_input()` or equivalent to reject focus on minimized frames.
4. **Drag-pin rule** — `try_set_focus()` must reject focus change if `InteractionState::Dragging` is active and the target surface differs from the drag target.
5. **clear_focus_if_dead() update** — Use generation check instead of (or in addition to) `surface_is_alive()`. Derive z-order from frame state, not hardcoded array.
6. **clear_focus_if_wrong_scene() update** — Already iterates frames — add generation validation to surface selection.

---

## 19. STOP FIRST Conditions

- Any SurfaceId reuse without LifecycleGeneration safety
- Any sexdisplay lifecycle policy ownership
- Any Destroyed surface resurrection
- Any focus on Tombstoned or Minimized surface
- Any lifecycle transition during active drag (without drag cancel first)
- Any kernel/ABI/sex-pdx edit without explicit handoff
- Any dynamic allocation for FSM state without generation safety
- Any sexdisplay protocol extension before A5/A7 audit
- Any close that bypasses Closing→Tombstoned→Destroyed chain
- Any implementation before this spec is accepted

---

## 20. Remaining Open Questions

1. **WINDOWS Vec migration timing:** A5 vs defer. A2 recommends preserve temporarily; A3 adds state alongside.
2. **Caller identity mechanism detail:** A4 must design `FocusSource` enum and validation rules. Exact shape depends on how PD focus requests arrive (PDX message type).
3. **Tombstone reclamation policy:** A6 must define generation safety period. Not specified in V1 — 8-entry ring buffer is sufficient.
4. **0xEE opcode resolution:** A5/A7 must decide separate opcodes vs payload field vs accept collision. Not resolved in A2.
5. **Panel surface lifecycle:** Panel surfaces (0x92-0x96) toggle between alive and dead directly, bypassing the 8-state FSM. A3 must decide whether panels adopt the full FSM or remain special-cased.
6. **FrameId generation:** Should FrameIds also carry a generation counter? Deferred — FrameIds are even more static than SurfaceIds in V1.
7. **Hardcoded z-order for clear_focus_if_dead():** Should be derived from frame z-order. But current z-order is implicit (sort by Frame array index + active scene + zoom status). A4 must design dynamic z-order derivation.

---

## Document References

- `docs/A_COMPOSITOR_LIFECYCLE_PLAN_V1.md` — parent plan doc
- `docs/handoff/A1_COMPOSITOR_LIFECYCLE_AUDIT_V1.md` — audit findings informing this spec
- `servers/silk-shell/src/main.rs` — current implementation context (no code changed)
