# FRAME_LIGHTS_POLICY_SPEC_V1

## Verdict: PASS REVIEW ONLY — Docs-only policy spec.

## Frame Lights
| Light | Color | Action | V1 Status |
|-------|-------|--------|-----------|
| Red | 0x00FF4444 | Close active tab | disabled (close_allowed=0) |
| Yellow | 0x00FFCC44 | Minimize to card | keyboard-only |
| Green | 0x0044FF44 | Zoom/unzoom | keyboard-only |

## Safety Rules
- Red must not close core apps (close_allowed=0)
- Red enabled ONLY for disposable/safe tab proof
- Frame close requires lifecycle proof (future)
- No destructive surprises

## Ownership
- silk-shell: action policy
- sexdisplay: bounded pixel render only (future)
- Spindle: help/status only
- Linen: objects not shell policy

## States
visible | disabled | armed | unavailable | pending_future

## Implementation Ladder
| Phase | Deliverable |
|-------|-------------|
| 0 | Docs policy (this) |
| 1 | Status/help stub markers |
| 2 | Visual noninteractive lights |
| 3 | Keyboard yellow/green |
| 4 | Red for disposable proof only |
| 5 | Pointer hover/click (deferred) |

## STOP FIRST Boundaries
- No close of core apps
- No pointer/hover until stability proven
- No sexdisplay policy ownership
- No kernel/pdx/ABI changes
