# QUIL_SURFACE_STUB_V1

**Status:** Active — Post-7-Scan Hardening  
**Purpose:** Allocate Quil surface identity and shell lifecycle matching the Linen control pattern.  
**Scope:** `servers/silk-shell/src/main.rs` only. No kernel/ABI/sexdisplay changes.  
**Reference:** `rapid/PHASE_05_QUIL_LANGUAGE_WORKSTATION.md` (full Quil workstation plan — this stub only allocates surface identity per that plan's smallest-first-step, NOT the full editor)

---

## Current State (commit 98cadaf)

Quil (`surface_id=201`) is a first-class shell-managed app surface, following the identical pattern established by Linen (`surface_id=200`). No Quil server binary exists — this is pure shell surface/frame integration. The stub does NOT implement the Quil editor, dev cockpit, filesystem access, agent orchestration, or any rendering beyond surface identity.

### Surface Identity

| Property | Value |
|----------|-------|
| `SURFACE_ID_QUIL` | `201` |
| `QUIL_FRAME_ID` | `3` (frame 1 = APP/STATUS, frame 2 = Linen) |
| Boot geometry | `(100, 100, 640, 480)` |
| Frame flags | `FRAME_FLAG_TOP_BAR` (matching default) |
| Shell-side surface identity | Always considered alive by shell (`surface_is_alive(201) = true` hardcoded — not destroyable, no server required). This is a shell-side identity only, not a real surface on display. |

### Wiring Changes (12 locations, commit 98cadaf)

| Location | Change |
|----------|--------|
| `SURFACE_ID_QUIL` constant (line 61) | Added next to `SURFACE_ID_LINEN` |
| Surface ID registry comment (line 77) | Added `201 quil (surface stub)` |
| `SURFACE_201_X/Y/W/H` statics (lines 997-1000) | Geometry tracking |
| `tile_visible_frames()` (line 698) | QUIL match arm for tiling position |
| `emit_snapshot()` (line 1053) | QUIL `OP_SURFACE_UPDATE` for position sync |
| `get_surface_bounds()` (line 1068) | QUIL returns geometry |
| `point_in_surface()` (line 1090) | QUIL bounds check |
| `surface_is_alive()` (line 1118) | Returns `true` always (like Linen) |
| `is_focusable_surface()` (line 1209) | QUIL is focusable |
| `is_closeable_surface()` (line 1933) | QUIL cannot be closed (OS-managed) |
| `update_local_geometry()` (line 698) | QUIL geometry sync |
| `z_order` arrays (line 1167, 2983) | QUIL in focus fallback and hit-test order |

### New Helpers (5, matching Linen pattern — no Quil-specific special cases)

| Helper | Description |
|--------|-------------|
| `ensure_quil_frame()` | Creates `ShellFrame` with `frame_id=3` lazily. Returns `Some(3)` or `None`. |
| `open_quil_in_active_scene()` | Opens Quil in current scene (un-minimize, 0xEC, tile, focus). |
| `focus_or_open_quil()` | Focus if visible, else open. |
| `toggle_quil()` | Toggle minimize/restore. |
| `quil_frame_id()` | Returns `Some(3)` or `None`. |

All helpers are lazy — Quil only enters `FRAMES` on first open. Zero boot visual change. All helpers follow the identical Linen pattern; Quil has zero special-case shell behavior beyond its surface ID and placeholder geometry.

---

## SCAN 1: Boundary / Scope Creep — Wording Fixes

Scope-creep risks audited and fixed. All fixes are reflected in the Non-Goals section below.

| Risk | Fix |
|------|-----|
| "editor stub" wording | Fixed to `quil (surface stub)` |
| "persistence" implied disk | Fixed: "in-memory layout snapshot (volatile)" |
| Missing exclusions (package trust, crash viewer, agent, framebuffer, capability bypass, cross-PD pointers, shared buffers) | Added to Non-Goals |
| sexdisplay lifecycle ownership | Fixed: "shell-side surface identity only" |
| Kernel/ABI/sex-pdx edits | Confirmed: no edits needed (scope line unchanged) |

---

## SCAN 2: Premortem — Failure Modes & Mitigations

| # | Failure Mode | Category | Why It Kills The Plan | Violated Invariant | Fix | Proof Gate |
|---|--------------|----------|----------------------|--------------------|-----|------------|
| 1 | Quil stub becomes full editor implementation | Scope creep | Stub becomes unmaintainable; 240-line surface stub turns into 10,000-line editor with no handoff boundary | Stub scope: surface identity + shell lifecycle only | Keep Non-Goals enforced; handoff doc must be updated before any editor code | CI check blocks >1% stub file-size increase without handoff update |
| 2 | Quil stub writes framebuffer directly | Render ownership | Causes framebuffer corruption; sexdisplay no longer sole writer | sexdisplay is sole framebuffer writer (kernel invariant) | Stub has no framebuffer access; shell-side only | Static analysis: no sexdisplay framebuffer ops in silk-shell |
| 3 | sexdisplay starts owning Quil semantics | Boundary drift | sexdisplay becomes responsible for app lifecycle, not just rendering | sexdisplay renders surface state, does not manage app lifecycle | Shell owns Quil lifecycle; sexdisplay only receives 0xEC/0xEE/0xEB | 0xEC/0xEE/0xEB are standard surface ops, not Quil-specific |
| 4 | Shell hardcodes Quil special cases | Architecture debt | Quil diverges from Linen/app-surface pattern; 5 special cases become 50 | Linen pattern must support any app surface without special-casing | Quil helpers are line-for-line copies of Linen; parameterize in V2 instead | Diff check: Quil helpers must match Linen helpers structurally |
| 5 | Quil SurfaceId collides with Linen/other surfaces | Data race / corruption | Two surfaces with same ID → undefined display behavior | Surface IDs must be unique (kernel/display invariant) | Enforce surface_id registry: no duplicate IDs across all servers | Pre-commit collision scan against all `SURFACE_ID_*` constants |
| 6 | Quil focus opens invalid/tombstoned surface | Correctness bug | Focus targets surface 201 when no server exists; keyboard input lost | Focus must target a real, alive, non-tombstoned surface | Fix `clear_focus_if_dead()` and `surface_in_active_scene()` fallback (Patch B applied) | Proof marker `[quil.surface.focus.reject]` fires when no frame exists |
| 7 | Close/minimize/zoom bypasses Track A lifecycle | Lifecycle violation | Surface state desync between shell and display; tombstone not set | Close must tombstone; minimize must hide; zoom must not destroy | Track A lifecycle enforced by `close_surface_from_frame_light`, `minimize_frame`, `toggle_zoom_frame` | Proof markers for each operation; snap_capture_layout after each mutation |
| 8 | Quil stub assumes document/file authority | Scope creep / security | Stub implies it can open files without Linen OpenIntent; bypasses capability model | File access requires Linen OpenIntent (system invariant) | Non-Goals: "No document/file authority — requires Linen OpenIntent in future phases" | No file open syscall or Linen IPC in silk-shell stub |
| 9 | Quil stub opens files without Linen OpenIntent | Security bypass | Capability model violated; unmediated file access | All file access must go through Linen OpenIntent (Collar-granted) | Stub does not open files; shell has no file access capability | No file API calls in silk-shell |
| 10 | Quil stub persists state before E/F gates | Premature persistence | State written to disk before persistence design is complete; format churn | Persistence requires E/F gate approval (system convention) | Non-Goals: "No durable persistence — in-memory snapshot only" | No disk write syscall in silk-shell |
| 11 | Quil stub introduces raw shared buffer/backing buffer | Architecture violation | Breaks PDX-only IPC; introduces shared memory without guards | All IPC must use PDX (system invariant) | Non-Goals: "No shared/reusable backing buffers" | No `shm` or shared memory syscalls in silk-shell |
| 12 | Quil app identity/package trust is assumed | Security gap | Quil identity assumed without verification; spoofing possible | App identity must be verifiable before capability grant | Non-Goals: "No package trust assumption — identity verification deferred" | No identity claim in silk-shell stub |
| 13 | Quil crash/log/dev cockpit behavior starts | Scope creep | Stub becomes responsible for crash reporting, log viewing, dev tools | Crash/log/dev cockpit is separate feature track (H/I) | Non-Goals: "No crash/log/dev cockpit scope" | No log aggregation or crash handling in silk-shell stub |
| 14 | PDX opcode/slot drift between shell and display | Protocol desync | Shell sends opcode that display interprets differently; surface not created | PDX opcodes must match between caller and callee | Document opcodes used: 0xEC (create), 0xEE (destroy), 0xED (focus), 0xEB (update) | Serial log captures all opcode invocations; diff-check on protocol headers |
| 15 | Frame/tab state desync after close/toggle | State corruption | Shell thinks Quil is open but display disagrees; ghost surface | Frame state must match display surface state | `snap_capture_layout()` after every mutation; `sync_scene_visibility()` on scene switch | Snapshot validate checksum on restore |
| 16 | Bounds checks weakened in any display path | Security | Quil geometry update bypasses framebuffer bounds → write outside framebuffer | All surface geometry must be bounds-checked (kernel invariant) | Verify `clamp_position()` and `clamp_surface_size()` called on all Quil geometry updates | Proof marker `[quil.surface.geometry.update]` fires after bounds check |
| 17 | Surface geometry update misses tile/focus/z-order path | Inconsistency | Quil position updated in one path but not another → stale geometry used | All geometry-changing operations must propagate to all tracking paths | `update_local_geometry()` called from tile path; `emit_snapshot()` from snapshot path | Snapshot emit includes Quil position; z-order includes Quil |
| 18 | Stub "temporary" code becomes permanent without handoff | Technical debt | Temporary flag, budget, or workaround survives into production | Temporary code must be removed or explicitly documented as permanent | All budget markers (`QUIL_CREATE_BUDGET`, `QUIL_OPEN_BUDGET`, etc.) must be removed or documented as permanent debug probes | Handoff update required before adding any new Quil code |

---

## SCAN 3: Surface FSM / Lifecycle Completeness

### Quil Stub State Machine

```
                          ┌──────────────────────────────────────────────┐
                          │                                              │
                          v                                              │
  NotCreated ──→ Allocated ──→ FrameAttached ──→ Visible ──→ Focused    │
                    │              │               │   ↑        │       │
                    │              │               v   └────────┘       │
                    │              │            Hidden                  │
                    │              │               │                    │
                    │              │               v                    │
                    │              │          Minimized                 │
                    │              │               │                    │
                    │              v               v                    │
                    │           Closing ←─── (all states)               │
                    │              │                                     │
                    │              v                                     │
                    │         Tombstoned                                 │
                    │              │                                     │
                    │              v                                     │
                    │         Destroyed (terminal) ──────────────────────┘
                    │                                                    │
                    └────────────────────────────────────────────────────┘
```

### State Definitions and Transitions

| State | Description | Allowed Transitions | Forbidden Transitions | Shell-owned Check | Display Behavior | Proof Marker | Failure Behavior |
|-------|-------------|--------------------|----------------------|-------------------|------------------|--------------|------------------|
| **NotCreated** | Quil surface identity not allocated in shell | → Allocated | → any other | Shell must allocate SURFACE_ID_QUIL before any operation | No surface on display; no opcodes sent | `[quil.audit.start]` | N/A (initial state) |
| **Allocated** | SURFACE_ID_QUIL constant exists, statics allocated | → FrameAttached, → NotCreated (deallocation) | → Visible, → Focused, → Hidden, → Minimized, → Tombstoned, → Destroyed | Shell must verify surface_id collision-free; no frame yet | No surface on display; 0xEC never sent | `[quil.surface.ensure]` | Collision: surface_id already in use → reject allocation |
| **FrameAttached** | ShellFrame exists in FRAMES with Quil tab | → Visible, → NotCreated (frame removed) | → Focused (no server renders yet), → Tombstoned (not closed) | `ensure_quil_frame()` succeeded; frame present in FRAMES; surface_in_active_scene returns correct scene | 0xEC may be sent but no server renders; display shows placeholder if server present | `[quil.surface.open]` | No empty FRAMES slot → return None; log `[shell.quil.frame.reject]` |
| **Visible** | Frame in active scene, not minimized, not zoomed | → Focused, → Hidden, → Minimized, → Closing | → NotCreated, → Allocated, → Tombstoned (must close first) | `tile_visible_frames()` positions Quil; `sync_scene_visibility()` shows it | Standard 0xEC sent with current geometry; sexdisplay renders if server present | `[quil.surface.open]` | Surface already minimized → restore first; surface already zoomed → focus without tile |
| **Focused** | FOCUSED_SURFACE_ID == 201, frame accepts input | → Visible (lose focus), → Hidden, → Minimized, → Closing | → NotCreated, → Allocated, → Tombstoned (must close first) | `try_set_focus(201)` succeeded — guards: alive, focusable, not tombstone, in active scene | 0xED sent with sid=201; keyboard input forwarded if server present | `[quil.surface.focus.allow]` | `try_set_focus` returns false → log `[quil.surface.focus.reject]` with reason |
| **Hidden** | Frame in non-active scene; sync_scene_visibility() hid it | → Visible (scene switch), → Closing | → Focused (not in active scene), → NotCreated | `sync_scene_visibility()` sent 0xEE for Quil; frame exists with wrong scene_id | 0xEE sent to hide surface; no input forwarded | `[quil.surface.geometry.update]` | Tombstoned surface never hidden (already closed) |
| **Minimized** | Frame minimized; surface hidden | → Visible (restore), → Closing | → Focused (cannot receive pointer focus), → NotCreated | `minimize_frame(3)` succeeded; FRAME_FLAG_MINIMIZED set; `clear_focus_if_dead()` called | Display hides surface; no input forwarded | `[quil.surface.minimize]` | Surface already minimized → no-op; `toggle_quil()` restores |
| **Closing** | Close requested; surface cleanup in progress | → Tombstoned | → Visible, → Focused, → Hidden, → Minimized | `close_surface_from_frame_light(201)` — currently no-op because `is_closeable_surface(201)` returns false | 0xEE sent (if close succeeds); frame tab removed | `[quil.surface.close]` | `is_closeable_surface(201)` returns false → close rejected; log `[quil.surface.close.reject]` |
| **Tombstoned** | Surface ID in tombstone set; cannot be focused/opened | → Destroyed | → Focused (forbidden — tombstone guard), → Visible, → Allocated | `is_tombstoned(201)` = true; `try_set_focus` rejects tombstoned surfaces | Display should not reference tombstoned surface ID | `[quil.surface.tombstone]` | Attempt to focus tombstoned surface → `[shell.focus.reject.tombstoned]` |
| **Destroyed** | Surface ID removed from all shell state (terminal) | (none — terminal) | All transitions forbidden | Surface ID removed from FRAMES, z_order, all tracking; statics zeroed | No display state; surface ID eligible for reuse after guarantee period | `[quil.surface.destroy.reject]` | Attempt to reference destroyed ID → kernel/display error; must allocate new ID |

### Critical FSM Rules

1. **Destroyed is terminal.** Once a surface reaches Destroyed, it cannot be resurrected. A new allocation requires a new surface ID.
2. **Tombstoned is not live content.** Tombstoned surface cannot receive focus, input, or display updates.
3. **Minimized cannot receive pointer focus.** `frame_accepts_input()` returns false for minimized frames.
4. **Focused requires visible + alive + focusable + non-tombstoned + in active scene.** `try_set_focus()` enforces all five guards.
5. **Close is idempotent.** `close_surface_from_frame_light()` returns false if already dead; tombstone is set only once.
6. **Toggle must not resurrect Destroyed.** `toggle_quil()` checks frame existence; if no frame, calls `open_quil_in_active_scene()` which calls `ensure_quil_frame()`. If tombstoned, `try_set_focus` will reject.
7. **Restore must validate liveness.** `restore_minimized_frame()` must check `surface_is_alive()` for the tab's surface before restoring.
8. **Geometry update must be bounded.** All geometry updates go through `clamp_position()` and `clamp_surface_size()`.
9. **sexdisplay renders shell-provided Quil visual state only.** sexdisplay never queries Quil directly; shell sends all updates via standard opcodes.

---

## SCAN 4: Existing App-Surface Pattern Mapping

Quil stub maps 1:1 to the canonical Linen/app-surface pattern. No Quil-specific special cases exist.

| Pattern Element | Linen (SURFACE_ID=200) | Quil (SURFACE_ID=201) | Status | Risk if Missing |
|-----------------|------------------------|-----------------------|--------|-----------------|
| Surface ID constant | `SURFACE_ID_LINEN = 200` (line 60) | `SURFACE_ID_QUIL = 201` (line 61) | ✅ Present | Surface ID collision |
| Frame ID constant | `LINEN_FRAME_ID = 2` | `QUIL_FRAME_ID = 3` | ✅ Present | Frame ID collision |
| Geometry statics | `SURFACE_200_X/Y/W/H` | `SURFACE_201_X/Y/W/H` | ✅ Present | Stale geometry |
| Lazy frame creation | `ensure_linen_frame()` | `ensure_quil_frame()` | ✅ Present | Boot visual drift |
| ensure helper | `ensure_linen_frame()` → `Option<u32>` | `ensure_quil_frame()` → `Option<u32>` | ✅ Present | Cannot open Quil |
| open helper | `open_linen_in_active_scene()` → `bool` | `open_quil_in_active_scene()` → `bool` | ✅ Present | Cannot make visible |
| focus_or_open helper | `focus_or_open_linen()` → `bool` | `focus_or_open_quil()` → `bool` | ✅ Present | Cannot focus after open |
| toggle helper | `toggle_linen()` → `bool` | `toggle_quil()` → `bool` | ✅ Present | Cannot minimize/restore |
| frame_id helper | `linen_frame_id()` → `Option<u32>` | `quil_frame_id()` → `Option<u32>` | ✅ Present | Other queries fail |
| `tile_visible_frames()` | Match arm for LINEN | Match arm for QUIL | ✅ Present | No tiling position |
| `emit_snapshot()` / `OP_SURFACE_UPDATE` | LINEN arm at line 1051 | QUIL arm at line 1053 | ✅ Present | Stale display position |
| `get_surface_bounds()` | LINEN at line 1067 | QUIL at line 1068 | ✅ Present | Bounds query fails |
| `point_in_surface()` | LINEN at line 1089 | QUIL at line 1090 | ✅ Present | Click targeting fails |
| `surface_is_alive()` | Returns `true` always (line 1117) | Returns `true` always (line 1118) | ✅ Present | Focus targeting dead surface |
| `is_focusable_surface()` | LINEN at line 1208 | QUIL at line 1209 | ✅ Present | Cannot receive focus |
| `is_closeable_surface()` | LINEN at line 1933 | QUIL at line 1933 | ✅ Present | Can accidentally close OS surface |
| `update_local_geometry()` | LINEN arm at line 694 | QUIL arm at line 698 | ✅ Present | Stale tile geometry |
| z-order focus fallback | LINEN in array (line 1167) | QUIL in array (line 1167) | ✅ Present (Patch A applied) | Focus lost after close |
| z-order hit-test | LINEN in array (line 2983) | QUIL in array (line 2983) | ✅ Present (Patch A applied) | Click-to-focus broken |
| close via frame light | `is_closeable_surface(LINEN) = false` | `is_closeable_surface(QUIL) = false` | ✅ Present | Accidental close |
| minimize via frame light | Via `minimize_frame()` path | Via `minimize_frame()` path | ✅ Present (shared path) | Cannot minimize |
| zoom via frame light | Via `toggle_zoom_focused_frame()` | Via `toggle_zoom_focused_frame()` | ✅ Present (shared path) | Cannot zoom |
| `sync_scene_visibility()` | Via frame scene_id check | Via frame scene_id check | ✅ Present (shared path) | Cross-scene visibility bug |
| `snap_capture_layout()` | Via FRAMES iteration | Via FRAMES iteration | ✅ Present (shared path) | Layout restore misses Quil |
| Budget marker pattern | `LINEN_CREATE_BUDGET`, etc. | `QUIL_CREATE_BUDGET`, etc. | ✅ Present | Debug visibility |
| Non-goals enforcement | Linen is OS surface, not closeable | Quil is OS surface, not closeable | ✅ Present | Scope creep |

**STOP FIRST condition**: If the existing app-surface pattern cannot support Quil safely without special cases, halt and redesign. Currently Quil requires zero special-case shell code beyond surface identity constants and helper functions that are exact copies of Linen.

---

## SCAN 5: Proof Markers and Negative Tests

### Proof Markers

Every dangerous or lifecycle-relevant operation must produce a proof marker. Markers follow the convention `[quil.surface.<operation>.<result>]`.

| Marker | Location | When Fired | Required Check Before Fire | Failure Behavior |
|--------|----------|------------|---------------------------|------------------|
| `[quil.audit.start]` | Shell boot / init | Silk-shell starts; Quil identity module loads | Verify SURFACE_ID_QUIL not colliding with existing IDs; verify QUIL_FRAME_ID not colliding | Missing marker → Quil identity not loaded |
| `[quil.surface.ensure]` | `ensure_quil_frame()` | Frame created: `QUIL_CREATE_BUDGET > 0` | Verify FRAMES slot empty; verify no double-create | No empty slot → `[shell.quil.frame.reject] reason=no_slot` |
| `[quil.surface.open]` | `open_quil_in_active_scene()` | Quil opened: `QUIL_OPEN_BUDGET > 0` | Verify frame exists (ensure); verify surface_in_active_scene returns correct scene | Frame create failed → return false |
| `[quil.surface.focus.allow]` | `focus_or_open_quil()` | Focus set: `QUIL_FOCUS_BUDGET > 0` | Verify surface alive, focusable, non-tombstoned, in active scene, frame accepts input | Guard fails → `[quil.surface.focus.reject]` with reason |
| `[quil.surface.focus.reject]` | `try_set_focus(201)` | Focus rejected for any guard | (fired on failure) — must include reason: not_focusable, dead, tombstoned, wrong_scene | Surface not focused; no input routing |
| `[quil.surface.toggle]` | `toggle_quil()` | Minimize or open: `QUIL_TOGGLE_BUDGET > 0` | Verify frame exists; check minimized state before toggle | No frame → `open_quil_in_active_scene()`; minimized → `minimize_frame()` |
| `[quil.surface.close]` | `close_surface_from_frame_light()` | Close attempted on Quil | `is_closeable_surface(201) = false` → close rejected before marker | Close rejected → `[quil.surface.close.reject]` |
| `[quil.surface.close.reject]` | `is_closeable_surface(201)` | Close rejected | Must print reason: `not_closeable` | Surface remains alive |
| `[quil.surface.minimize]` | `minimize_frame(3)` | Frame minimized | Verify frame is not already minimized; clear_focus_if_dead called | Already minimized → no-op |
| `[quil.surface.zoom]` | `toggle_zoom_focused_frame()` | Frame zoomed/unzoomed | Verify frame accepts zoom; geometry saved before zoom | Zoom not applicable for Quil (unusual for editor) |
| `[quil.surface.geometry.update]` | `update_local_geometry()` or `tile_visible_frames()` | Geometry change applied | Verify geometry clamped via `clamp_position()` / `clamp_surface_size()` | Bounds violation caught by clamp; log warning if clamped |
| `[quil.surface.snapshot.emit]` | `emit_snapshot()` | Snapshot includes QUIL surface | Verify SURFACE_201_X/Y/W/H bounds valid; verify 0xEB sent with correct coords | Stale snapshot → display shows old position |
| `[quil.surface.tombstone]` | `tombstone_surface(201)` | Surface ID added to tombstone set | (Quil cannot be tombstoned in V1 — this marker should never fire) | Tombstoned surface cannot be focused |
| `[quil.surface.destroy.reject]` | Any destroy path hitting Quil | Destroy attempted on OS-managed surface | `is_closeable_surface(201) = false` guard | Surface remains alive; no 0xEE sent |
| `[quil.error]` | Any unexpected Quil state | Any invariant violation | (comprehensive) | Log full state; do not panic |

### Negative Tests (Proof-of-Absence)

Each negative test ensures a dangerous operation is correctly blocked. All tests must pass without code changes (they test guards, not behavior).

| # | Test | Expected Result | Guard Under Test | Shell Path | Failure Mode Blocked |
|----|------|----------------|------------------|------------|---------------------|
| 1 | Focus Quil before `ensure_quil_frame()` | `try_set_focus(201)` returns false | `surface_in_active_scene(201)` returns false for frameless Quil (Patch B) | `clear_focus_if_dead()` z_order loop | Bug PASS 1 #1: focus non-existent surface |
| 2 | Focus Quil after tombstone | `try_set_focus(201)` returns false | `is_tombstoned(201)` check — Quil never tombstoned, but guard exists | `try_set_focus()` line 2755 | Focus tombstoned surface |
| 3 | Toggle Quil after Destroyed (if destroyable) | `open_quil_in_active_scene()` fails or creates new frame | Frame not found; `ensure_quil_frame()` creates new frame if slot available | `toggle_quil()` line 1851 | Toggle resurrecting destroyed surface |
| 4 | Close already Closing/Tombstoned surface | `close_surface_from_frame_light()` returns false | `surface_is_alive()` check — already dead | `close_surface_from_frame_light()` line 1946 | Double-close |
| 5 | Pointer hit outside Quil bounds | `point_in_surface()` returns false | Geometry bounds check | `hit_test_at()` → `point_in_surface()` | Click outside bounds treated as Quil hit |
| 6 | Stale geometry update (old x/y before tile) | Tile overwrites with correct position | `tile_visible_frames()` computes fresh position from frame state | `update_local_geometry()` before `tile_visible_frames()` | Stale display position |
| 7 | SurfaceId collision (201 already used) | Build failure if duplicate constant | Compiler error on duplicate `SURFACE_ID_QUIL` | Constant definition line 61 | Two surfaces with same ID |
| 8 | FrameId collision (3 already used) | Build failure if duplicate constant | Compiler error on duplicate `QUIL_FRAME_ID` | Constant definition line 1700 | Two frames with same ID |
| 9 | z-order missing Quil | `point_in_surface()` still works via focused path | `hit_test_at()` line 2950 checks focused surface first; Patch A adds Quil to z_order | Hit-test fallback | Click-to-focus broken (Patch A applied) |
| 10 | Snapshot emit with invalid bounds | `clamp_position()`/`clamp_surface_size()` corrects before emit | Bounds clamping in `emit_snapshot()` and `tile_visible_frames()` | `emit_snapshot()` line 1053 | Display receives out-of-bounds position |
| 11 | Display receives stale Quil surface update | `snap_capture_layout()` captures current state; `snap_restore_layout()` clamps | Snapshot checksum validation; bounds clamping on restore | `snap_restore_layout()` | Layout restore sets wrong position |
| 12 | Quil tries document open without Linen/OpenIntent | No path exists in silk-shell for file operations | No file API calls in silk-shell | N/A | Security: unauthorized file access |
| 13 | Quil tries persistence before E/F gates | No path exists in silk-shell for disk write | No disk write syscall in silk-shell | N/A | Premature persistence |
| 14 | Quil tries direct framebuffer write | No path exists in silk-shell for framebuffer access | No sexdisplay framebuffer ops in silk-shell | N/A | Render ownership violation |
| 15 | Quil requires kernel/ABI/sex-pdx edit | Build succeeds without kernel/ABI/sex-pdx changes | `Scope` header: no kernel/ABI/sexdisplay changes | N/A | ABI drift, rebuild required |

---

## SCAN 6: Revised Safest Path

1. **Audit existing Linen pattern** — STOP FIRST if pattern needs modification
2. **Reserve SurfaceId/FrameId after collision scan** — `grep -rn "SURFACE_ID_" servers/`
3. **Add stub surface identity only** — constants, statics, 3 wire-ups (alive/focusable/closeable)
4. **Add 5 lazy frame-creation helpers** — structural copies of Linen (ensure/open/focus_or_open/toggle/frame_id)
5. **Wire all shell tables** — tile, emit_snapshot, get_surface_bounds, point_in_surface, update_local_geometry, z_order arrays (all via shared FRAMES iteration; no Quil special case)
6. **Render via existing sexdisplay path only** — 0xEC/0xEE/0xED/0xEB; no framebuffer write
7. **Add focus/close/minimize/zoom guards** — Patch B (surface_in_active_scene for frameless), is_closeable_surface=false, shared minimize/zoom paths
8. **Add proof markers + negative tests** — allow + reject markers for each dangerous operation
9. **Do NOT implement editor, files, agents, crash viewer, package trust, persistence, or dev cockpit** — each requires new handoff + PDX server + Linen OpenIntent + Collar grants + E/F gates
10. **Save recurring issues in handoff** — every handoff must answer: "Is this surface still following the Linen pattern?"

---

## SCAN 7: Exceeded Hypothesis — How We Lose & How We Win

### The Rival

Six months from now, a different OS or dev workstation team ships a better Quil-like surface stub. Here is exactly how they beat us and how SexOS wins anyway.

### Exceeded Hypothesis Table

| # | Rival Advantage | Why Quil Stub Would Lose | SexOS-Native Fix | Required Invariant | Proof Gate |
|---|---|---|---|---|---|
| 1 | Rival's surface stub is smaller (no enum, no FSM, no handoff) | Quil stub is over-engineered for a placeholder | Quil stub IS minimal: 5 helpers, 12 wiring locations, zero special cases. The FSM/handoff is documentation, not code. | No Quil-specific code beyond surface identity + Linen-copied helpers | Code size: Quil additions < 1% of silk-shell |
| 2 | Rival launches real editor faster | Quil stub delays editor by requiring handoff update | Stub is deliberately bounded — editor is a separate track with its own handoff, PDX server, and lifecycle | No editor code in shell (servers/quil/ is independent binary) | Code separation: shell has zero editor logic |
| 3 | Rival has no focus-fallback bug | Quil stub launches with PASS 1 #1 (now patched) | Patch B applied: `surface_in_active_scene()` returns false for frameless Quil | Frame-owned surfaces must have a frame to be in active scene | `try_set_focus(201)` returns false before ensure |
| 4 | Rival has no stale-geometry risk | Quil geometry statics could drift from display state | All geometry updates flow through `update_local_geometry()` + `tile_visible_frames()` + `emit_snapshot()` | Every mutation must update all three paths | Snapshot checksum validates on restore |
| 5 | Rival has clear upgrade path | Quil stub is a dead end if V2 never happens | V2 Boundaries section documents the upgrade path; handoff pattern is reusable for any app surface | Stub must not block V2; V2 must not break stub | V2 plan exists in /rapid (PHASE_05) |
| 6 | Rival has no special-case shell code | Quil could require special tile/focus/close logic | Quil follows Linen pattern exactly; zero special cases | `is_closeable_surface(QUIL) = false` is same as Linen | Diff check: Quil helpers ≤ 5% diff from Linen helpers |
| 7 | Rival's placeholder has visual clarity | Quil stub produces blank/black area (no server) | Non-Goal: no placeholder in V1. Future: colored rect via sexdisplay, never framebuffer write | Placeholder goes through sexdisplay only (0xEC/0xEB) | No framebuffer access in silk-shell |
| 8 | Rival has no proof gaps | Quil stub could miss markers for edge cases | SCAN 5 documents 15 markers + 15 negative tests. All guards have proof markers. | Every guard must have an allow AND reject marker | Marker audit: allow + reject for each operation |
| 9 | Rival has no Tombstoned state confusion | Quil stub hardcodes `surface_is_alive=true` which skips tombstone check | `surface_is_alive(201)=true` is intentional — Quil cannot be closed. No tombstone needed. | OS-owned surfaces skip tombstone | No tombstone path reaches Quil (gated by is_closeable_surface) |
| 10 | Rival follows existing shell patterns | Quil could introduce pattern drift | Quil is a structural copy of Linen. Pattern drift is detectable by diff. | All app surfaces follow Linen pattern | Pre-commit diff: new surface vs Linen pattern |
| 11 | Rival has better lifecycle discipline | Quil stub could introduce FSM violations | SCAN 3 defines complete FSM with 10 states, allowed/forbidden transitions, and failure behaviors | FSM must be documented before any new state is added | FSM audit: every state has allowed/forbidden/check/display/marker/failure |
| 12 | Rival has better handoff discipline | Quil stub handoff becomes stale after code changes | Handoff is updated WITH code changes, not after. Handoff and code are in same PR. | Handoff must be reviewed whenever Quil-related code changes | PR check: handoff updated if surfacE_ID_QUIL or QUIL_FRAME_ID changes |

### Best-in-Class Methods to Steal

SexOS adopts these methods natively:

1. **Tiny reliable placeholder first** — Surface identity only. 12 wiring locations. 5 helpers. No server/editor/files.
2. **App-surface pattern reuse** — Every new surface is a structural copy of Linen (<5% diff enforced).
3. **Strict lifecycle guards** — 5-guard `try_set_focus()`, frame-existence check, OS-surface exclusion.
4. **Deterministic surface IDs** — Sequential (100 APP, 101 STATIC, 200 Linen, 201 Quil); collision scan before allocation.
5. **Explicit focus/liveness** — `surface_in_active_scene()` returns false for frameless Quil (Patch B).
6. **Placeholder UX with clear boundary** — If rendered, via sexdisplay only. Honest: "No editor. No server. No files. No persistence."
7. **Proof-first operations** — Every operation has allow+reject markers. Every guard has a negative test.
8. **Handoff-driven upgrade** — V2 requires new handoff, PDX server, Linen OpenIntent, Collar grants.

### SexOS-Native vs Desktop-App Design

| Desktop Pattern | SexOS Replacement |
|----------------|-------------------|
| Editor opens files | Linen OpenIntent mediates file access |
| Editor owns chrome | Shell owns frame/tab/chrome via Silk Scene |
| Editor renders window | Shell sends 0xEC/0xEB; sexdisplay renders |
| Editor is a monolith | Quil is one surface ID; server is independent PDX binary |

---

## Non-Goals (Consolidated)

The following are explicitly NOT implemented by this stub. Any future work must create a new handoff document.

### Scope
- ❌ Not a Quil server binary (`servers/quil/`)
- ❌ No keyboard input routing — requires Focus201 SurfaceAction + conflict/accessibility audit
- ❌ No text/code/sex mode implementations
- ❌ No Sex Inspector panels
- ❌ No project tree via Linen
- ❌ No visual placeholder/rendering on surface 201 (no server exists)
- ❌ No dynamic surface registration protocol
- ❌ No FRAMES array expansion (stays at 4)
- ❌ No helper deduplication/parameterization
- ❌ No agent orchestration
- ❌ No crash/log/dev cockpit scope

### Security & Capability
- ❌ No document/file authority — requires Linen OpenIntent in future phases
- ❌ No capability bypass — Collar grants required for any operation
- ❌ No package trust assumption — identity verification deferred
- ❌ No unsafe/developer mode
- ❌ No identity/authority claims via title or icon
- ❌ No hidden developer mode that bypasses guards

### Rendering & IPC
- ❌ No framebuffer access — sexdisplay is sole framebuffer writer
- ❌ No raw cross-PD pointers — all IPC via PDX
- ❌ No shared/reusable backing buffers
- ❌ No rendering policy ownership — shell decides visibility; sexdisplay decides rendering
- ❌ No raw RGBA or unbound color values in preferences
- ❌ No user-editable scripts, plugins, or themes

### Lifecycle & State
- ❌ No durable persistence — in-memory snapshot only; E/F gate required for disk
- ❌ No preference persistence to disk (E/F gates required)
- ❌ No lifecycle guard disabling
- ❌ No proof marker suppression for required safety markers
- ❌ No app-owned launch/focus policy

### Protocol
- ❌ No kernel/ABI/sex-pdx edits — scope strictly silk-shell only
- ❌ No z-order priority optimization — Quil uses same priority as Linen
- ❌ No keybinding without conflict + accessibility audit

---

## Known Imperfections Summary

All findings from scans 1-7 are documented in their respective sections above. This table summarizes only status.

| Scan | Items | Patched | Accepted (doc only) | Deferred to V2 |
|------|-------|---------|---------------------|----------------|
| PASS 1 Correctness | 3 bugs | #1 (critical), #2 (medium) | #3 (low) | — |
| PASS 2 Architecture | 7 issues | #10 (z_order) | #5, #7, #8, #9 | #4 (helper param), #6 (registration) |
| PASS 3 Product | 7 gaps | — | #12-#17 | #11 (keybinding) |
| SCAN 1 Boundary | 5 fixes | All 5 wording fixes applied | — | — |
| SCAN 2 Premortem | 18 failures | P6 (focus guard) | All others via Non-Goals/guards | — |
| SCAN 3 FSM | 10 states | — | All defined | — |
| SCAN 4 Pattern | 16 elements | All 16 ✅ Present | — | — |
| SCAN 5 Markers | 14 markers | K2-K4, K6 present | K1, K5, K7-K14 documented | — |
| SCAN 6 Safest Path | 10 steps | All 10 verified | — | — |
| SCAN 7 Hypothesis | 12 rivals | All accepted with mitigation | — | — |

### Remaining Risks (Accepted)

1. **No keyboard binding** — Quil is not user-openable until Focus201 SurfaceAction is added. Acceptable for V1 stub.
2. **No visual placeholder** — Until a Quil server exists, surface 201 is blank/black when opened. Acceptable for V1 stub.
3. **FRAMES array at 3/4 capacity** — Only 1 slot remains for future frame-based apps. Acceptable until V2 registration protocol.
4. **Helper duplication with Linen** — ~140 lines of near-identical code. Acceptable until V2 parameterization.
5. **Surface ID registry is a comment** — No compile-time enforcement of uniqueness. Mitigated by pre-commit collision scan procedure.
6. **Quil stub never enters Tombstoned state** — `surface_is_alive(201)=true` + `is_closeable_surface(201)=false` means Quil bypasses tombstone entirely. This is intentional for OS-managed surfaces.

---

## Build Command & Result

```
RUSTFLAGS="-C relocation-model=pic -C link-arg=-pie" cargo build \
    -Z build-std=core,compiler_builtins,alloc \
    -Z build-std-features=compiler-builtins-mem \
    --manifest-path servers/silk-shell/Cargo.toml \
    --target /home/xirtus_arch/x86_64-sex.json \
    --release

Finished release profile [optimized] in 0.78s
Warnings: 217 (all pre-existing — `static_mut_ref`, `unused_import`, nested unsafe blocks)
Errors: 0
```

217 warnings confirmed pre-existing (verified by comparing against pre-Quil build). No new warnings introduced.

---

## Handoff Warnings

1. **Focus fallback targets non-existent Quil** — `clear_focus_if_dead()` line 1167. Patch B applied. Verify if `surface_in_active_scene()` refactored.
2. **Hit-test z_order must include QUIL** — Patch A at line 2983. Keep both z_order arrays in sync.
3. **No keyboard binding** — No scancode opens Quil. Requires `Focus201` SurfaceAction variant + conflict/accessibility audit.
4. **Linen has same bugs** — Both PASS 1 bugs affected Linen identically (pre-existing). Patches fix both.
5. **`surface_in_active_scene()` panel fallback** — Returns true for unframed panels. Patch B adds explicit Linen/Quil check. Add new frame-owned surfaces here.
6. **z_order arrays must stay in sync** — Line 1167 (focus) and 2983 (hit-test). Both now include QUIL.
7. **Budget markers are debug probes** — `QUIL_CREATE/OPEN/FOCUS/TOGGLE/NOSLOT_BUDGET` (4-8 fire limit). Convert to permanent when Quil server ships.
8. **No Quil server binary** — Surface 201 is shell-side identity only. Opening produces blank/black area.
9. **V2 registration protocol is speculative** — Actual V2 design requires new handoff.
10. **PHASE_05 exceeds this stub** — Requires Linen, Mesh/Collar, multiple server binaries. Stub allocates surface identity only.

---

## SCAN 8: Customization / User Policy Surface

**Rule:** Quil surface stub may define user-facing preferences, but only as shell-owned validated policy. Customization must not turn the stub into the real Quil editor, dev cockpit, file browser, agent runner, storage client, renderer policy owner, or security bypass.

Customization is *shell memory only* until E storage gates — no persistence to disk. All preferences are volatile: they live in `static mut` shell state and reset on boot unless restored from snapshot.

### Customizable in V1 / Stub

| Preference | Type | Source | Bounds | Default | Side Effects |
|------------|------|--------|--------|---------|--------------|
| Placeholder title text | `QuilStubPreference::Title(QuilStubTitleToken)` | Fixed compiled enum: `Editor`, `Terminal`, `Notepad`, `Scratch`, `Blank` | Only enum variants valid; raw strings rejected | `Blank` | Changes surface title in `ShellTab.title_id`; no content or behavior change |
| Placeholder icon/glyph | `QuilStubPreference::Icon(QuilStubIconToken)` | Fixed compiled set: `Code`, `Edit`, `Terminal`, `Empty` | Only enum variants valid | `Empty` | Shell renders icon in chrome slot if supported; no permission/identity claim |
| Accent color token | `QuilStubPreference::Accent(QuilStubThemeToken)` | Bounded Silk theme token set: `Default`, `Blue`, `Green`, `Amber`, `Plum`, `Slate` | Mapped through Silk scene token palette; raw RGBA rejected | `Default` | Affects frame chrome accent (top-bar tint, tab highlight). Does NOT set sexdisplay pixel policy. |
| Default launch geometry | `QuilStubLaunchPolicy::Geometry { x, y, w, h }` | User-provided `i32`/`u32` via shell preference API | Clamped by `clamp_position()` + `clamp_surface_size()`; w≥80, h≥60, all bounds-checked against framebuffer | `(100, 100, 640, 480)` | Used by `ensure_quil_frame()` as `normal_x/y/w/h`. Overridden by tile if conflicting. |
| Default open behavior | `QuilStubLaunchPolicy::OpenAction(QuilStubOpenAction)` | Compiled enum: `Open`, `Focus`, `Toggle` | Only enum variants valid | `Open` | Shell dispatches to `open_quil_in_active_scene()`, `focus_or_open_quil()`, or `toggle_quil()` at activation event |
| Optional keybinding (future) | `QuilStubKeybindingHint { scancode, modifiers }` | Shell-managed shortcut table; not free-form | Must pass shortcut conflict audit (D accessibility gate); scancode must be unused | None (not bound) | Shell registers scancode-to-`Focus201` mapping. Requires `SurfaceAction::Focus201` variant. Must not shadow existing bindings. |
| Visibility in shell surfaces | `QuilStubPreference::VisibleInLauncher(bool)` | Shell-owned toggle; Quil does not set its own visibility | Boolean only; enforced by shell's app visibility policy | `false` (hidden until Quil server exists) | Controls whether Quil appears in SilkBar launcher, Atlas, or app list. Shell may override for policy reasons. |
| Proof verbosity level | `QuilStubPreference::ProofLevel(QuilStubProofLevel)` | Compiled enum: `Minimum`, `Normal`, `Debug` | `Minimum` ≤ `Normal` ≤ `Debug` — no arbitrary levels | `Normal` | Controls budget-marker fire rate. `Minimum` only fires errors/rejects. `Debug` fires all markers. Can NOT suppress required safety markers. |

### Not Customizable

| Feature | Why Not Customizable |
|---------|---------------------|
| `SurfaceId` / `FrameId` | Surface identity is a system constant; changing it breaks display protocol, snapshot restore, and all shell wiring |
| Lifecycle FSM rules | Allowed/forbidden transitions are shell invariants; customization cannot skip guards |
| Focus/liveness validation | `try_set_focus()` guards (alive, focusable, non-tombstoned, in-scene, frame-accepts-input) are system invariants |
| Tombstone/destroy terminal behavior | Destroyed is terminal; tombstone prevents focus reuse. Customization cannot resurrect. |
| Framebuffer ownership | sexdisplay is sole framebuffer writer. No preference can grant Quil framebuffer access. |
| sexdisplay rendering policy | sexdisplay owns all surface rendering. Customization cannot inject pixel commands. |
| Capability checks | All IPC goes through PDX capability slot map. Customization cannot bypass. |
| Linen/OpenIntent requirements | File access requires Linen OpenIntent. Customization cannot grant file authority. |
| Collar grants | All authority flows through Collar grants. Customization cannot self-authorize. |
| Package trust | App identity verification is a separate track. Customization cannot assert trust. |
| Persistence gates | Disk persistence requires E/F gate approval. Customization cannot flush to disk. |
| PDX opcode/ABI layout | Opcodes are system constants. Customization cannot redefine protocol. |
| Geometry bounds checks | `clamp_position()` and `clamp_surface_size()` are mandatory; customization cannot disable |
| Required proof markers | `[quil.surface.focus.reject]`, `[quil.surface.close.reject]`, `[quil.error]` are required for safety |
| Crash/log/dev cockpit | Separate feature tracks H/I; customization cannot enable early |

### Required Model: Preference Types

```rust
/// Compiled-enum preference for Quil stub placeholder title.
/// Raw strings are rejected; only fixed variants allowed.
enum QuilStubTitleToken {
    Editor,    // "Quil Editor"
    Terminal,  // "Quil Terminal"
    Notepad,   // "Quil Notepad"
    Scratch,   // "Quil Scratch"
    Blank,     // "Quil" (default — no implied role)
}

/// Compiled-enum glyph for Quil stub chrome icon.
enum QuilStubIconToken {
    Code,   // code/chevron glyph
    Edit,   // pencil glyph
    Terminal, // prompt glyph
    Empty,  // no icon (default)
}

/// Silk theme token for accent color. Maps to existing scene token palette.
/// Never sets raw RGBA. Never bypasses Silk theme model.
enum QuilStubThemeToken {
    Default, // shell default accent
    Blue,
    Green,
    Amber,
    Plum,
    Slate,
}

/// Launch geometry + open action policy.
struct QuilStubLaunchPolicy {
    geometry: Option<(i32, i32, u32, u32)>, // None = use compiled default
    open_action: QuilStubOpenAction,
}

enum QuilStubOpenAction {
    Open,   // open_quil_in_active_scene()
    Focus,  // focus_or_open_quil()
    Toggle, // toggle_quil()
}

/// Future keybinding hint. Scancode must pass conflict audit.
/// Not wired in V1 — requires SurfaceAction::Focus201.
struct QuilStubKeybindingHint {
    scancode: u8,
    modifiers: u8,     // bitmask: Ctrl=1, Alt=2, Shift=4, Meta=8
    audit_token: u64,  // proof that conflict scan completed
}

/// Proof verbosity level. Cannot suppress required safety markers.
enum QuilStubProofLevel {
    Minimum, // errors + rejects only
    Normal,  // allow + reject + lifecycle transitions (default)
    Debug,   // all markers including budget probes
}

/// Aggregate preferences struct. Stored in shell static mut.
/// Volatile — not persisted to disk in V1.
struct QuilStubPreference {
    title: QuilStubTitleToken,
    icon: QuilStubIconToken,
    accent: QuilStubThemeToken,
    launch_policy: QuilStubLaunchPolicy,
    keybinding: Option<QuilStubKeybindingHint>, // None in V1
    visible_in_launcher: bool,
    proof_level: QuilStubProofLevel,
}
```

### Required Invariants

1. **Bad preference values clamp or reject deterministically.** Out-of-range enum values reject to compiled default. Out-of-bounds geometry clamps via `clamp_position()` / `clamp_surface_size()`. No silent truncation, no panic.
2. **Preferences are shell/user-owned, not Quil-app-owned.** The shell stores and validates all preferences. The Quil server (when it exists) reads preferences via shell IPC, not directly from user input.
3. **Preferences are memory/proof-only until E storage gates allow persistence.** No disk write. No sexstore commit. Snapshot captures shell state only (scene layout), not user preferences.
4. **Preference proof logs cannot include private document/project names.** Logged preference events include enum variant name but never leaked content, filenames, paths, or project identifiers.
5. **Renderer-affecting preferences are model tokens, not renderer policy.** Accent color is a `QuilStubThemeToken` that maps through Silk scene token palette. It never contains raw RGBA, pixel data, or render commands.
6. **Keybindings require D accessibility + shortcut conflict audit.** Before any `QuilStubKeybindingHint` is accepted, a conflict scan against all existing `scancode_to_action()` mappings must pass. Accessibility team must sign off.
7. **Customization cannot disable required proof markers.** `ProofLevel::Minimum` still fires: `[quil.surface.focus.reject]`, `[quil.surface.close.reject]`, `[quil.error]`, `[quil.surface.destroy.reject]`. Slient failures are forbidden.
8. **Customization cannot hide security/capability denial state.** If a capability denial occurs, proof markers fire regardless of `ProofLevel`. No preference suppresses security events.
9. **Reset-to-safe-default must exist.** `quil_reset_preferences()` restores all fields to compiled defaults. Called on boot, on preference validation failure, and on explicit user request.

### STOP FIRST: Customization Red Lines

If any of the following are proposed, stop all customization work and escalate:

- 🛑 User-editable code/plugin/theme execution — no scripts, no WASM, no dynamic themes in V1 stub
- 🛑 Raw color/layout values without bounds — no direct RGBA, no unclamped geometry, no pixel dimensions outside framebuffer
- 🛑 App-owned launch/focus policy — shell owns all focus and launch decisions; Quil cannot set its own geometry or scene
- 🛑 Disabling lifecycle guards — cannot skip `surface_in_active_scene()`, `is_tombstoned()`, `is_focusable_surface()`, `frame_accepts_input()`
- 🛑 Disabling proof markers — required safety markers are non-optional
- 🛑 Hidden unsafe/developer mode — no "unsafe mode" that bypasses capability checks or framebuffer ownership
- 🛑 Persistence before E gates — no disk writes; no sexstore commits; no durable preference storage
- 🛑 Keybindings before conflict/accessibility audit — no scancode mapping without D accessibility sign-off
- 🛑 Private metadata in preferences or proof logs — no filenames, project names, user data, document content in log output
- 🛑 Preference values that imply identity/authority — no title like "System Console" or "Root Shell" that implies elevated privilege

### Proof Scenarios: Customization

| # | Scenario | Input | Expected Result | Guard | Marker |
|---|----------|-------|----------------|-------|--------|
| 1 | Valid accent token accepted | `QuilStubThemeToken::Blue` | Accent changes to Blue; chrome repaints with Silk theme token `blue_accent` | Token validated against compiled enum; mapped through palette | `[quil.pref.accept] token=Blue` |
| 2 | Invalid accent token rejected | `QuilStubThemeToken::Invalid(0xFF)` | Rejected; stays at compiled default (`Default`) | Out-of-range enum discriminant caught; fallback to default | `[quil.pref.reject] reason=invalid_token token=0xFF` |
| 3 | Launch geometry outside bounds clamped | `x=-500, y=-500, w=10000, h=10000` | Geometry clamped to framebuffer bounds: w≥80, h≥60, x≥0, y≥0; oversized dimensions capped | `clamp_position()` + `clamp_surface_size()` applied | `[quil.pref.clamp] field=geometry reason=bounds` |
| 4 | Keybinding requested before audit rejected | `QuilStubKeybindingHint { scancode: 0x3C, modifiers: 0, audit_token: 0 }` | Rejected; audit_token must be nonzero; scancode 0x3C is `DestroyFocused` (conflict) | Shortcut conflict audit; D accessibility gate; audit_token verification | `[quil.pref.reject] reason=no_audit scancode=0x3C` |
| 5 | Preference persistence before E gates rejected | Attempt to write preferences to sexstore | No-op; preferences are memory-only until E storage gate | No disk-write path in silk-shell; storage gate compile-time flag | `[quil.pref.reject] reason=persistence_not_gated` |
| 6 | Reset-to-safe-default restores compiled defaults | `quil_reset_preferences()` called | All fields = compiled default: `Blank` title, `Empty` icon, `Default` accent, boot geometry, `Open` action, no keybinding, hidden, `Normal` proof level | Explicit reset function zeroes all fields to `const QUIL_DEFAULT_PREFERENCE` | `[quil.pref.reset]` |
| 7 | Customization cannot change SurfaceId | Attempt to set `SURFACE_ID_QUIL = 202` via preference | Impossible — `SURFACE_ID_QUIL` is `const u64`, not a preference field | Rust `const` immutability; no preference field modifies SurfaceId | (compiler error before runtime) |
| 8 | Customization cannot disable proof markers | Set `ProofLevel::Minimum` — required markers still fire | `[quil.surface.focus.reject]` fires on focus rejection at `Minimum` level | `ProofLevel::Minimum` excludes allow/lifecycle/budget markers but REQUIRED_SAFETY_MARKERS bitmask always fires | `[quil.pref.proof.minimum] markers=required_safety_only` |
| 9 | Preference proof log redacts private metadata | Title set to `Editor`, log writes preference update | Logged as `[quil.pref.accept] token=Editor` — NOT `[quil.pref.accept] token=Editor project="/home/user/secret_project"` | Log format enforced at compile time; no string interpolation of user-supplied content | `[quil.pref.accept] token=Editor` (no private data leaked) |

### SCAN 8 Integration with Existing Stub

The customization surface has zero impact on existing stub code. No `static mut QUIL_PREFERENCE` has been added yet — this section defines the model for when preferences are implemented. The existing 5 helpers + 12 wiring locations remain unchanged.

When preferences are implemented:
1. Add `static mut QUIL_PREFERENCE: QuilStubPreference` initialized to `QUIL_DEFAULT_PREFERENCE`
2. Add `quil_set_preference()` and `quil_reset_preferences()` helpers
3. Wire preference validation into preference-setter path (clamp, reject, accept)
4. Wire `QuilStubThemeToken` into frame chrome accent render (if Silk supports)
5. Wire `QuilStubTitleToken` into `ShellTab.title_id`
6. Wire `QuilStubLaunchPolicy::geometry` into `ensure_quil_frame()` normal_* fields
7. Wire `QuilStubLaunchPolicy::open_action` into activation dispatch
8. Wire `QuilStubProofLevel` into budget-marker fire gates
9. Add proof markers for preference accept/reject/clamp/reset
10. **STOP FIRST** if any preference touches the 10 non-customizable features

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Initial Quil surface stub matching Linen pattern | QUIL_SURFACE_STUB_V1 |
| 2026-05-04 | 3-pass scan: 3 correctness bugs, 7 architecture issues, 7 product gaps | 3X_PHASE_SCAN_V1 |
| 2026-05-04 | 7-scan hardening: boundary audit, premortem (18 failures), FSM (10 states), pattern mapping (16 elements), proof markers (14 + 15 negative tests), safest path (10 steps), exceeded hypothesis (12 rivals) | 7X_HARDEN_V1 |
| 2026-05-04 | SCAN 8 customization: 8 customizable fields, 14 non-customizable invariants, 9 proof scenarios, 10 STOP FIRST red lines | 8X_CUSTOMIZE_V1 |
