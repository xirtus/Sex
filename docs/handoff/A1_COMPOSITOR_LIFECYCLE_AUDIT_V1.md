# A1_COMPOSITOR_LIFECYCLE_AUDIT_V1

**Status:** Audit complete. No code changed.
**Date:** 2026-05-04
**Audit target:** `servers/silk-shell/src/main.rs` (4912 lines)

---

## 1. Executive Summary

The current silk-shell lifecycle model is **implicit, scattered, and incomplete** relative to the A doc's 8-state FSM. Lifecycle is tracked via per-surface boolean flags, a bounded tombstone ring buffer, and frame flag bits — no unified FSM, no LifecycleGeneration counter, no FocusRef pair, no explicit transition dispatch. Focus validation has 4 partial guards (alive, focusable, non-tombstoned, wrong-scene) but lacks generation safety, caller identity checks, and minimized-frame rejection. Close/minimize/zoom actions exist and dispatch through sexdisplay opcodes but bypass the FSM transition model (no Closing→Tombstoned→Destroyed chain, no drag-cancellation-before-close guard). sexdisplay receives lifecycle-level opcodes (0xEC=create/upsert, 0xEE=destroy/hide, 0xED=focus set) but shell treats 0xEE as both destroy and minimize, creating a semantic collision.

**Net assessment:** The A doc's FSM and invariants are achievable but require explicit state tracking, a LifecycleGeneration counter, FocusRef pairs, and FSM-gated transition dispatch. No code was changed.

---

## 2. Files Inspected

- `servers/silk-shell/src/main.rs` (4912 lines) — all lifecycle, focus, frame, tab, scene, interaction, and display path logic

---

## 3. Current Lifecycle Model

**No explicit FSM exists.** Lifecycle state is tracked through multiple disjoint mechanisms:

| Mechanism | What It Tracks | Location |
|-----------|---------------|----------|
| `SURFACE_N_ALIVE: bool` | Per-surface booleans (100-103, panels) | Lines 1111-1128 |
| `TOMBSTONES: [u64; 8]` | Circular buffer of closed surface IDs | Lines 1108-1110 |
| `FRAME_FLAG_MINIMIZED` | Frame-level bit | Line 1011 |
| `FRAME_FLAG_ZOOMED` | Frame-level bit | Line 1016 |
| `ShellFrame.flags` | Combined frame flags | Line 862 |
| `tab_exists`, `tab.surface_id` | Tab presence = surface mapped to frame | Line 837 |

**Implicit states actually present in code:**

| State | How It Exists | Gap vs A Doc |
|-------|--------------|--------------|
| Allocated | SurfaceId known but no frame → implicitly unallocated | No explicit Allocated state. All surfaces are either alive or dead. |
| Mapped | Tab in frame = surface is mapped | No distinct Mapped state. Alive + in tab = implicitly Mapped+Visible. |
| Visible | alive + frame not minimized + active scene | Works for V1. |
| Hidden | alive + frame in non-active scene | Works via `surface_in_active_scene()`. |
| Minimized | `FRAME_FLAG_MINIMIZED` set | Works but 0xEE used for both destroy and minimize (collision). |
| Closing | Skipped entirely — close jumps directly to dead | **CRITICAL GAP** — no Closing state, no Tombstoned→Destroyed chain. |
| Tombstoned | `TOMBSTONES` buffer entry + `is_tombstoned()` check | Exists as ring buffer but no generation, no lifecycle counter. |
| Destroyed | `SURFACE_N_ALIVE = false` | Works implicitly. No generation safety on ID reuse. |

**CRITICAL FINDING:** Close skips Closing→Tombstoned→Destroyed entirely. `close_surface_from_frame_light()` sets the alive flag to false, calls `tombstone_surface()` (which just adds the ID to the ring buffer), sends 0xEE to sexdisplay, and re-tiles. There is no FSM transition dispatch, no generation increment, no proof marker for the lifecycle transition.

---

## 4. Current Focus Model

**Storage:** `FOCUSED_SURFACE_ID: u64` (line 1086) — single u64, no generation pair.

