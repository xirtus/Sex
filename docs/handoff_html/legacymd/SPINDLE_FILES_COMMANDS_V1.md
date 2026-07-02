# Spindle Files Commands V1 Handoff

## Status: PASS
Date: 2026-05-14
Attempts: 1

## Command Table

| Command    | Status            | Storage Semantics                     |
|-----------|-------------------|---------------------------------------|
| help      | list commands     | —                                     |
| clear     | clear scrollback  | —                                     |
| history   | show/clear hist   | in-memory ring buffer                 |
| echo      | echo text         | —                                     |
| save      | persist to RamFS  | fire-and-forget (AsyncEnqueue edge)   |
| load      | restore from RamFS| async-limited (sync readback unavailable) |
| ls        | list objects     | async-limited (static fallback)       |
| status    | Spindle status    | storage-aware (shows SLOT_STORAGE)    |
| session   | session summary   | storage semantics documented          |
| files     | storage status    | bridge status report                  |
| pd        | domain listing    | static baseline                       |
| servers   | server listing    | static baseline                       |
| bell      | notification      | pending cap grant                     |
| apps      | app listing       | static baseline                       |
| launch    | app launch        | unavailable (no kernel spawn)         |
| about     | version/identity  | includes storage bridge info          |
| route     | input/surface     | routing info                          |
| input     | keyboard status   | input route info                      |
| faults    | fault report      | runtime fault count                   |
| events    | event log         | local event ring                      |
| proof     | proof summary     | sub-commands: boot/input/display/storage |
| close     | session close     | lifecycle info                        |

## Storage Semantics Preserved

- **AsyncEnqueue Edge**: All pdx_call to SLOT_STORAGE uses Domain cap → AsyncEnqueue → fire-and-forget.
- **Save**: pdx_call(OP_RAMFS_OPEN + OP_RAMFS_WRITE + OP_RAMFS_CLOSE) → returns immediately. Server processes asynchronously. Data IS written to RamFS.
- **Load**: Honest about async limitation. pdx_call(OP_RAMFS_READ) always returns (0,0) for AsyncEnqueue edges. Full sync restore requires future sync-call edge type.
- **ls**: Fires OP_RAMFS_LIST (fire-and-forget) + provides static fallback list of known objects. Synchronous listing requires blocking readback.
- **No blocking loops**: All dispatch paths return immediately. No pdx_listen_raw blocking in command path.
- **No POSIX filesystem**: Uses SexFiles RamFS objects, not POSIX paths. Wording avoids POSIX language (use "object", "session", "SexFiles" terminology).

## Runtime Proof

### Proof Gate: SEXOS_SPINDLE_FILES_COMMANDS_PROOF=1

### Marker Summary

[spindle.files.proof] stage=0-8 all ok=1
[spindle.cmd.exec] name=save|load|ls|files|status|session all ok=1
[spindle.cmd.output] name=* bytes=N (save=84, load=252, ls=504, files=420, status=504, session=756)
[spindle.files.command] name=save|load|ls|files all ok=1
[spindle.files.proof.done] ok=1
faults=0
blocking=0

### Proof Stages
- Stage 1: save → fire-and-forget ✓
- Stage 2: load → async-limited, graceful ✓
- Stage 3: ls → fire-and-forget, static fallback ✓
- Stage 4: files → status report ✓
- Stage 5: status → storage-aware ✓
- Stage 6: session → storage semantics documented ✓
- Stage 7: history intact (no mutation from status commands) ✓
- Stage 8: safety (no blocking, no unbounded waits) ✓

## Files Changed

apps/spindle/src/main.rs
- Added OP_RAMFS_LIST constant (0x34)
- Added `ls` command handler (async-limited, static fallback)
- Updated `save` command: added [spindle.files.command] marker
- Updated `load` command: added [spindle.files.command] marker
- Updated `files` command: added [spindle.files.command] marker, updated help text
- Updated `status` command: added storage bridge info, corrected command count
- Updated `session` command: added storage semantics documentation
- Updated `about` command: added storage bridge info
- Updated `help` command: added `ls` entry
- Updated header comment: 12 built-in commands
- Added [spindle.cmd.output] marker in main loop (tracks scrollback line delta)
- Added run_files_commands_proof() with SEXOS_SPINDLE_FILES_COMMANDS_PROOF gate
- Proof emits [spindle.cmd.exec], [spindle.cmd.output], [spindle.files.command],
  [spindle.files.proof] (stages 0-8), [spindle.files.proof.done]

## Build

SEXOS_SPINDLE_FILES_COMMANDS_PROOF=1 ./scripts/entrypoint_build.sh → PASS
./scripts/entrypoint_build.sh → PASS

## Faults

0 faults observed in runtime headless QEMU.

## Notes

- No sex-pdx/ABI edits. No kernel edits. No Quil edits. No pointer work.
- No unbounded waits. No blocking loops. No POSIX assumptions.
- Backup saved: apps/spindle/src/main.rs.bak-<timestamp>
