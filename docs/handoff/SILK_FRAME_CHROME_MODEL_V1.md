# SILK_FRAME_CHROME_MODEL_V1

## Result: PASS IMPLEMENTED — 78/78 gates

## Model Table
| Component | Count | Type |
|-----------|-------|------|
| Scenes | 1 | Workspace (static derived) |
| Frames | 3 | Spindle(0), Quil(1), Linen(2) |
| Tabs | 3 | 1 per frame |
| Surfaces | 4 | Spindle, Quil, Linen, WebStub |

All derived/static from current known surfaces. No runtime state changes.

## Chrome State Vocabulary
hidden | rim_only | tab_visible | tab_strip | minimized_card | zoomed

Current: tab_visible on all 3 frames.

## Commands Added
- `frame-chrome`: model overview + chrome states
- `scene-status`: Workspace scene summary

## What Is NOT Implemented
- Visual rim rendering (future Phase 3)
- Hover tab (pointer dependency, future Phase 6)
- Frame Lights actions (future Phase 5)
- Close (close_allowed=0)
- Atlas scene view (future Phase 7)

## Safety
3 files, +60 lines. No kernel/pdx/ABI changes. Marker-only model.
