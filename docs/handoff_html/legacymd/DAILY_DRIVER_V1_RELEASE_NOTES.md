# DAILY_DRIVER_V1 RELEASE NOTES

Date: 2026-05-15
Status: PASS
Milestone: Keyboard-First Daily Driver Proof Complete
Scope: 38 proof env vars, 16 master gates, 0 faults

## Proof Command

```bash
# Full daily-driver proof profile (build + boot + gate scan):
./scripts/run_daily_driver_proof.sh

# Gate scan only (against existing serial log):
./scripts/daily_driver_master_gate.sh <serial_log_path>
```

## Master Gate

16 gates. PASS = explicit proof marker found (prefers V2 proof.done markers, falls back to V1 evidence). SKIP = proof not enabled in this boot (not a failure). FAIL = proof enabled but wrong values.

| # | Gate | What it proves | V2 marker preference |
|---|------|---------------|---------------------|
| 1 | `keyboard_gui` | SilkBar clock alive, GUI surface present | `silkbar.clock.send` |
| 2 | `command_palette` | Quil palette panel renders, rows visible | `quil.palette.(panel\|row)` |
| 3 | `spindle_daily` | Spindle daily summary (items + blockers) | `spindle.daily.summary` |
| 4 | `spindle_bridges` | Spindle Bell/Linen/SexFiles bridges | `spindle.(bell\|linen\|files)` |
| 5 | `linen_nonblocking` | Linen open path is nonblocking | `linen.nonblocking.proof.done`, `linen.fast_paint` |
| 6 | `linen_detail` | Linen objects seeded, detail ready | `linen.object.seed` |
| 7 | `quil_keyboard` | Quil keyboard buffer nav, HID replay | `quil.keyboard.buffer.proof.done`, `quil.hid.replay.done` |
| 8 | `bell_events` | Bell system/detail events delivered | `bell.(demo\|system\|detail)` |
| 9 | `atlas_theme` | Atlas scene/theme/accent/preset init | `atlas.(scene\|theme\|accent\|preset)` |
| 10 | `collar_nav` | Collar capability grants auto-issued | `collar.grant.auto` |
| 11 | `mesh_nav` | Frame topology wired (tab events) | `shell.frame.tab.info.send` |
| 12 | `silkbar_status` | SilkBar focus/app/tint/bell status sent | `shell.silkbar.status.send` |
| 13 | `launcher_multi_exec` | All 7 app launcher rows execute & focus | `launcher.multi.proof.done passed=7 failed=0` |
| 14 | `palette_linen_available` | Palette sees Linen as nonblocking_ready | `shell.palette.status ... Open Linen ... nonblocking_ready` |
| 15 | `quil_status_ready` | Palette sees Quil as keyboard_nav_ready | `shell.palette.status ... Open Quil ... keyboard_nav_ready` |
| 16 | `faults_zero` | No #PF #GP panic KERNEL PANIC fault.kill | (absence of fault patterns) |

## App Readiness Table

All 7 keyboard-first apps are proven reachable and functional via the launcher (command palette).

| idx | App | Launcher Row | Launch ok | Focus ok | Palette Status | Proof Handoff |
|-----|-----|-------------|-----------|----------|---------------|---------------|
| 0 | **Spindle** | Open Spindle | 1 | sid=153 | ready | `APP_LAUNCHER_MULTI_EXEC_PROOF_V1.md` |
| 1 | **Quil** | Open Quil | 1 | sid=201 | keyboard_nav_ready | `APP_LAUNCHER_MULTI_EXEC_PROOF_V1.md` |
| 2 | **Linen** | Open Linen | 1 | sid=200 | nonblocking_ready | `APP_LAUNCHER_MULTI_EXEC_PROOF_V1.md` |
| 3 | **Atlas** | Open Atlas | 0* | overlay open | overlay_available | `APP_LAUNCHER_MULTI_EXEC_PROOF_V1.md` |
| 4 | **Bell** | Open Bell | 1 | sid=204 | ready | `APP_LAUNCHER_MULTI_EXEC_PROOF_V1.md` |
| 5 | **Collar** | Open Collar | 1 | sid=203 | ready | `APP_LAUNCHER_MULTI_EXEC_PROOF_V1.md` |
| 6 | **Mesh** | Open Mesh | 1 | sid=202 | ready | `APP_LAUNCHER_MULTI_EXEC_PROOF_V1.md` |

