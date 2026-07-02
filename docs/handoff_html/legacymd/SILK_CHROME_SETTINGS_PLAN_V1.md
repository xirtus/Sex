# SILK_CHROME_SETTINGS_PLAN_V1

## Status

Design (2026-05-04). Canon update for configurable Frame Chrome and future Scene/Theme Settings app. No code changed.

---

## New Canon: Frame Chrome Visual Modes

Silk Frame Chrome must support two visual modes, both owned by the shell/display pipeline (not by apps). No app draws its own chrome, titlebar, or toolbar.

### 1. Default Mode — Top Bar ON

The default visual mode for frames features a collapsible thin top bar inspired by Apple OS X / liquid-glass aesthetics:

- **Three colored lights** at top-left (existing Frame Lights):
  - Red = close (existing)
  - Yellow = minimize/collapse (existing)
  - Green = zoom/maximize (existing)
- **Tab strip** adjacent to lights, showing active tabs as colored blocks or labels (future)
- **Neon rim** around the entire frame, thin (4px default)
- The top bar integrates the existing Frame Lights, tab strip, and top rim into a unified chrome band
- Can collapse to minimal mode via user setting or per-frame toggle

**Relationship to existing code:**
| Existing Element | Maps To |
|-----------------|---------|
| Frame Lights | CLOSE / MINIMIZE / ZOOM (implemented) |
| Neon rim | FRAME_RIM_PX / FRAME_RIM_COLOR (implemented) |
| Tab strip | FRAME_TAB_STRIP_PX (model done, rendering needs IPC) |
| Top bar | Future: taller chrome band above content area |
| Lights → actions | close/minimize/zoom (all implemented) |

### 2. Minimal Mode — Top Bar OFF

When the top bar is collapsed:

- **No persistent top bar** — only the thin 4px neon rim remains visible
- **Hover-revealed tab identity** — hovering over the rim shows which tab is active (requires text pipeline)
- **Frame Lights appear on hover/chrome reveal** — close/minimize/zoom lights only visible when pointer enters the chrome zone
- **Tab strip hidden** until hover or chrome reveal

**Current state:** This is very close to what exists NOW (4px rim + persistent lights). The difference is that in full minimal mode, lights would be hover-revealed rather than always-on.

---

## Settings Architecture (Future)

### Scene/Theme Settings App

A future settings application with the following responsibilities:

| Setting | Scope | Default | Configurable |
|---------|-------|---------|-------------|
| Top bar on/off | Per-Scene, Per-monitor | ON | User toggle |
| Chrome density | Per-Scene | normal (4px rim) | compact, normal, spacious |
| Rim thickness | Per-Scene | 4px | 1-8px |
| Rim color | Per-Scene | `0x00C0F0FF` (neon cyan) | User pick |
| Light style | Per-Scene | colored circles | circles, squares, outline, monochrome |
| Tab strip mode | Per-Scene | blocks | blocks, labels (future), hidden |
| Hover reveal behavior | Per-Scene | enabled | on, off, delayed |
| Glass intensity | Per-Scene | medium | none, light, medium, heavy |
| Wallpaper/background | Per-monitor | default gradient | solid color, gradient, image (future) |
| Monitor scale/layout | Per-monitor | 1.0 | 1.0, 1.25, 1.5, 2.0 |
| Scene theme override | Per-Scene | inherit global | custom palette |

### Scope definitions

- **Per-monitor:** Settings apply to a specific physical display output
- **Per-Scene:** Settings apply to a virtual desktop / Scene workspace
- **Global:** Settings apply to all monitors and Scenes unless overridden

---

## Implementation Roadmap

### Short-term (next phases, no new settings)

1. **FRAME_TAB_STRIP_IPC_PLAN_V1** ← next recommended
   - Design IPC for tab metadata to sexdisplay
2. **FRAME_TAB_STRIP_RENDER_V1**
   - Render colored tab blocks in sexdisplay composite_pixel()

### Medium-term (model-only, settings plumbing)

3. **FRAME_TOP_BAR_MODEL_V1**
   - Define top bar geometry (height, content zones)
   - Define chrome mode constants (DEFAULT / MINIMAL)
   - Model how top bar interacts with existing rim/lights/tab strip
   - No renderer changes yet

4. **FRAME_TOP_BAR_RENDER_PLAN_V1**
   - Design rendering approach for top bar in sexdisplay
   - Determine if top bar replaces rim or extends it
   - Specify pixel layout for lights + tabs + optional frame title

5. **FRAME_TOP_BAR_RENDER_V1**
   - Implement top bar rendering in sexdisplay composite_pixel()
   - Top bar = taller chrome band (maybe 12-20px) replacing the top 4px rim
   - Lights and tab strip move into the top bar
   - Minimal mode = 4px rim (current behavior)

6. **FRAME_CHROME_MODE_SETTINGS_V1**
   - Add chrome mode toggle (default/minimal) to shell state
   - IPC to communicate mode to sexdisplay
   - Per-frame chrome mode setting
   - No UI yet — keyboard shortcut or config file

### Long-term (settings application)

7. **SCENE_THEME_SETTINGS_MODEL_V1**
   - Define theme token model (rim color, light style, glass intensity, etc.)
   - Per-Scene and per-monitor override model
   - No renderer changes yet

8. **SCENE_THEME_SETTINGS_APP_PLAN_V1**
   - Design settings application (new PDX server or shell extension)
   - Settings IPC protocol
   - Storage model (no filesystem — in-memory, baked into boot config)
   - UI wireframe for settings panels

9. **SCENE_THEME_SETTINGS_APP_V1**
   - Implement settings application
   - Wire chrome mode toggle to sexdisplay
   - Wire rim color/thickness to sexdisplay
   - Wire light style to sexdisplay

---

## Design Constraints (Preserved)

| Constraint | How maintained |
|------------|---------------|
| No app draws chrome | Shell and display only |
| sexdisplay sole framebuffer writer | All rendering goes through sexdisplay |
| No dynamic allocation | Settings stored in fixed-size structs, not heap |
| No kernel/ABI changes | All settings IPC goes through existing PDX opcode space |
| Framebuffer bounds preserved | All chrome rendering goes through clamp_surface() |
| Lights always actionable | Click priority: lights > tab strip > rim drag |
| Close/minimize/zoom always work | In both default and minimal mode |
| Tab strip model already built | ShellFrame.tabs[], frame_tab_at() helpers exist |

---

## Forbidden

- Code changes in this phase
- Renderer changes
- ABI/protocol changes
- Settings app implementation
- Top bar rendering
- Any chrome mode behavior change

---

## Files

| File | Role |
|------|------|
| `docs/handoff/SILK_CHROME_SETTINGS_PLAN_V1.md` | This document |
| `.claude/plans/splendid-brewing-starlight.md` | Updated with roadmap phases |

## Next Phase

### FRAME_TAB_STRIP_IPC_PLAN_V1 (unchanged)

Continue with the existing next phase: design IPC protocol for communicating tab metadata (tab_count, active_tab) from silk-shell to sexdisplay. The chrome settings plan does not change the immediate next step.
