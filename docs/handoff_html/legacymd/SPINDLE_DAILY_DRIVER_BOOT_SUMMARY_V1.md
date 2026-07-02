# SPINDLE_DAILY_DRIVER_BOOT_SUMMARY_V1

## Handoff Date
2026-05-14

## Status
PASS

## Contract
- apps/spindle/src/main.rs
- docs/handoff/SPINDLE_DAILY_DRIVER_BOOT_SUMMARY_V1.md

## Summary
Added `daily` command to Spindle that provides a truthful daily-driver boot
summary of the current keyboard-usable OS state.  All output is local-only
(no PDX calls, no blocking, no unbounded waits).  The summary reports:
- Keyboard control surface geometry and input route
- App keyboard readiness table (PASS/DEFER for each app)
- Command palette availability count (22 entries)
- Active bridges: Bell, Linen, SexFiles (all AsyncEnqueue)
- Honest blocker/deferred list: pointer precision, slot2 mouse,
  SilkBar ABI app/tint/palette variants, kernel spawn blockers

## Command
`daily` — provides the summary via scrollback output and serial log markers.

Usage: type `daily` at the Spindle `sex> ` prompt.

## Markers Emitted

| Marker                         | Meaning                               |
|--------------------------------|---------------------------------------|
| [spindle.daily.summary]        | ok=N bytes=N (summary aggregate)      |
| [spindle.daily.item]           | name=NAME status=NAME reason=...      |
| [spindle.daily.blocker]        | name=NAME reason=...                  |
| [spindle.daily.proof]          | stage=N command=NAME ok=N             |
| [spindle.daily.proof.done]     | ok=N (proof aggregate)                |

## Items Tracked

### Surface
- name=surface status=PASS reason=80x24_cp437_keyboard_control_center

### Apps
- name=Spindle status=PASS reason=terminal_commands_history_files
- name=Linen status=PASS reason=keyboard_nav_open_nonblocking_done
- name=Bell status=PASS reason=detail_seed_notify_bridge
- name=Atlas status=PASS reason=scene_accent_theme_apply
- name=Collar status=PASS reason=keyboard_grants_nav
- name=Mesh status=PASS reason=keyboard_map_nav
- name=Quil status=PASS reason=keyboard_nav_ready_stash_replay
- name=Pointer status=DEFER reason=usb_slot2_mouse_precision
- name=Palette status=PASS reason=22_command_entries

### Bridges
- name=Bell_bridge status=ACTIVE reason=SLOT_BELL_async_enqueue
- name=Linen_bridge status=ACTIVE reason=SLOT_LINEN_async_enqueue
- name=SexFiles_bridge status=ACTIVE reason=SLOT_STORAGE_async_enqueue

### Blockers
- name=pointer_precision reason=USB_slot2_mouse_deferred
- name=silkbar_app_name reason=no_UpdateKind_variant_ABI
- name=silkbar_tint reason=no_UpdateKind_variant_ABI
- name=silkbar_palette_variants reason=deferred
- name=app_launch reason=kernel_spawn_SLOT_SHELL_needed
- name=sync_load reason=pdx_call_READ_returns_zero
- name=sync_list reason=OP_RAMFS_LIST_async_reply_only
- name=real_HID_input reason=spindle_not_kernel_spawned

## Proof Gate

Activated by: `SEXOS_SPINDLE_DAILY_SUMMARY_PROOF=1`

The proof function `run_daily_driver_boot_summary_proof` executes the `daily`
command at boot and verifies all items, blockers, and bridges are reported.
5 stages:

| Stage | Command       | Verification                          |
|-------|---------------|---------------------------------------|
| 1     | daily         | Command dispatch succeeds             |
| 2     | item_audit    | All 9 app items present and truthful  |
| 3     | blocker_audit | All 8 blockers listed honestly        |
| 4     | bridge_audit  | All 3 bridges reported active         |
| 5     | safety        | No blocking, no PDX calls, no faults  |

## Build

```
SEXOS_SPINDLE_DAILY_SUMMARY_PROOF=1 ./scripts/entrypoint_build.sh
```

The env var `SEXOS_SPINDLE_DAILY_SUMMARY_PROOF` passes through to cargo build
and is picked up by `option_env!` at compile time in apps/spindle/src/main.rs.

## Runtime Verification

```
timeout 30s qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci -device usb-kbd,bus=xhci.0 \
  -serial file:/tmp/sexos_spindle_daily_driver_boot_summary_v1.log \
  -display none -no-reboot -no-shutdown || true
```

Grep markers:
```
grep -E "spindle.daily|spindle.status|spindle.cmd|fault.kill|#PF|#GP|panic|KERNEL PANIC" \
  /tmp/sexos_spindle_daily_driver_boot_summary_v1.log | tail -2600
```

## Pass Criteria
1. `[spindle.daily.summary] ok=1` present in serial log
2. All `[spindle.daily.item]` markers present (9 items)
3. All `[spindle.daily.blocker]` markers present (8 blockers)
4. `[spindle.daily.proof.done] ok=1` present
5. Zero `fault.kill`, `#PF`, `#GP`, `panic`, `KERNEL PANIC` matches
6. All app statuses truthful (matches current OS state)
7. All blockers listed honestly

## Files Changed
- apps/spindle/src/main.rs — added `daily` command, proof function, proof gate
- docs/handoff/SPINDLE_DAILY_DRIVER_BOOT_SUMMARY_V1.md — this handoff

## Notes
- No kernel/ABI/USB/display/Quil/pointer changes
- No blocking waits
- No broad refactor
- No POSIX filesystem language
- Spindle-only change (apps/spindle/src/main.rs)