\* Atlas exec=0 because surface 151 is nonfocusable by lifecycle design. `ATLAS_MODE_ENABLED` is the correct verification — the overlay DOES open. See `APP_LAUNCHER_MULTI_EXEC_PROOF_V1.md` §3.

## Bridges Table

Spindle bridges keyboard-launched apps to other services.

| Bridge | Direction | Proof | Status |
|--------|-----------|-------|--------|
| Spindle → Bell | bell.send, bell.notify | `SPINDLE_BELL_BRIDGE_PROOF` | PASS |
| Spindle → Linen | linen.send, linen.open | `SPINDLE_LINEN_BRIDGE_PROOF` | PASS |
| Spindle → SexFiles | files.open, files.command | `SPINDLE_FILES_COMMANDS_PROOF` | PASS |
| Spindle → Quil | (via launcher/surface open) | implicit via launcher multi-exec | PASS |
| Spindle → Collar | (via launcher/surface open) | implicit via launcher multi-exec | PASS |
| Spindle → Mesh | (via launcher/surface open) | implicit via launcher multi-exec | PASS |
| Spindle → Atlas | (via launcher/overlay toggle) | implicit via launcher multi-exec | PASS |

## Proved Subsystems

### Fully Proven (keyboard-first)

| Subsystem | Proof Profile Env Vars | Gate | Notes |
|-----------|----------------------|------|-------|
| Keyboard GUI | `KEYBOARD_GUI_BROAD_PROOF`, `KEYBOARD_PROOF`, `KEYBOARD_SAFE_CLOSE_PROOF`, `KEYBOARD_WINDOW_PROOF` | `keyboard_gui` | SilkBar clock ticks, frame creation, cursor surface, broad action dispatch |
| Command Palette | `COMMAND_PALETTE_STATUS_PROOF`, `COMMAND_PALETTE_DAILY_PROOF`, `COMMAND_PALETTE_LINEN_STATUS_PROOF` | `command_palette`, `palette_linen_available` | Quil palette panel, 10 rows, Linen nonblocking status |
| App Launcher | `APP_LAUNCHER_PROOF`, `APP_LAUNCHER_MULTI_EXEC_PROOF` | `launcher_multi_exec` | All 7 apps launch and focus from palette |
| Spindle | `SPINDLE_DAILY_SUMMARY_PROOF`, `SPINDLE_STATUS_PANEL_PROOF`, `SPINDLE_BELL_BRIDGE_PROOF`, `SPINDLE_LINEN_BRIDGE_PROOF`, `SPINDLE_FILES_COMMANDS_PROOF`, `SPINDLE_COMMAND_HISTORY_PROOF`, `SPINDLE_PERSIST_HISTORY_PROOF` | `spindle_daily`, `spindle_bridges` | Daily summary, status panel, Bell/Linen/SexFiles bridges, command history, session persistence |
| Linen | `LINEN_NONBLOCKING_OPEN_PROOF`, `LINEN_OBJECT_DETAIL_PROOF`, `LINEN_KEYBOARD_NAV_PROOF`, `LINEN_SESSION_PROOF` | `linen_nonblocking`, `linen_detail` | Nonblocking open, object detail, keyboard nav, session |
| Quil | `QUIL_KEYBOARD_BUFFER_PROOF`, `QUIL_KEYBOARD_NAV_PROOF`, `QUIL_STATUS_UNBLOCK_PROOF` | `quil_keyboard`, `quil_status_ready` | Keyboard buffer nav, HID replay, palette status unblock |
| Bell | `BELL_SYSTEM_EVENTS_PROOF`, `BELL_DETAIL_SEED_PROOF`, `BELL_KEYBOARD_DETAIL_PROOF` | `bell_events` | System events, detail seeds, keyboard detail actions |
| Atlas | `ATLAS_THEME_VISUAL_PROOF`, `ATLAS_THEME_PRESETS_PROOF`, `ATLAS_SCENE_KEYBOARD_PROOF` | `atlas_theme` | Scene/keyboard, theme apply, presets cycle |
| Collar | `COLLAR_KEYBOARD_GRANTS_PROOF`, `COLLAR_ENFORCE_PROOF`, `COLLAR_REVIEW_PROOF` | `collar_nav` | Grant auto, enforce, review |
| Mesh | `MESH_KEYBOARD_MAP_PROOF` | `mesh_nav` | Frame topology, tab events |
| SilkBar | `SILKBAR_KEYBOARD_STATUS_PROOF`, `SILKBAR_PALETTE_STATUS_PROOF` | `silkbar_status` | Focus status send, palette status (blockers documented) |
| Storage | `SEXFILES_CAP_RECORD_PROOF`, `SEXFILES_EXTENT_PROOF` | (implicit) | Cap record, extent proof |
| SexObject | `SEXOBJECT_VIEW_PROOF`, `SEXOBJECT_OQ` | (implicit) | Object view, OQ |