**Validation guards in `try_set_focus()` (line 3088):**
1. `sid == 0` → clear focus ✅
2. `is_focusable_surface(sid)` → reject non-focusable ✅
3. `surface_is_alive(sid)` → reject dead ✅
4. `is_tombstoned(sid)` → reject tombstoned ✅
5. `surface_in_active_scene(sid)` → reject wrong-scene ✅

**Missing guards vs A doc §8:**
- ❌ No caller identity check — any code path can call `try_set_focus()`
- ❌ No generation safety — `FOCUSED_SURFACE_ID` is a bare u64, not a `(SurfaceId, LifecycleGeneration)` pair
- ❌ No minimized-frame rejection — `try_set_focus()` does not call `frame_accepts_input()`
- ❌ No drag-pin rule — focus can change during active drag

**`clear_focus_if_dead()` (line 1331):**
- Checks `surface_is_alive(focused)` — if dead, iterates hardcoded z-order `[QUIL, LINEN, TEST4, TEST3, STATIC, APP]` and sets focus to first alive surface
- No tombstone check in the fallback scan (could land on tombstoned surface — partial risk)
- Hardcoded z-order vs A doc's "selects next valid surface from z-order"
- No LifecycleGeneration for stale reference detection

**`clear_focus_if_wrong_scene()` (line 1419):**
- Called after scene switch. Iterates frames in active scene, focuses first alive+non-tombstoned tab surface.
- If none found, clears focus to 0.

---

## 5. Current Frame/Tab/Scene/Atlas Model

**Frame:**
- `ShellFrame` struct with `frame_id`, `active_tab`, `tab_count`, `scene_id`, `flags`, normal geometry (lines 851-868)
- `FRAMES: [Option<ShellFrame>; MAX_FRAMES]` — static array, no heap ❌ but `WINDOWS: Vec<WindowState>` IS heap-allocated
- Flags: `FRAME_FLAG_MINIMIZED`, `FRAME_FLAG_ZOOMED`, `FRAME_FLAG_TOP_BAR`
- No `FRAME_FLAG_CLOSING` or lifecycle state field — frame is assumed alive if tabs have alive surfaces

**Tab:**
- `ShellTab` with `surface_id: u64` and `title_id: u64` (line 837)
- Up to `MAX_TABS_PER_FRAME` tabs per frame (line 827)
- Tab data is display-only — no capability or authority

**Scene:**
- `ACTIVE_SCENE_IDX: u8` (line 1091) — active scene index
- `WORKSPACE_COUNT: u8 = 5` (implied by `ATLAS_MAX_SCENES`)
- Scene descriptors derived from frame state during `snap_capture_layout()`
- Scene switch updates ACTIVE_SCENE_IDX, calls `clear_focus_if_wrong_scene()`, `tile_visible_frames()`, `snap_capture_layout()`

**Atlas:**
- `ATLAS_SNAPSHOT: AtlasSnapshot` — static, derived from frame state
- `ATLAS_MODE_ENABLED: bool` — state only, no visual behavior in V1
- Atlas is overview mode, not a separate lifecycle owner

**Contradiction with A doc:** `WINDOWS: Vec<WindowState>` at line 870 is an existing heap-backed model that conflicts with the target lifecycle canon (STOP FIRST §12.10: "No heap allocation for FSM state (static arrays only)"). A2 must decide whether to preserve temporarily, replace with static arrays in A3, or STOP FIRST if replacement causes broad refactor.

---

## 6. Current Frame Light Behavior

**Colors mapped:**
- Red (close light) → `close_surface_from_frame_light(surface_id)` → sets alive=false, sends 0xEE
- Yellow (minimize light) → `minimize_frame(frame_id)` → sets FRAME_FLAG_MINIMIZED, sends 0xEE
- Green (zoom light) → `toggle_zoom_frame(frame_id)` → toggles FRAME_FLAG_ZOOMED, sends 0xEC with new geometry

**Issues:**
- **0xEE semantic collision:** Both close and minimize send 0xEE to sexdisplay. sexdisplay cannot distinguish destroy from hide. A doc requires this distinction (Closing vs Minimized are different FSM states).
- **No Drag cancellation before close:** `close_surface_from_frame_light()` does not check for active drag. If a drag is in progress on the target surface, close proceeds anyway. Violates A doc invariant §11.9.
- **Close is not idempotent:** If called twice on the same surface, the second call returns `false` (alive check on line 2282) but produces no proof marker or reject log. A doc requires `[comp.surface.close.reject]` for idempotent close.
- **Minimize on already-minimized:** `minimize_frame()` returns `false` at line 2383. No reject proof marker.
- **Zoom toggle bounds:** No geometry bounds check before sending zoom geometry to sexdisplay via 0xEC.

