# APP_LAUNCH_EXEC_REVISIT_SLOTSHELL_V1

## Result: PASS IMPLEMENTED — 84/84 gates

## Previous STOP FIRST (V3)
Spindle had no SLOT_SHELL. Cross-PD launch blocked.

## Current SLOT_SHELL Truth
- SLOT_SHELL=6 granted to Spindle PD (kernel config, proven)
- Spindle sends pdx_call(SLOT_SHELL, 0x15, app_id, 0, 0) fire-and-forget
- silk-shell receives OP_SHELL_LAUNCH_REQUEST=0x15, calls existing focus/open
- App-local protocol only (both sides define own consts, matches 0x47 pattern)

## App Launch Exec Truth
| App | launch_exec | Route |
|-----|-------------|-------|
| Spindle | 1 | self-hosted |
| Quil | 1 | SLOT_SHELL→open_quil_in_active_scene |
| Linen | 1 | SLOT_SHELL→open_linen_in_active_scene |
| Bell/Atlas/Collar/Mesh | 0 | no focus path yet (deferred) |
| WebStub | 0 | no surface, network=0 |

## Markers
- Spindle: `[spindle.launch.request] app=NAME status=0` (enqueued)
- Shell: `[shell.launch.request.recv]` + `[shell.launch.request.exec]`

## Safety
No kernel/pdx/global ABI changes. App-local opcode 0x15. 4 files, +55 lines.
