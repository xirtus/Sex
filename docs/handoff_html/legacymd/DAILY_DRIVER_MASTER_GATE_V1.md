# DAILY_DRIVER_MASTER_GATE_V1

## Handoff Date
2026-05-14

## Status
PASS

## Contract
- scripts/daily_driver_master_gate.sh
- docs/handoff/DAILY_DRIVER_MASTER_GATE_V1.md

## Summary
A host-side shell script that scans a SexOS serial boot log for daily-driver
readiness evidence across 13 marker groups.  It is a pure log scanner — no
source-code, kernel, ABI, USB, input, display, or app behavior changes.

The script accepts a single serial log path and prints a PASS/FAIL/SKIP table.
It returns exit code 0 if all enabled gates pass and zero faults are found.

## Script Usage

```
./scripts/daily_driver_master_gate.sh <serial_log_path>
```

Example:
```
./scripts/daily_driver_master_gate.sh /tmp/sexos_spindle_daily_driver_boot_summary_v1.log
```

## Marker Groups (13)

| # | Gate                | Evidence Patterns                                      |
|---|---------------------|--------------------------------------------------------|
| 1 | keyboard_gui        | `silkbar.clock.send`                                   |
| 2 | command_palette     | `quil.palette.(panel\|draw\|row\|selected)`              |
| 3 | spindle_daily       | `spindle.daily.summary`, `spindle.daily.item`, `blocker`|
| 4 | spindle_bridges     | `spindle.(bell\|linen\|files\|sexfiles\|daily.item.*bridge)`|
| 5 | linen_nonblocking   | `linen.*nonblock`, `linen.open.intent`, daily summary   |
| 6 | linen_detail        | `linen.object.seed`                                    |
| 7 | quil_keyboard       | `quil.(keyboard\|stash\|replay\|hid)`, `quil.buffer.seed` |
| 8 | bell_events         | `bell.(demo\|list\|detail\|event\|system)`               |
| 9 | atlas_theme         | `atlas.(scene\|theme\|accent\|preset)`                  |
|10 | collar_nav          | `collar.grant.(auto\|nav)`                              |
|11 | mesh_nav            | `shell.frame.(tab\|create\|topbar\|light)`              |
|12 | silkbar_status      | `shell.silkbar.status.send`, `silkbar.clock.send`      |
|13 | faults_zero         | Absence of `fault.kill`, `#PF`, `#GP`, `panic`, etc.   |

## Return Codes

| Exit | Meaning                                               |
|------|-------------------------------------------------------|
| 0    | All enabled gates PASS, faults = 0                    |
| 1    | Any enabled gate FAILS or faults detected             |
| 2    | Fatal error (log not found, etc.)                     |

## How to Build Proof ISO with Env Gates

The script is a log scanner only.  To produce a log with all proof gates enabled:

```bash
# Enable desired proof gates and build
SEXOS_SPINDLE_DAILY_SUMMARY_PROOF=1 \
SEXOS_SPINDLE_STATUS_PANEL_PROOF=1 \
SEXOS_SPINDLE_BELL_BRIDGE_PROOF=1 \
SEXOS_SPINDLE_LINEN_BRIDGE_PROOF=1 \
SEXOS_SPINDLE_FILES_COMMANDS_PROOF=1 \
./scripts/entrypoint_build.sh

# Boot the ISO in QEMU with serial log capture
timeout 30s qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci -device usb-kbd,bus=xhci.0 \
  -serial file:/tmp/sexos_boot.log \
  -display none -no-reboot -no-shutdown || true

# Scan the log
./scripts/daily_driver_master_gate.sh /tmp/sexos_boot.log
```

## Runtime Command (QEMU)

```
timeout 30s qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci -device usb-kbd,bus=xhci.0 \
  -serial file:/tmp/sexos_daily_driver_gate.log \
  -display none -no-reboot -no-shutdown || true
```

## Gate Scoring Logic

- **PASS**: At least one evidence marker from the group is found in the log.
- **SKIP**: No evidence markers found — proof gate not enabled in this boot.
- **FAIL**: Evidence expected but missing (applies to `keyboard_gui` which should always be present) or `faults_zero` fails.
- **FINAL**: PASS if all gates are PASS or SKIP and faults_zero is PASS. FAIL if any gate FAILS or faults found.

## Caveats / Deferred Blockers

- **Not all proofs are enabled by default.**  Each proof gate requires a specific
  `SEXOS_SPINDLE_*_PROOF=1` or `SEXOS_*_PROOF=1` environment variable at build time.
  Gates for disabled proofs will appear as SKIP — this is not a failure.

- **Pointer precision** (USB slot2 mouse) is deferred.  The `faults_zero` gate
  only checks for crash/dereference faults, not for missing pointer support.

- **SilkBar ABI blockers** (app name, tint, palette UpdateKind variants) are
  documented in Spindle's daily summary and blockers output but are not
  individually checked by this gate script.  They are ABI-level blockers that
  require silkbar protocol changes.

- **App launch** requires kernel spawn and SLOT_SHELL — deferred.

- **Synchronous readback** (pdx_call READ, OP_RAMFS_LIST sync) is async-only
  with the current Domain-cap AsyncEnqueue edge.  Deferred to future PDX
  protocol enhancement.

- The script uses basic POSIX `grep` / `printf` / `wc` but this is **host-side
  only**.  It does not imply or require POSIX filesystem semantics inside SexOS.

## Files Changed
- scripts/daily_driver_master_gate.sh — new gate script
- docs/handoff/DAILY_DRIVER_MASTER_GATE_V1.md — this handoff

## Notes
- No kernel/ABI/USB/input/display/app behavior changes
- No broad refactor
- No source-code changes (scripts/docs only)
- Host-side shell script — does not run inside SexOS