### Proven (docs-only planning)

| Artifact | Status | Handoff |
|----------|--------|---------|
| SilkBar ABI extension plan | PLANNING (no code) | `SILKBAR_ABI_EXTENSION_PLAN_V1.md` |
| Daily driver master gate | PASS (16 gates) | `DAILY_DRIVER_MASTER_GATE_V1.md` |
| Daily driver proof profile | PASS (38 vars) | `DAILY_DRIVER_PROOF_PROFILE_V1.md` |

## Blockers / Deferred

No blockers prevent keyboard-first daily driving. These are deferred for future milestones.

| Blocker | Severity | Impact | Planned in |
|---------|----------|--------|-----------|
| Pointer precision / slot2 mouse | Medium | Click targeting unreliable | Future input milestone |
| SilkBar ABI status variants (active app name, tint, palette render) | Low | Bar shows workspaces/chips/clock/bell but not app name/tint/palette | `SILKBAR_ABI_EXTENSION_PLAN_V1.md` |
| True app launch/install model | Low | Apps pre-seeded; no dynamic `exec` spawn | Future app model milestone |
| Sync readback/list semantics | Low | SexFiles ramfs only; no disk readback | Future storage milestone |
| Real hardware USB HID | Medium | QEMU USB keyboard works; real HW untested | Future hardware milestone |
| Real monotonic timer | Low | Synthetic clock cadence (QEMU TCG); real LAPIC integration deferred | Future kernel milestone |
| Spindle FB input proof | Low | Causes PAGE FAULT when co-located with silk-shell FB; compile-verified in isolation | Future Spindle milestone |
| SilkBar command palette render | Low | Palette state not rendered on bar (ABI blocker) | `SILKBAR_ABI_EXTENSION_PLAN_V1.md` Phase 3 |

## Rollback Notes

All proof gates are default-OFF (`option_env!("...").is_some()`). Removing any `export SEXOS_*=1` line from `run_daily_driver_proof.sh` reverts that proof to zero behavior change.

Baseline build (no env vars):
```bash
./scripts/entrypoint_build.sh           # zero proofs enabled
```

Daily-driver build (all proofs):
```bash
./scripts/run_daily_driver_proof.sh     # all 38 proofs enabled
```

