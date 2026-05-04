# PHASE 02: Shell Surface Ownership + Scene/Frame/Tab Core

## Goal
Create the real Silk shell. Silk owns the workspace graph, frame tree, tabs, focus, tiling, hover, and mode switching. Sexdisplay renders only what shell tells it — no policy in the display server.

## Ownership
- **silk-shell** (exclusive): all shell state, policy, FSM, hit-test dispatch
- **sexdisplay** (read-only consumer): renders surface chrome per shell's commands

## What Already Exists (~70% done)
- **SurfaceAction enum**: Move/Focus/Destroy/Recreate/Snap/Resize actions all defined
- **ShellInteractionState**: Idle/ClickPending/Dragging/PanelActive FSM
- **Frame model**: `ShellFrame` with tabs array, flags (MINIMIZED, ZOOMED, TOP_BAR), normal geometry
- **Tab stack**: `frame_tab_count()`, `frame_active_tab_index()`, `switch_to_tab()`, multi-tab boot init
- **Frame Lights**: CLOSE/MINIMIZE/ZOOM with hit-target detection, action dispatch, markers
- **Chrome hit-testing**: `hit_test_surface_chrome()` with rim/lights/tab strip dispatch
- **Top bar**: `FRAME_FLAG_TOP_BAR`, toggle via F4, rendering in sexdisplay via chrome_flags
- **Appearance tokens**: `OP_APPEARANCE_TOKENS` (0xFC) with 4 presets, F5 cycling, in-memory state
- **Focus management**: `try_set_focus()`, `clear_focus_if_dead()`, `FOCUSED_SURFACE_ID`
- **Rim drag**: Drag FSM with start/move/end markers
- **SceneAppearanceState**: preset_idx + custom_colors + flags in silk-shell

## What's Missing (~30% — focus here)
- **Scene model**: No virtual desktop / workspace switching (SilkBar has WORKSPACE_COUNT=5 but no shell-side scene awareness)
- **Surface ID tombstones**: No formal dead surface tracking (surface_is_alive() exists but no tombstone slot rotation)
- **Tiling engine**: SnapLeft/Right/Maximize/Center exist as manual actions but no automatic tiling
- **Minimize/restore UI**: Works via keyboard (PageUp) but no click-to-minimize via Frame Light (the light dispatch exists but minimize action may not be wired)
- **Hover reveal**: frame_lights_mode / tab_strip_mode for hover-reveal in minimal mode (model exists in token flags but no behavior)

## Bundle

| Task | Detail | Effort | Priority |
|------|--------|--------|----------|
| Scene model (virtual desktop) | Silk-shell `Scene` struct, active scene tracking, workspace list | 4h | High (unlocks SilkBar workspace switching) |
| Surface ID tombstones | Dead surface detection, slot reuse safety, tombstone timeout | 2h | High (prevents surface ID collision bugs) |
| Tiling engine | Auto-tile on snap, resize-adjacent, half/third layout options | 6h | Medium (SnapLeft/Right exist but manual) |
| Hover-reveal chrome | When minimal_rim_mode and pointer in chrome zone, show lights/tabs | 3h | Low (polish, not core) |
| Frame Light minimize wiring | Verify MINIMIZE light dispatch calls minimize_frame() | 1h | High (bug fix if missing) |

## Smallest First Step
Add the Scene model: a `Scene` struct with an array of workspace slots. One static instance. Active workspace index tracked. This is trivially safe (static, fixed-size) and immediately enables SilkBar workspace switching to actually work.

## Dependencies
- **Blocking**: None (independent of kernel, display, input)
- **Blocked by**: Nothing
- **Can parallelize with**: Phase 0 (gates), Phase 1 (display contract), Phase 3A (input policy) — but Phase 3A's click dispatch depends on the chrome hit-test model already built, so Phase 2 must be done BEFORE Phase 3A completes. However, Phase 2 is already 70% done.

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Scene model causes dispatch confusion (which scene is active?) | Low | Medium | Single static `ACTIVE_SCENE_IDX: u8`. Focus always belongs to a scene. Linear scan. |
| Tiling engine adds dynamic state | Medium | High | Static fixed-size layout table (max 4 tiles per scene). No Vec, no heap. Reject if more tiles requested. |
| Hover reveal interacts badly with drag FSM | Low | Medium | Hover is purely visual (sexdisplay token change). No FSM interaction. Shell hit targets unchanged. |

## Exit Criteria (Done Checklist)
- [ ] `Scene` struct with workspace slots (at least 2, max 8)
- [ ] Active scene switching via keyboard shortcut or SilkBar click
- [ ] Surface ID tombstones prevent reuse of recently-dead IDs
- [ ] SnapLeft/Right produces a tiled pair (two surfaces side by side)
- [ ] Frame Light MINIMIZE click actually minimizes the frame
- [ ] All existing chrome/tab/light/focus/drag markers still fire
- [ ] Default build + boot passes. No new warnings.

## Testing Strategy
- **Scene**: Switch workspace, verify focus moves to correct scene's top surface
- **Tombstones**: Destroy surface, create new surface, verify it gets a fresh ID
- **Tiling**: SnapLeft, then SnapRight, verify both surfaces visible and adjacent
- **Regression**: All `[shell.frame.*]` and `[shell.drag.*]` markers fire at expected counts

## Efficiency Opportunity
**The Scene model and appearance tokens should converge.** Scenes should carry their own appearance token override (different scenes can have different color schemes). Phase 2 should wire the `SceneAppearanceState` into the `Scene` struct so that switching scenes also switches colors. This is a 2-hour addition that makes scenes feel 10x more complete.

## Completeness Gain
Shell/window/session: **70% existing + 25% new → 95%** (revised upward because most work already exists)

## Files Changed
- `servers/silk-shell/src/main.rs` (Scene struct, tombstones, tiling, minimize wiring)
- `servers/silk-shell/src/main.rs` (scene→appearance token wiring)

## Forbidden
- Kernel edits
- Renderer rewrites
- Dynamic allocation (static arrays only)
- Animation system
- True alpha/blur

## Next Phase
PHASE_03_INPUT_COMPLETION_USB_MOUSE.md

## Parallel Note
Phase 2 is mostly additive to existing shell code and doesn't block on Phase 0 or Phase 1. These three phases can proceed in parallel with different owners.
