# BOOT_COMPOSITION_ORDER_PROOF_V1

Date: 2026-05-06

## Root Cause
Confirmed: this was composition semantics, not a new geometry/kernel/display failure.

`sexdisplay` composites:
1. Non-focused surfaces first
2. Focused surface last/top

Boot focus is Quil (`sid=201`), and Quil now correctly occupies full content rect, so Linen (`sid=200`) is covered by design.

## Decision Implemented
Option 2: Linen hidden-consistent at boot.
- Keep focus on Quil (sid=201).
- Keep boot geometry fix (Quil full content rect).
- Do not claim Linen is boot-visible when focused Quil overlays it.

## Marker Updates
Added/updated boot truth markers:
- `[silk-shell.compose.order] focused_top=1 focus=201`
- `[silk-shell.boot.surface.visible] sid=201 visible=...`
- `[silk-shell.boot.surface.hidden] sid=200 reason=focused_quil_covers`
- `[silk-shell.boot.zorder] visible_count=1 first=201`
- `[silk-shell.boot.ui.ready] surfaces=1 focus=201`

Fallback markers:
- `[silk-shell.boot.surface.hidden] sid=200 reason=inactive_or_tombstoned`
- `[silk-shell.boot.zorder.reject] reason=focused_not_visible sid=201`

## Build
- `./scripts/entrypoint_build.sh` passes.

## Notes
- No kernel changes.
- No ABI/slot/opcode changes.
- No sexdisplay policy changes.
- No geometry rollback.