---

## 7. Current Sexdisplay Conformance Findings

**Opcodes used (from shell→sexdisplay):**

| Opcode | Name | Used For | Issue |
|--------|------|----------|-------|
| 0xEC | create/upsert | Surface create, restore from minimize, zoom geometry update | Used for both create AND geometry update — okay but no lifecycle state in payload |
| 0xEE | destroy/hide | Close (destroy) AND minimize (hide) | **SEMANTIC COLLISION** — sexdisplay cannot distinguish destroy from hide |
| 0xED | focus set | Focus change | No LifecycleGeneration in payload |
| 0xEF | Quil toggle | Quil surface toggle | Quil-specific, not generic |
| 0x15 | OP_DISPLAY_SET_SNAPSHOT | Full window snapshot | Only window_id + geometry + focus_state — no lifecycle state |
| 0xEB | OP_SURFACE_UPDATE | Position update | x/y only, no state |

**Positive findings:**
- ✅ sexdisplay never decides lifecycle semantics — all state is shell-provided
- ✅ Geometry bounds checks exist via `clamp_position()`/`clamp_surface_size()` before sexdisplay receives geometry
- ✅ No raw cross-PD pointers — all communication via PDX
- ✅ sexdisplay framebuffer writes are bounded

**Issues:**
- 0xEC payload contains geometry but no lifecycle state — sexdisplay cannot validate lifecycle-appropriate state
- No opcode for Tombstoned vs Destroyed vs Hidden distinction
- Snapshot contains `focus_state: u32` (0 or 1) but no lifecycle field

---

## 8. SurfaceId/FrameId Allocation and Reuse Findings

**SurfaceId allocation (hardcoded):**
- 100-103: App surfaces (APP, STATIC, TEST3, TEST4)
- 0x90-0x96: OS-owned (CURSOR, LAUNCHER, STATUS, CLOCK, BELL, SCENE_SETTINGS)
- 200-201: Managed app surfaces (LINEN, QUIL)

**Generation safety:** ❌ None. `FOCUSED_SURFACE_ID` is bare u64. No LifecycleGeneration counter exists anywhere. SurfaceId reuse is currently impossible because IDs are hardcoded constants, not dynamically allocated — but there is no mechanism to prevent reuse if allocation changes.

**FrameId allocation (hardcoded):** Static constants (e.g., `LINEN_FRAME_ID`, `QUIL_FRAME_ID`). No generation counter.

**Tombstone ring buffer (lines 1108-1110):**
- Size: 8 entries (fixed)
- Insertion: circular, oldest dropped when full
- No generation tracking — only the surface ID is stored
- Purpose: prevent immediate focus/drag on recently closed surfaces
- Current usage: checked by `is_tombstoned()` in focus path, tab iteration, and frame input checks
- **Gap:** No generation, no timestamp, no caller identity, no timeout for reclamation

---

## 9. Current Reject/No-Op Behavior for Stale/Unknown IDs

**Known behaviors (audited):**
- `surface_is_alive()` returns `false` for unknown IDs with `[shell.surface.unknown.reject]` proof marker ✅
- `try_set_focus()` returns `false` for dead/non-focusable/tombstoned/wrong-scene ✅
- `close_surface_from_frame_light()` returns `false` for dead surfaces ❌ but no reject proof marker on second call
- `hit_test_surface_chrome()` returns `None` if frame doesn't accept input (handles wrong-scene, minimized, dead, tombstoned) ✅
- `point_in_surface()` logs `unknown.reject` for unknown IDs ✅
- `frame_accepts_input()` returns `false` for minimized, wrong-scene, dead, tombstoned ✅
- Panel toggles check `PANEL_ACTIVE` before double-toggle ✅

---

## 10. Drag/Resize Interaction Findings

