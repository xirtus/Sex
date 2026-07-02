# SilkBar Keyboard Status Integration V1

## Status: PASS (with documented ABI blockers)
Date: 2026-05-14
Attempts: 1

## SilkBar Status Path Status

### Working (keyboard-driven status updates):

| Trigger | SilkBar Update | Opcode | Marker |
|---------|---------------|--------|--------|
| Focus change (try_set_focus) | Focus state + window options | OP_SILKBAR_FOCUS_STATE (0xF9) | [shell.silkbar.status.send] |
| Scene switch (switch_scene) | Workspace active index | OP_SILKBAR_WORKSPACE_ACTIVE (0xF8) | [shell.silkbar.status.send] |
| SilkBar receives workspace | Workspace chips updated | — | [silkbar.workspace.recv] |
| SilkBar receives focus | Selected options updated | — | [silkbar.selected.options.recv] |
| Bell count (silkbar polls Bell) | Bell presence dot+count | OP_BELL_LIST → SetBellPresence | [silkbar.bell.poll.reply] |

### ABI Gaps (STOP FIRST — documented blocker):

| Feature | Blocker | Impact |
|---------|---------|--------|
| Active app name in SilkBar | No UpdateKind variant in silkbar-model | SilkBar can't display focused app name |
| Tint/accent in SilkBar | No UpdateKind variant in silkbar-model | SilkBar can't display active tint |

Adding these requires:
1. New `SetActiveAppName` and `SetTint` variants in `silkbar-model::UpdateKind` (ABI change)
2. New chip slot or display slot in silkbar-model (ABI change)
3. Extended `OP_SILKBAR_FOCUS_STATE` args or new opcode (protocol change)
4. sexdisplay renderer update for new chip kind (display protocol change)

All are STOP FIRST per mission rules.

## Proof Table

| Stage | Action | ok | SilkBar Status Sent |
|-------|--------|----|---------------------|
| 0 | Focus Spindle | 1 | focus=153 app=Spindle tint=0 |
| 1 | Focus Bell | 1 | focus=204 app=Bell tint=0 |
| 2 | Focus Mesh | 1 | focus=202 app=Mesh tint=0 |
| 3 | Apply Atlas accent | 1 | workspace update + tint=1 propagated |
| 4 | Focus Linen | 1 | focus=200 app=Linen tint=1 |
| 5 | ABI gap docs | 1 | 2 blockers documented |

## Runtime Proof Counts

```
[shell.silkbar.status.send]  focus=153 app=Spindle tint=0 bell=0 ok=1 reason=focus_set
[shell.silkbar.status.send]  focus=204 app=Bell    tint=0 bell=0 ok=1 reason=focus_set
[shell.silkbar.status.send]  focus=202 app=Mesh    tint=0 bell=0 ok=1 reason=focus_set
[shell.silkbar.status.send]  focus=200 app=Linen   tint=1 bell=0 ok=1 reason=focus_set  ← tint propagated!
[shell.silkbar.status.send]  focus=201 app=Quil    tint=1 bell=0 ok=1 reason=focus_set
[silkbar.workspace.recv]     index=0
[silkbar.workspace.active.set] index=0
[silkbar.keyboard.status.proof]       stage=0-5 all ok=1
[silkbar.keyboard.status.proof.blocker] name=active_app_name reason=no_UpdateKind_variant
[silkbar.keyboard.status.proof.blocker] name=tint_accent reason=no_UpdateKind_variant
[silkbar.keyboard.status.proof.done]   ok=1
faults: 0
```

Key observation: `focus=200 app=Linen tint=1` — the Atlas accent apply (stage 3)
propagates accent=1 to ACTIVE_TINT_IDX, and subsequent focus changes report the
updated tint. SilkBar receives the focus state update via OP_SILKBAR_FOCUS_STATE
but cannot display the tint (blocker documented).

## Files Changed

`servers/silk-shell/src/main.rs`
- Added `SILKBAR_KEYBOARD_STATUS_PROOF_ENABLED` const
- Added `SILKBAR_KEYBOARD_STATUS_PROOF_DONE` static flag
- Added `[shell.silkbar.status.send]` markers in `try_set_focus()` (focus clear + focus set branches)
- Added `[shell.silkbar.status.send]` marker + `OP_SILKBAR_WORKSPACE_ACTIVE` send in `switch_scene()`
- Added `maybe_run_silkbar_keyboard_status_proof()` proof function (~100 lines)
- Added proof call site in main loop

`docs/handoff/SILKBAR_KEYBOARD_STATUS_INTEGRATION_V1.md` (created)

## Build Results
```
SEXOS_SILKBAR_KEYBOARD_STATUS_PROOF=1 ./scripts/entrypoint_build.sh → PASS
./scripts/entrypoint_build.sh → PASS
```

## Notes
- No kernel, sex-pdx/ABI, sexusb, sexinput, sexdisplay, or Quil edits.
- `OP_SILKBAR_WORKSPACE_ACTIVE` now sent from `switch_scene()` so keyboard-driven
  scene switches (Atlas Enter, next_scene, prev_scene) update SilkBar workspace chips.
  Previously only sent from click handler and boot sequence.
- Tint propagation from Atlas accent (ATLAS_THEME_APPLY_VISUAL_PROOF_V1) now visible
  in SilkBar status markers — tint changes from 0→1 after accent apply.
- Bell count shows 0 because silk-shell's local BELL_EVENTS ring is populated
  independently from the Bell server queue (see BELL_EVENT_DETAIL_SEED_V1).
