# ATLAS_SCENE_SWITCH_KEYBOARD_V1

Status: PASS
Date: 2026-05-14

## Scope
- Added default-off Atlas keyboard proof gate in `silk-shell`.
- Reused existing Atlas overlay/nav/apply logic.
- Added marker aliases required by mission without broad model changes.
- Added bounded accent-prev keyboard path (`Z`) complementary to existing accent-next (`A`).

## Files Changed
- servers/silk-shell/src/main.rs
- docs/handoff/ATLAS_SCENE_SWITCH_KEYBOARD_V1.md

## Changes
1. Proof gate
- Added `SEXOS_ATLAS_SCENE_KEYBOARD_PROOF` gate.
- Added `maybe_run_atlas_scene_keyboard_proof()` and wired it into the existing proof-run chain.
- Proof stages:
  - open/focus Atlas overlay
  - next scene / previous scene
  - next accent / previous accent
  - apply/commit
  - close/back

2. Marker aliases
- Added:
  - `[atlas.key.recv] code=N down=N mod=N`
  - `[atlas.scene.nav] old=N new=N count=N`
  - `[atlas.accent.nav] old=N new=N count=N`
  - `[atlas.scene.apply] scene=N accent=N ok=N reason=...`
  - `[atlas.overlay.toggle] enabled=N ok=N reason=...`
  - `[atlas.scene.keyboard.proof] stage=N action=NAME ok=N reason=...`
  - `[atlas.scene.keyboard.proof.done] ok=N`

3. Keyboard behavior additions
- Existing scene navigation kept as-is (arrow-key nav in Atlas mode).
- Existing accent-next (`A`, scancode `0x1E`) kept; added accent-prev:
  - `Z` (`0x2C`) decrements selected scene accent token with wrap.
- Existing apply/commit via `Enter` kept.
- Existing close/back via `Esc` kept.

## Build
- `SEXOS_ATLAS_SCENE_KEYBOARD_PROOF=1 ./scripts/entrypoint_build.sh` -> PASS
- `./scripts/entrypoint_build.sh` -> PASS

## Runtime (Headless)
Log:
- `/tmp/sexos_atlas_scene_switch_keyboard_v1.log`

Observed key markers:
- `[atlas.overlay.toggle] enabled=1 ok=1 reason=opened`
- `[atlas.scene.keyboard.proof] stage=0 action=open_focus ok=1 reason=ok`
- `[atlas.key.recv] code=77 down=1 mod=0`
- `[atlas.scene.nav] old=0 new=1 count=5`
- `[atlas.key.recv] code=75 down=1 mod=0`
- `[atlas.scene.nav] old=1 new=0 count=5`
- `[atlas.accent.nav] old=0 new=1 count=5`
- `[atlas.accent.nav] old=1 new=0 count=5`
- `[atlas.scene.apply] scene=0 accent=0 ok=1 reason=ok`
- `[atlas.overlay.toggle] enabled=0 ok=1 reason=cancel_close`
- `[atlas.scene.keyboard.proof.done] ok=1`

Counts:
- `atlas.key.recv`: 6
- `atlas.scene.nav`: 2
- `atlas.accent.nav`: 2
- `atlas.scene.apply`: 1
- `atlas.overlay.toggle`: 4
- `atlas.scene.keyboard.proof`: 8
- `atlas.scene.keyboard.proof.done`: 1
- faults (`fault.kill|#PF|#GP|panic|KERNEL PANIC`): 0

## Outcome
- Atlas open/focus/toggle proven.
- Scene navigation proven.
- Accent next/prev proven.
- Apply/commit proven.
- Close/back proven.
- No kernel/ABI/USB/pointer/Quil changes.
