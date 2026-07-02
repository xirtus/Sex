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

<!-- SCAN 1 applied: wording fixes reflected in Non-Goals below -->

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

Quil stub maps 1:1 to the canonical Linen/app-surface pattern. Verified elements: SurfaceId constant (201), FrameId (3), geometry statics, 5 helpers (ensure/open/focus_or_open/toggle/frame_id), shell tables (tile, emit_snapshot, get_surface_bounds, point_in_surface, update_local_geometry), focus/close/minimize/zoom guards, z-order arrays (focus + hit-test), snapshot capture/restore, scene sync, budget markers, non-goals enforcement — all structural copies of Linen with zero special-case shell code. Risk if missing: surface ID collision, stale geometry, focus targeting dead surfaces, or scope creep.

**STOP FIRST**: If the pattern cannot support Quil without special cases, halt and redesign. Currently zero special cases exist. Pre-commit diff enforces <5% divergence from Linen helpers.

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

### Negative Tests (condensed — reject cases named)

All 15 negative tests guard the same pattern: every dangerous operation is blocked by a compile-time or guard-time invariant. Reject cases:

- **Focus before frame** → `[quil.surface.focus.reject]` reason=no_frame (Patch B: `surface_in_active_scene(201)`=false)
- **Focus tombstoned** → `[quil.surface.focus.reject]` reason=tombstoned
- **Toggle after destroy** → no frame found; `ensure_quil_frame()` creates new if slot available (no resurrect)
- **Double-close** → `[quil.surface.close.reject]` reason=already_dead
- **Pointer outside bounds** → `point_in_surface(201, x, y)`=false
- **Stale geometry** → `tile_visible_frames()` overwrites with current frame position
- **SurfaceId/FrameId collision** → compiler error on duplicate constants
- **z-order missing** → Patch A adds Quil; `hit_test_at()` fallback works
- **Invalid snapshot bounds** → `clamp_position()`/`clamp_surface_size()` corrects before `[quil.surface.snapshot.emit]`
- **Forbidden paths** (file open, disk write, framebuffer, ABI edits) → no code path exists in silk-shell; compile-time absent

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

Six months from now, a rival ships a better Quil-like surface stub. Below is how they beat us and how SexOS wins.

### Exceeded Hypothesis (condensed)

| Category | Rival Advantage | SexOS-Native Fix | Invariant/Proof Gate |
|----------|----------------|------------------|---------------------|
| Stub size | Smaller stub (no FSM/handoff) | 5 helpers, 12 wiring, <1% of silk-shell. FSM is documentation, not code. | No Quil-specific code beyond Linen-copied helpers |
| Editor speed | Launches real editor faster | Stub deliberately bounded — editor is separate track (PDX server, handoff, Collar) | Shell has zero editor logic |
| Upgrade path | Clear V2 path | V2 boundaries documented; handoff pattern reusable for any app surface | V2 plan in /rapid (PHASE_05) |
| Visual clarity | Placeholder has pixels | Non-Goal: no placeholder in V1. Future: colored rect via sexdisplay only. | No framebuffer access in silk-shell |
| Pattern adherence | No focus-fallback bug, stale geometry, special cases, proof gaps, tombstone confusion, pattern drift, lifecycle gaps, handoff staleness | All merge to one invariant: **Quil follows Linen pattern exactly.** Focus guard (Patch B), 3-path geometry update, zero special cases, allow+reject markers for every guard, OS-surface tombstone skip, FSM complete, handoff updated in same PR. | Diff check ≤5% from Linen; `try_set_focus` returns false before frame; every guard has allow+reject marker; handoff reviewed with code changes |

### SexOS-Native Design Principles

- **Tiny reliable placeholder first** — Surface identity only. 12 wiring locations. 5 helpers. No server/editor/files.
- **App-surface pattern reuse** — Every new surface is a structural copy of Linen (<5% diff enforced).
- **Proof-first operations** — Every operation has allow+reject markers. Every guard has a negative test.
- **Desktop vs SexOS**: Editor opens files → Linen OpenIntent mediates; Editor owns chrome → Shell owns via Silk Scene; Editor renders window → sexdisplay renders; Editor is monolith → Quil is one surface ID + independent PDX binary.

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

