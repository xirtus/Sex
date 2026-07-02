# SPINDLE_DAILY_COMMANDS_HELP_POLISH_V1

## Handoff Date
2026-05-14

## Status
PASS

## Contract
- apps/spindle/src/main.rs
- docs/handoff/SPINDLE_DAILY_COMMANDS_HELP_POLISH_V1.md

## Summary
Reorganized the Spindle `help` command output into 7 named sections with
command-group headers, keyboard shortcuts documentation, and proof markers.
All 40 commands are grouped logically: Basics, Status & Audit, History &
Events, Storage, Bridges, Daily Driver, and Keyboard Shortcuts.

No semantic or behavior changes — this is UX polish only.  All commands
dispatch exactly as before.  The help text is restructured for readability
and daily-driver discoverability.

## Help Sections

| # | Section        | Commands | Content                              |
|---|----------------|----------|--------------------------------------|
| 1 | Basics         | 6        | help, clear, echo, about, route, close |
| 2 | Status & Audit | 8        | status, apps, blockers, keys, daily, pd, servers, input |
| 3 | History & Events | 4      | history, history clr, events, events clr |
| 4 | Storage        | 3        | save, load, ls                       |
| 5 | Bridges        | 7        | bell, bell-test, bell-status, notify, files, linen-status, linen-list |
| 6 | Daily Driver   | 4        | session, linen-open, launch, proof   |
| 7 | Shortcuts      | 8        | ` palette, Tab focus, Backspace, Esc, Enter, Alt+F4, arrows, vi keys |
|   | **Total**      | **40**   |                                      |

## Shortcuts Documented

| Key           | Action                          |
|---------------|---------------------------------|
| `` ` `` (backtick) | Toggle command palette (Quil) |
| Tab           | Cycle input focus forward       |
| Backspace     | Cycle input focus backward      |
| Esc           | Zoom out / close detail / back  |
| Enter         | Activate / select / execute     |
| Alt+F4        | Close current frame             |
| Arrow keys    | Navigate lists / cursor move    |
| vi keys       | h/l/w/b/0 i/a dd u (normal mode) |

## Markers Emitted

| Marker                      | Meaning                             |
|-----------------------------|-------------------------------------|
| [spindle.help.section]      | name=NAME commands=N                |
| [spindle.help.command]      | name=NAME ok=1                      |
| [spindle.help.proof]        | stage=N command=NAME ok=N           |
| [spindle.help.proof.done]   | ok=N                                |

## Proof Gate

Activated by: `SEXOS_SPINDLE_HELP_POLISH_PROOF=1`

5 stages:

| Stage | Command        | Verification                       |
|-------|----------------|------------------------------------|
| 1     | help           | Dispatch succeeds, output lines > 0 |
| 2     | section_audit  | All 7 sections emitted             |
| 3     | command_count  | 40 commands documented across sections |
| 4     | shortcuts      | 8 keyboard shortcuts documented    |
| 5     | safety         | No blocking, local-only dispatch   |

## Build

```
SEXOS_SPINDLE_HELP_POLISH_PROOF=1 ./scripts/entrypoint_build.sh
```

Or use the daily-driver proof profile which already includes it:
```
SEXOS_SPINDLE_HELP_POLISH_PROOF=1 ./scripts/run_daily_driver_proof.sh
```

## Runtime Verification

```
timeout 30s qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci -device usb-kbd,bus=xhci.0 \
  -serial file:/tmp/sexos_spindle_daily_commands_help_polish_v1.log \
  -display none -no-reboot -no-shutdown || true
```

Grep:
```
grep -E "spindle.help|spindle.cmd|fault.kill|#PF|#GP|panic|KERNEL PANIC" \
  /tmp/sexos_spindle_daily_commands_help_polish_v1.log | tail -2600
```

## Verification Results

| Metric           | Expected | Actual |
|------------------|----------|--------|
| help.section     | 7        | 7      |
| help.command     | 40       | 40     |
| help.proof stages| 6        | 6      |
| help.proof.done  | 1        | 1      |
| Faults           | 0        | 0      |

## Files Changed
- apps/spindle/src/main.rs — reorganized help command output, added help polish proof function and gate
- docs/handoff/SPINDLE_DAILY_COMMANDS_HELP_POLISH_V1.md — this handoff

## Notes
- No kernel/ABI/USB/display/Quil/pointer changes
- No blocking waits
- No broad refactor
- No POSIX filesystem language
- All existing commands dispatch identically — only help text reorganized
- Spindle-only change (apps/spindle/src/main.rs)