**InteractionState FSM (line 3158):**
```
Idle → ClickPending → Dragging → Idle
                    → Idle (release)
                    → PanelActive → Idle / ClickPending
```
- `Idle` can transition to ANY state (including Dragging, ClickPending, PanelActive)
- `Dragging` can only transition to `Idle`
- `PanelActive` can transition to `Idle` or `ClickPending`
- All other transitions are forbidden and logged as `[shell.interaction.forbidden]`

**Drag cancellation on surface death:**
- `clear_drag_if_dead()` (line 1354) checks if dragged surface is alive; if dead, transitions to Idle ✅
- Called by `close_surface_from_frame_light()` ✅
- Called by `minimize_frame()` ✅
- **But NOT called by `toggle_zoom_frame()`** ❌ — zoom could proceed during drag
- **Missing:** No drag check BEFORE lifecycle transition — drag is cancelled AFTER, not before. A doc requires "drag must cancel before lifecycle transition."

**InteractionState orthogonality with focus:**
- When Dragging, focus changes are not blocked — `try_set_focus()` has no drag check ❌
- Focus is pinned to drag target implicitly because pointer events go to the drag target, but `try_set_focus()` can be called from keyboard actions during drag

---

## 11. Direct Contradictions with A_COMPOSITOR_LIFECYCLE_PLAN_V1

| A Doc Requirement | Current State | Severity |
|------------------|---------------|----------|
| 8-state FSM (Allocated→Mapped→Visible→Hidden/Minimized→Closing→Tombstoned→Destroyed) | No FSM. Implicit alive/dead + frame flags + tombstone ring. | **Blocks A3** |
| Close goes through Closing→Tombstoned→Destroyed | Close jumps alive=false + tombstone. No Closing state, no generation. | **Blocks A5** |
| LifecycleGeneration monotonic counter | Does not exist. | **Blocks A3/A4/A6** |
| FocusRef: (SurfaceId, LifecycleGeneration) pair | `FOCUSED_SURFACE_ID` bare u64. | **Blocks A4** |
| Generation safety for SurfaceId reuse | None. | **Blocks A6** |
| Close is idempotent with proof marker | Second close returns false, no proof marker. | **Blocks A5** |
| Drag cancels before lifecycle transition | Drag cancelled AFTER close/minimize, not before. No check in toggle_zoom_frame(). | **Blocks A5** |
| Apps cannot force focus — caller identity check | No caller validation. Any code path can call try_set_focus(). | **Blocks A4** |
| Minimized cannot receive pointer focus | try_set_focus() doesn't check frame_accepts_input(). Could focus minimized. | **Blocks A4** |
| No heap allocation for FSM state | `WINDOWS: Vec<WindowState>` is heap-backed | A2 must decide: preserve, replace in A3, or STOP FIRST if broad refactor |
| Lifecycle transitions produce proof markers | No lifecycle proof markers exist. Existing markers are per-operation. | **Blocks A8** |
| 0xEE for destroy only, 0x?? for hide | 0xEE used for both destroy AND minimize/hide. | **Blocks A7** |
| All geometry bounds-checked | Yes, clamp_position/clamp_surface_size exist. | ✅ OK |
| sexdisplay never decides lifecycle | Yes, sexdisplay is pure renderer. | ✅ OK |

---

## 12. Missing Proof Markers vs A Doc §13