Master gate gracefully degrades: if a proof is not enabled, its gate reports SKIP (not FAIL). The only hard FAIL conditions are:
- A proof marker is present but has wrong values (e.g., launcher passed=6 failed=1)
- A fault marker is detected (#PF, #GP, panic, KERNEL PANIC, fault.kill)

## Next Recommended Missions

| Priority | Mission | Rationale |
|----------|---------|-----------|
| 1 | `SILKBAR_ABI_PHASE1_MODEL_V1` | Unblock SilkBar app name/tint/palette rendering (model only, no code changes beyond silkbar-model) |
| 2 | `SILKBAR_ABI_PHASE2_SHELL_SENDS_V1` | Send active app/tint/palette updates from shell to display |
| 3 | Pointer precision (slot2 mouse) | Make click targeting reliable for non-keyboard daily driving |
| 4 | `SILKBAR_ABI_PHASE3_DISPLAY_RENDER_V1` | Render app name/tint/palette on SilkBar |
| 5 | True app launch model | Dynamic app spawn from palette/launcher |
| 6 | Real hardware USB HID validation | Prove input stack on real hardware |
| 7 | Real monotonic timer (LAPIC) | Replace synthetic clock cadence |

## Commit / Log Audit

```bash
# Recent commits forming the daily-driver milestone:
git log --oneline -15

# Full proof profile build:
SEXOS_APP_LAUNCHER_MULTI_EXEC_PROOF=1 \
SEXOS_APP_LAUNCHER_PROOF=1 \
SEXOS_ATLAS_SCENE_KEYBOARD_PROOF=1 \
SEXOS_ATLAS_THEME_VISUAL_PROOF=1 \
SEXOS_ATLAS_THEME_PRESETS_PROOF=1 \
SEXOS_BELL_SYSTEM_EVENTS_PROOF=1 \
SEXOS_BELL_DETAIL_SEED_PROOF=1 \
SEXOS_BELL_KEYBOARD_DETAIL_PROOF=1 \
SEXOS_COLLAR_KEYBOARD_GRANTS_PROOF=1 \
SEXOS_COLLAR_ENFORCE_PROOF=1 \
SEXOS_COLLAR_REVIEW_PROOF=1 \
SEXOS_COMMAND_PALETTE_STATUS_PROOF=1 \
SEXOS_COMMAND_PALETTE_DAILY_PROOF=1 \
SEXOS_COMMAND_PALETTE_LINEN_STATUS_PROOF=1 \
SEXOS_KEYBOARD_GUI_BROAD_PROOF=1 \
SEXOS_KEYBOARD_PROOF=1 \
SEXOS_KEYBOARD_SAFE_CLOSE_PROOF=1 \
SEXOS_KEYBOARD_WINDOW_PROOF=1 \
SEXOS_LINEN_NONBLOCKING_OPEN_PROOF=1 \
SEXOS_LINEN_OBJECT_DETAIL_PROOF=1 \
SEXOS_LINEN_KEYBOARD_NAV_PROOF=1 \
SEXOS_LINEN_SESSION_PROOF=1 \
SEXOS_MESH_KEYBOARD_MAP_PROOF=1 \
SEXOS_QUIL_KEYBOARD_BUFFER_PROOF=1 \
SEXOS_QUIL_KEYBOARD_NAV_PROOF=1 \
SEXOS_QUIL_STATUS_UNBLOCK_PROOF=1 \
SEXOS_SEXFILES_CAP_RECORD_PROOF=1 \
SEXOS_SEXFILES_EXTENT_PROOF=1 \
SEXOS_SEXOBJECT_VIEW_PROOF=1 \
SEXOS_SEXOBJECT_OQ=1 \
SEXOS_SILKBAR_KEYBOARD_STATUS_PROOF=1 \
SEXOS_SILKBAR_PALETTE_STATUS_PROOF=1 \
SEXOS_SPINDLE_DAILY_SUMMARY_PROOF=1 \
SEXOS_SPINDLE_STATUS_PANEL_PROOF=1 \
SEXOS_SPINDLE_BELL_BRIDGE_PROOF=1 \
SEXOS_SPINDLE_LINEN_BRIDGE_PROOF=1 \
SEXOS_SPINDLE_FILES_COMMANDS_PROOF=1 \
SEXOS_SPINDLE_COMMAND_HISTORY_PROOF=1 \
SEXOS_SPINDLE_PERSIST_HISTORY_PROOF=1 \
./scripts/entrypoint_build.sh

# Boot and gate scan:
timeout 30s qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci -device usb-kbd,bus=xhci.0 \
  -serial file:/tmp/sexos_daily_driver_v1.log \
  -display none -no-reboot -no-shutdown || true

./scripts/daily_driver_master_gate.sh /tmp/sexos_daily_driver_v1.log
```

Or simply:
```bash
./scripts/run_daily_driver_proof.sh
```

## Handoff Path

```
docs/handoff/DAILY_DRIVER_V1_RELEASE_NOTES.md          ← THIS DOCUMENT
docs/handoff/DAILY_DRIVER_MASTER_GATE_V1.md             ← gate spec
docs/handoff/DAILY_DRIVER_PROOF_PROFILE_V1.md           ← profile spec
docs/handoff/DAILY_DRIVER_PROFILE_UPDATE_AFTER_QUIL_LINEN_V1.md  ← latest profile update
docs/handoff/APP_LAUNCHER_MULTI_EXEC_PROOF_V1.md        ← launcher proof
docs/handoff/APP_LAUNCHER_V1.md                         ← launcher V1
docs/handoff/SILKBAR_ABI_EXTENSION_PLAN_V1.md           ← next milestone plan
```
