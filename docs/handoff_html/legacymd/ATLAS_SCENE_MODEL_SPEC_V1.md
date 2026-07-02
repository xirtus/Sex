# ATLAS_SCENE_MODEL_SPEC_V1

## Verdict: PASS REVIEW ONLY — Docs-only spec.

## Core Terms
| Term | Definition |
|------|-----------|
| Scene | Workspace/layout unit containing 1-N Frames |
| Atlas | Overview/collage of all Scenes. Keyboard-navigable. |
| Frame | Window container in a Scene. Has Chrome, Lights, Rim. |
| Tab | Document/app surface inside a Frame |
| Surface | Rendered app content area |
| Minimized Card | Collapsed Frame thumbnail in taskbar |
| Scene Thumbnail | Static preview of a Scene's layout |
| Scene Focus | Which Scene current input targets |

## Model
**Scene**: id, label, active, frame_count, minimized_count, layout_kind, safe_preview
**Atlas**: mode, scene_count, active_scene, thumbnails, selected_scene

## Ownership
| Component | Owner | Role |
|-----------|-------|------|
| Scene/Atlas policy | silk-shell | Layout, focus, switching |
| Thumbnail rendering | sexdisplay | Bounded pixels (future) |
| Active scene status | SilkBar | Display only |
| Project/session objects | Linen | Not Scene policy |
| Cap graph visualization | Mesh | Future |

## Phase Ladder
| Phase | Deliverable |
|-------|-------------|
| 0 | Docs spec (this) |
| 1 | Atlas status stub/markers |
| 2 | Scene lifecycle markers |
| 3 | Keyboard Scene switch proof |
| 4 | Noninteractive Atlas preview |
| 5 | Pointer drag only after stability |
| 6 | Live collage (future) |

## Forbidden
- Live thumbnails, pointer drag, surface protocol redesign
- Renderer-owned Scene policy, shared framebuffer
- Alpha/blur/shadow/full-frame effects

## App Relationships
- Quil/Linen: launchable surfaces (launch_exec=1)
- WebStub: placeholder_requested, no surface
- Atlas: overlay/nonfocusable (current truth)