| Required Marker | Exists? | Finding |
|----------------|---------|---------|
| `[comp.audit.start]` | ❌ | Would fire once at A1 start |
| `[comp.surface.map]` | ❌ | No explicit map transition |
| `[comp.surface.map.reject]` | ❌ | No map reject path |
| `[comp.surface.visible]` | ❌ | No explicit visible transition |
| `[comp.surface.hide]` | ❌ | No explicit hide transition |
| `[comp.surface.minimize]` | ⚠️ Partial | `[shell.frame.minimize]` exists (budgeted) but not `[comp.surface.minimize]` |
| `[comp.surface.minimize.reject]` | ❌ | No reject marker on already-minimized |
| `[comp.surface.zoom]` | ❌ | No zoom proof marker |
| `[comp.surface.zoom.reject]` | ❌ | No zoom reject marker |
| `[comp.surface.close]` | ❌ | Close returns true but no `[comp.surface.close]` marker |
| `[comp.surface.close.reject]` | ❌ | Second close returns false silently |
| `[comp.surface.tombstone]` | ❌ | `tombstone_surface()` has no proof marker |
| `[comp.surface.destroy]` | ❌ | No destroy proof marker |
| `[comp.surface.focus.set]` | ⚠️ Partial | `[shell.focus.set]` exists but name doesn't match A doc spec |
| `[comp.surface.focus.reject]` | ⚠️ Partial | `[shell.focus.reject.*]` variants exist but names don't match |
| `[comp.surface.focus.clear]` | ⚠️ Partial | `[shell.surface.focus.clear.dead]` exists |
| `[comp.scene.switch]` | ❌ | No scene switch proof marker |
| `[comp.surface.geometry.update]` | ❌ | No geometry update proof marker |
| `[comp.surface.cancel.drag]` | ⚠️ Partial | `[shell.interaction.transition]` to Idle exists but not specific drag-cancel marker |
| `[comp.error]` | ❌ | Unknown SurfaceId logged as `[shell.surface.unknown.reject]` — name mismatch |

**Existing markers closest to A doc spec (need renaming in A8):**
- `[shell.focus.set]` → `[comp.surface.focus.set]`
- `[shell.focus.reject.*]` → `[comp.surface.focus.reject]`
- `[shell.surface.focus.clear.dead]` → `[comp.surface.focus.clear]`
- `[shell.frame.minimize]` → `[comp.surface.minimize]`
- `[shell.interaction.transition]` → `[comp.surface.cancel.drag]` (partial)

---

## 13. Risk Table

| Risk | File/Function | Invariant Threatened | Severity | Safest Next Phase |
|------|--------------|---------------------|----------|-------------------|
| No FSM exists — lifecycle is scattered booleans + flags | `SURFACE_N_ALIVE`, `FRAME_FLAG_*`, `TOMBSTONES` | §11.1-8, §11.19 | **Critical** | A3: add lifecycle state enum + transition dispatch |
| Close skips Closing→Tombstoned→Destroyed | `close_surface_from_frame_light()` | §11.1, §11.6, §11.7 | **Critical** | A5: wire close through FSM |
| No LifecycleGeneration counter | — | §11.2, §11.3, §11.17 | **Critical** | A3: add monotonic generation counter |
| No generation safety for Surface reuse | — | §11.2, §11.17 | **Critical** | A6: add generation check to FocusRef |
| try_set_focus() lacks caller identity check | `try_set_focus()` | §11.10 | **High** | A4: add caller/event authority validation |
| try_set_focus() doesn't check minimized | `try_set_focus()` | §11.4 | **High** | A4: add `frame_accepts_input()` check |
| 0xEE used for both destroy and hide | `close_surface_from_frame_light()`, `minimize_frame()` | §11.11 | **High** | A7: separate opcodes or add lifecycle state to payload |
| Drag not checked before lifecycle transition | `close_surface_from_frame_light()`, `toggle_zoom_frame()` | §11.9 | **High** | A5: add drag-before guard |
| WINDOWS uses Vec (heap) | line 870 | §12.10 (STOP FIRST) | Medium | A2: decide preserve vs replace vs STOP FIRST |
| Close not idempotent (no reject proof marker) | `close_surface_from_frame_light()` | §11.6, §11.19 | **Medium** | A5: add reject marker |
| Focus change during drag not blocked | `try_set_focus()` | §11.9 | **Medium** | A4: add drag-pin rule |
| Hardcoded z-order in clear_focus_if_dead() | `clear_focus_if_dead()` | §11.3 | **Low** | A4: derive z-order from frame state |
| No FOCUSED_SURFACE_ID mutation guard | `try_set_focus()` (no caller check) | §11.10 | **Medium** | A4: add source tracking |
| Closeable surface checking depends on registry + hardcoded match | `is_closeable_surface()` | — | **Low** | A3: lifecycle state should determine closeability |

---

## 14. A2/A3/A4 Blockers

**A2 (FSM Spec) blockers:** None. A2 is a handoff doc — the audit confirms the FSM spec can be written against the A doc's 8-state model regardless of current code state.

