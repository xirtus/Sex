# SILKBAR_ABI_PHASE2_SHELL_SEND_V1

Date: 2026-05-15
Status: PASS
Scope: servers/silk-shell/src/main.rs only — producer send path, zero receiver dependency

## 1. PASS/STOP FIRST

| Status | Meaning |
|--------|---------|
| **PASS** | All 3 new SilkBar update kinds sent from shell. Proof exercises all 3. 16/16 daily-driver gates PASS. Zero faults. |
| **STOP FIRST** | No kernel/sex-pdx/sexdisplay/silkbar edits. No opcode changes. No renderer dependency. Old receivers silently drop unknown kinds. |

## 2. Attempts Used

| Attempt | Result | Notes |
|---------|--------|-------|
| 1 | FAIL | pdx_call returns (u64,u64) tuple — send helper compared against integer |
| 2 | PASS | Fixed send helper to use fire-and-forget pdx_call pattern |

## 3. Send Table

| Kind | Hook Point(s) | Fields | Marker Pattern |
|------|-------------|--------|---------------|
| SetActiveApp (8) | `try_set_focus()` after status send, `switch_scene()` after ws notify | a=FOCUSED_SURFACE_ID, b=0 | `[shell.silkbar.phase2.send] kind=SetActiveApp` |
| SetTintAccent (9) | `try_set_focus()` after status send, `switch_scene()` after ws notify, `atlas_apply_scene_accent_to_chrome()` on change | a=ACTIVE_TINT_IDX (0-7), b=0 | `[shell.silkbar.phase2.send] kind=SetTintAccent` |
| SetPaletteState (10) | `toggle_command_palette()` open/close, `palette_select_next/prev()` on nav | a=packed(open\|selected<<1\|available<<9), b=0 | `[shell.silkbar.phase2.send] kind=SetPaletteState` |

### Packed Format (SetPaletteState)

```
a: u64 = open (bit 0) | selected<<1 (bits 1-8) | available<<9 (bits 9-16)
```

## 4. Files Changed

- `servers/silk-shell/src/main.rs` — additive producer sends:
  - Added `OP_SILKBAR_UPDATE` to sex_pdx import
  - Added `UpdateKind` to silkbar_model import
  - Added `SILKBAR_PHASE2_SHELL_PROOF_ENABLED` compile-time gate (SEXOS_SILKBAR_PHASE2_SHELL_PROOF)
  - Added `SILKBAR_PHASE2_SHELL_PROOF_DONE` / `SILKBAR_PHASE2_SHELL_PROOF_STAGE` state variables
  - Added `send_silkbar_phase2_update()` helper (~20 lines)
  - Added `maybe_run_silkbar_phase2_shell_proof()` proof function (~35 lines)
  - Hooked `try_set_focus()` with SetActiveApp + SetTintAccent sends
  - Hooked `switch_scene()` with SetActiveApp + SetTintAccent sends
  - Hooked `atlas_apply_scene_accent_to_chrome()` with SetTintAccent send (only on change)
  - Hooked `toggle_command_palette()` with SetPaletteState send (open + close)
  - Hooked `palette_select_next/prev()` with SetPaletteState send (on nav)
  - Called `maybe_run_silkbar_phase2_shell_proof()` from main loop
- `docs/handoff/SILKBAR_ABI_PHASE2_SHELL_SEND_V1.md` — this handoff

## 5. Build/Proof Result

```
SEXOS_SILKBAR_PHASE2_SHELL_PROOF=1 ./scripts/entrypoint_build.sh → PASS
./scripts/entrypoint_build.sh → PASS (baseline, zero behavior change)
./scripts/run_daily_driver_proof.sh → 16/16 PASS, 0 faults
```

## 6. Runtime Proof Counts

From 30s headless QEMU boot with `SEXOS_SILKBAR_PHASE2_SHELL_PROOF=1`:

```
[shell.silkbar.phase2.send] kind=SetActiveApp  a=201 b=0 ok=1 reason=sent
[shell.silkbar.phase2.send] kind=SetTintAccent a=0   b=0 ok=1 reason=sent
[shell.silkbar.phase2.send] kind=SetPaletteState a=0 b=0 ok=1 reason=sent
[shell.silkbar.phase2.send] kind=SetActiveApp  a=202 b=0 ok=1 reason=sent
... (7 total sends across normal boot events)

[silkbar.phase2.shell.proof] stage=0 action=start          ok=1 reason=phase2_proof_begin
[silkbar.phase2.shell.proof] stage=1 action=SetActiveApp   ok=1 reason=sent
[silkbar.phase2.shell.proof] stage=2 action=SetTintAccent  ok=1 reason=sent
[silkbar.phase2.shell.proof] stage=3 action=SetPaletteState ok=1 reason=sent
[silkbar.phase2.shell.proof.done] ok=1

Faults: 0
```

| Metric | Count |
|--------|-------|
| `shell.silkbar.phase2.send` markers | 7 |
| `silkbar.phase2.shell.proof` stages | 4 (0-3) |
| `silkbar.phase2.shell.proof.done` | 1 (ok=1) |
| SetActiveApp sends | 3 |
| SetTintAccent sends | 3 |
| SetPaletteState sends | 1 |
| Faults | 0 |

## 7. Preserved Constraints

- No kernel edits
- No sex-pdx edits (OP_SILKBAR_UPDATE already existed)
- No sexdisplay edits (consumer unchanged, silently drops kind=8/9/10)
- No silkbar daemon edits (shell sends directly to sexdisplay, bypassing relay)
- No opcode changes (OP_SILKBAR_UPDATE opcode and slot unchanged)
- No SilkBarUpdate struct change (still 16 bytes)
- Existing silkbar status sends preserved (OP_SILKBAR_FOCUS_STATE, OP_SILKBAR_WORKSPACE_ACTIVE)
- Baseline build has zero behavior change (SILKBAR_PHASE2_SHELL_PROOF_ENABLED = false → send returns early)
- Old sexdisplay receivers: silently drop kind=8/9/10 (existing `_ => false` in apply_update)
- Zero faults in both proof and baseline builds

## Handoff Path

```
docs/handoff/SILKBAR_ABI_PHASE2_SHELL_SEND_V1.md         ← THIS DOCUMENT
docs/handoff/SILKBAR_ABI_PHASE1_MODEL_V1.md                ← Phase 1 (model)
docs/handoff/SILKBAR_ABI_EXTENSION_PLAN_V1.md               ← design authority
```

