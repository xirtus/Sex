# SPINDLE_COMMAND_HISTORY_V1

Status: PASS
Date: 2026-05-14

## Scope
- Added bounded command history navigation (Up/Down) in Spindle key handling.
- Added required command/history proof markers.
- Added simple `echo` built-in command.
- Kept existing no-heap/no-std bounded structures and Enter/backspace behavior.

## Files Changed
- apps/spindle/src/main.rs
- docs/handoff/SPINDLE_COMMAND_HISTORY_V1.md

## Implementation Notes
- `CmdLine` now tracks history-nav state (`hist_nav`) and a saved pre-nav snapshot (`nav_saved`).
- `History::push` now returns ring index used for `[spindle.history.push]` marker.
- Added `history_nav(...)` helper:
  - `Up` (`0x48`) recalls older history entries.
  - `Down` (`0x50`) moves toward newer entries and restores current typed line when exiting nav.
- Added markers on normal Enter execution path:
  - `[spindle.cmd.recv]`
  - `[spindle.history.push]`
  - `[spindle.cmd.exec]`
- Added proof gate:
  - `option_env!("SEXOS_SPINDLE_COMMAND_HISTORY_PROOF").is_some()`
  - Runs `run_command_history_proof(...)` once during startup after state init.
- Added `echo` command:
  - `echo <text>` prints `<text>`
  - `echo` with empty args prints `echo: missing text`

## Build Results
1. `SEXOS_SPINDLE_COMMAND_HISTORY_PROOF=1 ./scripts/entrypoint_build.sh` -> PASS
2. `./scripts/entrypoint_build.sh` -> PASS

## Runtime Proof (Headless)
Command:
- `timeout 30s qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku -cdrom ./sexos-v1.0.0.iso -device nec-usb-xhci,id=xhci -device usb-kbd,bus=xhci.0 -serial file:/tmp/sexos_spindle_command_history_v1.log -display none -no-reboot -no-shutdown || true`

Key markers observed:
- `[spindle.command.history.proof] stage=0 action=start ok=1`
- `[spindle.cmd.recv] line_len=4`
- `[spindle.history.push] idx=0 len=4`
- `[spindle.cmd.exec] name=help ok=1 reason=ok`
- `[spindle.cmd.exec] name=echo ok=1 reason=ok`
- `[spindle.cmd.exec] name=history ok=1 reason=ok`
- `[spindle.cmd.exec] name=clear ok=1 reason=ok`
- `[spindle.history.nav] dir=up idx=0 len=5 ok=1`
- `[spindle.history.nav] dir=down idx=0 len=0 ok=1`
- `[spindle.command.history.proof.done] ok=1`

Counts:
- `spindle.cmd.recv`: 4
- `spindle.cmd.exec`: 5
- `spindle.history.push`: 4
- `spindle.history.nav`: 2
- `spindle.command.history.proof`: 9
- `spindle.command.history.proof.done`: 1
- faults (`fault.kill|#PF|#GP|panic|KERNEL PANIC`): 0

## Notes
- A prior zero-marker run was caused by using a normal non-proof rebuild after proof build, which replaced ISO contents. Final proof run used proof build immediately before runtime.