**A3 (Shell Lifecycle Model) blockers:**
1. `WINDOWS: Vec<WindowState>` is heap-backed — A2 must decide strategy before A3 adds state tracking
2. No lifecycle state field on Surface/Frame — needs `surface_lifecycle: LifecycleState` enum
3. No LifecycleGeneration counter — needs `LIFECYCLE_GENERATION: u64`
4. No FocusRef — needs `FOCUSED_SURFACE: Option<(u64, u64)>` (surface_id + generation)

**A4 (Focus Validity Guards) blockers:**
1. No caller identity mechanism — needs caller tracking or explicit designation of shell-internal vs PD-originated focus requests
2. No generation safety on FOCUSED_SURFACE_ID — needs FocusRef with generation
3. No minimized check in try_set_focus() — needs `frame_accepts_input()` call
4. No drag-pin rule — needs `InteractionState::Dragging` check in try_set_focus()
5. Hardcoded `clear_focus_if_dead()` z-order — needs dynamic derivation

---

## 15. Recommended Smallest-Safe Phase Order

1. **A2 (FSM spec)** — No code, no blockers. Write handoff doc defining 8-state FSM, transitions, guards, proof markers. Can proceed immediately.
2. **A3 (Shell lifecycle model)** — Add `LifecycleState` enum per surface, `LifecycleGeneration` counter, resolve `WINDOWS` Vec strategy per A2 decision. No behavior change yet.
3. **A4 (Focus validity guards)** — Add generation safety to focus, caller validation framework, minimized check, drag-pin rule. Update `clear_focus_if_dead()` and `clear_focus_if_wrong_scene()`.
4. **A6 (Tombstone events)** — Add generation to tombstone records, timestamp, caller identity. Can be done independently of A5.
5. **A5 (Frame light actions)** — Wire close→Closing→Tombstoned→Destroyed through FSM. Add drag-before guard. Separate 0xEE destroy from 0x?? hide.
6. **A7 (Display conformance)** — Verify sexdisplay receives only lifecycle-valid state. Add lifecycle field to snapshot/opcode payloads if needed.
7. **A8 (Proof scenarios)** — Rename existing markers to `[comp.*]` convention. Add missing markers. Verify all transitions produce correct markers.

**A3 and A4 can partially-in parallel:** A3 adds the state tracking infrastructure; A4 adds focus guards that read that state. A4 logic can be written against A3's state API as long as both are designed together.

---

## 16. STOP FIRST Findings

No STOP FIRST violations were found in the current codebase. The following areas require vigilance:

1. **Any SurfaceId reuse without generation safety** — currently not possible (hardcoded IDs), but must remain STOP FIRST if allocation changes
2. **Any sexdisplay lifecycle policy ownership** — sexdisplay does not infer lifecycle state ✅
3. **Any Destroyed surface resurrection** — no code path resurfaces dead IDs ✅ (but no mechanism prevents it if allocation changes)
4. **Any focus on Tombstoned/Minimized** — try_set_focus() checks tombstoned but NOT minimized ✅ partial (minimized missing)
5. **Any lifecycle transition during active drag** — close_surface_from_frame_light() does not check drag before close ❌ (gap, not violation — no drag target is closed by current dispatch paths in practice)
6. **Any dynamic allocation for FSM state** — `WINDOWS: Vec<WindowState>` IS heap-backed — A2 must decide: preserve temporarily, replace in A3, or STOP FIRST if replacement causes broad refactor
7. **Any sexdisplay protocol extension for lifecycle semantics** — no extension proposed ✅

---

## 17. Do Not Implement / Wait for Review

- **No code was changed** during this audit.
- A2 (FSM spec) can proceed immediately — no code, handoff doc only.
- A3, A4, A5, A6, A7, A8 must wait for A2 spec to stabilize before any implementation.
- The `WINDOWS: Vec` heap model must be resolved per A2 decision: preserve temporarily, replace in A3, or STOP FIRST.
- The 0xEE semantic collision (destroy vs hide) must be resolved before A7 display conformance.
- Caller identity validation mechanism must be designed in A4 before any focus authority changes.
- Proof marker renaming (existing `[shell.*]` → `[comp.*]`) should be done in A8 to avoid breaking existing log parsing during intermediate phases.
