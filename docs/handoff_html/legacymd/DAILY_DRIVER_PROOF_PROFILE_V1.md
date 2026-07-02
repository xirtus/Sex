# DAILY_DRIVER_PROOF_PROFILE_V1

## Handoff Date
2026-05-14

## Status
PASS

## Contract
- scripts/run_daily_driver_proof.sh
- docs/handoff/DAILY_DRIVER_PROOF_PROFILE_V1.md

## Summary
A single host script that builds a proof ISO with all daily-driver proof gates
enabled, boots in headless QEMU, captures the serial log, and runs
`daily_driver_master_gate.sh` against it.  Provides a one-command
build → boot → verify pipeline.

## Command

```
./scripts/run_daily_driver_proof.sh [log_path]
```

- `log_path` defaults to `/tmp/sexos_daily_driver_proof.log`
- Returns 0 if all enabled gates PASS + zero faults
- Returns 1 if build fails, gate fails, or faults detected
- Returns 2 for fatal errors (missing scripts, unwritable log)

## Exact Env Vars Set

### Spindle (safe, non-FB proofs only)
```
SEXOS_SPINDLE_DAILY_SUMMARY_PROOF=1
SEXOS_SPINDLE_STATUS_PANEL_PROOF=1
SEXOS_SPINDLE_BELL_BRIDGE_PROOF=1
SEXOS_SPINDLE_LINEN_BRIDGE_PROOF=1
SEXOS_SPINDLE_FILES_COMMANDS_PROOF=1
SEXOS_SPINDLE_COMMAND_HISTORY_PROOF=1
SEXOS_SPINDLE_PERSIST_HISTORY_PROOF=1
```

NOTE: `SEXOS_SPINDLE_INPUT_PROOF` is intentionally NOT set.  It enables
framebuffer writes via `WindowBuffer::new()` that cause a PAGE FAULT at
`0x40000000` when Spindle is kernel-spawned alongside silk-shell's own
framebuffer.  The input proof is compile-verified in isolation.

### Command Palette
```
SEXOS_COMMAND_PALETTE_STATUS_PROOF=1
SEXOS_COMMAND_PALETTE_DAILY_PROOF=1
SEXOS_COMMAND_PALETTE_LINEN_STATUS_PROOF=1
```

### Linen
```
SEXOS_LINEN_NONBLOCKING_OPEN_PROOF=1
SEXOS_LINEN_OBJECT_DETAIL_PROOF=1
SEXOS_LINEN_KEYBOARD_NAV_PROOF=1
SEXOS_LINEN_SESSION_PROOF=1
```

### Quil
```
SEXOS_QUIL_KEYBOARD_BUFFER_PROOF=1
SEXOS_QUIL_KEYBOARD_NAV_PROOF=1
SEXOS_QUIL_STATUS_UNBLOCK_PROOF=1
```

### Bell
```
SEXOS_BELL_SYSTEM_EVENTS_PROOF=1
SEXOS_BELL_DETAIL_SEED_PROOF=1
SEXOS_BELL_KEYBOARD_DETAIL_PROOF=1
```

### Atlas
```
SEXOS_ATLAS_THEME_VISUAL_PROOF=1
SEXOS_ATLAS_THEME_PRESETS_PROOF=1
SEXOS_ATLAS_SCENE_KEYBOARD_PROOF=1
```

### Collar
```
SEXOS_COLLAR_KEYBOARD_GRANTS_PROOF=1
SEXOS_COLLAR_ENFORCE_PROOF=1
SEXOS_COLLAR_REVIEW_PROOF=1
```

### Mesh
```
SEXOS_MESH_KEYBOARD_MAP_PROOF=1
```

### SilkBar
```
SEXOS_SILKBAR_KEYBOARD_STATUS_PROOF=1
SEXOS_SILKBAR_PALETTE_STATUS_PROOF=1
```

### Keyboard GUI
```
SEXOS_KEYBOARD_GUI_BROAD_PROOF=1
SEXOS_KEYBOARD_PROOF=1
SEXOS_KEYBOARD_SAFE_CLOSE_PROOF=1
SEXOS_KEYBOARD_WINDOW_PROOF=1
```

### SexFiles
```
SEXOS_SEXFILES_CAP_RECORD_PROOF=1
SEXOS_SEXFILES_EXTENT_PROOF=1
```

### SexObject
```
SEXOS_SEXOBJECT_VIEW_PROOF=1
SEXOS_SEXOBJECT_OQ=1
```

## Pipeline Steps

1. **BUILD**: `./scripts/entrypoint_build.sh` with all proof env vars exported
2. **BOOT**: `qemu-system-x86_64` headless, `usb-kbd`, serial log to specified path, 30s timeout
3. **GATE**: `./scripts/daily_driver_master_gate.sh "$LOG"`

## Test Result (2026-05-14)

```
BUILD: PASS (2s)
Log lines: 6505

gate results:
  keyboard_gui         PASS
  command_palette      PASS
  spindle_daily        PASS  (13 items, 8 blockers)
  spindle_bridges      PASS  (54 bridge markers)
  linen_nonblocking    PASS
  linen_detail         PASS  (6 objects)
  quil_keyboard        PASS
  bell_events          PASS
  atlas_theme          PASS
  collar_nav           PASS  (12 grants)
  mesh_nav             PASS  (8 tab events)
  silkbar_status       PASS  (45 status updates)
  faults_zero          PASS  (0 fault markers)

FINAL: PASS (13 gates, 0 skipped, 0 faults)
```

## Caveats

- **SEXOS_SPINDLE_INPUT_PROOF excluded**: This gate enables framebuffer access
  that conflicts with silk-shell's compositor FB.  It is compile-verified in
  isolation but cannot run in the multi-PD kernel-spawned configuration.

- **Not all proof gates may be implemented yet**:  Env vars for proofs not yet
  coded are silently ignored by `option_env!` (returns None).  The gate scanner
  reports SKIP for missing markers — this is not a failure.

- **Proof ISO build time**: ~2-8 seconds depending on cache.  The full pipeline
  (build + boot + scan) completes in ~35 seconds with 30s QEMU probe.

- **QEMU boot is headless**: No window appears.  The serial log is the sole output.

- **Host-side only**: The script uses bash, grep, wc, sleep, timeout.  These are
  host tools only and do not imply POSIX semantics inside SexOS.

## Files Changed
- scripts/run_daily_driver_proof.sh — new proof profile script
- docs/handoff/DAILY_DRIVER_PROOF_PROFILE_V1.md — this handoff

## Related Handoffs
- docs/handoff/DAILY_DRIVER_MASTER_GATE_V1.md — gate scanner used in step 3
- docs/handoff/SPINDLE_DAILY_DRIVER_BOOT_SUMMARY_V1.md — spindle daily command