<!-- Accepted risks: no keyboard binding (requires Focus201 + audit), no visual placeholder (no server), FRAMES 3/4, helper dup ~140 lines, SurfaceId registry is comment (collision scan), Quil bypasses tombstone intentionally -->

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

**Rule:** Quil surface stub may define user-facing preferences, only as shell-owned validated policy. Customization must not turn the stub into the real Quil editor, dev cockpit, file browser, agent runner, storage client, renderer policy owner, or security bypass. Preferences are shell-memory-only until E storage gates.

### Customizable (8 fields)

| Preference | Type | Bounds | Default |
|------------|------|--------|---------|
| Placeholder title | Compiled enum: `Editor`, `Terminal`, `Notepad`, `Scratch`, `Blank` | Only enum variants; raw strings rejected | `Blank` |
| Icon/glyph | Compiled enum: `Code`, `Edit`, `Terminal`, `Empty` | Only enum variants | `Empty` |
| Accent color | Silk theme token: `Default`, `Blue`, `Green`, `Amber`, `Plum`, `Slate` | Mapped through palette; raw RGBA rejected | `Default` |
| Launch geometry | `{x, y, w, h}` via shell API | Clamped by `clamp_position/size`; w≥80, h≥60 | (100, 100, 640, 480) |
| Open behavior | Compiled enum: `Open`, `Focus`, `Toggle` | Only enum variants | `Open` |
| Keybinding (future) | `{scancode, modifiers}` | Must pass D accessibility + conflict audit | None |
| Launcher visibility | `bool` | Boolean only | `false` |
| Proof verbosity | Compiled enum: `Minimum`, `Normal`, `Debug` | Minimum ≤ Normal ≤ Debug; required safety markers never suppressed | `Normal` |

### NOT Customizable (15 system invariants)

`SurfaceId`/`FrameId`, lifecycle FSM rules, focus/liveness validation (5-guard `try_set_focus`), tombstone/destroy terminality, framebuffer ownership, sexdisplay rendering policy, PDX capability checks, Linen OpenIntent requirements, Collar grants, package trust, persistence gates (E/F), PDX opcode/ABI layout, geometry bounds clamping, required proof markers (`[quil.surface.focus.reject]`, `[quil.surface.close.reject]`, `[quil.error]`), crash/log/dev cockpit scope.

### Invariants (9)

1. Bad values clamp or reject deterministically — no silent truncation, no panic.
2. Preferences are shell/user-owned, not Quil-app-owned.
3. Memory/proof-only until E gates — no disk write, no sexstore commit.
4. Proof logs never include private document/project names — enum variants only.
5. Renderer-affecting preferences are model tokens, not pixel commands.
6. Keybindings require D accessibility + shortcut conflict audit.
7. Customization cannot disable required proof markers (`Minimum` still fires safety markers).
8. Customization cannot hide security/capability denial state.
9. Reset-to-safe-default must exist (restores compiled defaults on boot, validation failure, or user request).

### STOP FIRST (10 red lines)

User-editable code/plugins/WASM; raw RGBA or unclamped geometry; app-owned launch/focus policy; disabling lifecycle guards; disabling proof markers; hidden unsafe/developer mode; persistence before E gates; keybindings without conflict/accessibility audit; private metadata in pref logs; preference values implying identity/authority (e.g., "System Console").

### Proof Scenarios (condensed)

Valid token accepted → `[quil.pref.accept]`; invalid token rejected → `[quil.pref.reject]`; geometry out-of-bounds clamped → `[quil.pref.clamp]`; keybinding before audit → `[quil.pref.reject] reason=no_audit`; persistence before E gates → `[quil.pref.reject] reason=persistence_not_gated`; reset-to-default → `[quil.pref.reset]`; cannot change SurfaceId (compiler error); cannot suppress required safety markers at `Minimum` → `[quil.pref.proof.minimum]`; private metadata redacted from pref logs.

When implementing: add `static mut QUIL_PREFERENCE` + `quil_set/reset_preferences()` helpers, wire token enums into chrome/title/geometry/proof-level paths, add proof markers for accept/reject/clamp/reset. **STOP FIRST** if any preference touches the 15 non-customizable features.

