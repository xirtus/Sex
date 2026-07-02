# SILKBAR_COMMAND_PALETTE_STATUS_RENDER_V1

Date: 2026-05-14
Scope:
- `servers/silk-shell/src/main.rs`
- `docs/handoff/SILKBAR_COMMAND_PALETTE_STATUS_RENDER_V1.md`

## Problem
SilkBar/toolbar should reflect command palette state (open/close, selected item,
available count) and active app status. The existing SilkBar protocol only
carries focus state and workspace changes — palette open/close is an overlay
toggle that does not change focus.

## Investigation

### Existing SilkBar Protocol
`silkbar-model::UpdateKind` variants (no ABI changes):
- `SetWorkspaceActive = 0`
- `SetWorkspaceUrgent = 1`
- `SetChipVisible = 2`
- `SetChipKind = 3`
- `SetClock = 4`
- `SetThemeToken = 5`
- `SetSelectedOptions = 6`
- `SetBellPresence = 7`

**Missing**: No `UpdateKind` variant for palette visibility, selected index,
or available item count. No variant for active app name or tint/accent either
(already documented in SILKBAR_KEYBOARD_STATUS_INTEGRATION_V1.md).

### Focus Path Analysis
- `OP_SILKBAR_FOCUS_STATE` is sent on every focus change
- Command palette open/close does NOT change focus (overlay toggle)
- Therefore the SilkBar receives zero palette-specific data
- `[shell.silkbar.status.send]` markers only fire on focus/workspace changes

## Fix Implemented

### 1. Shell Markers (`[shell.palette.statusbar]`)
Added to `toggle_command_palette()` (ungated — fires whenever palette opens/closes):
```
On open:  [shell.palette.statusbar] open=1 selected=N available=N
On close: [shell.palette.statusbar] open=0 selected=0 available=0
```
Available count = items where `palette_item_status().0 == true`.

### 2. Proof Function (`maybe_run_silkbar_palette_status_proof`)
Gate: `SEXOS_SILKBAR_PALETTE_STATUS_PROOF=1` (default OFF, zero behavior change)

Proof stages (single-call pattern):
1. Snapshot pre-open state (focus ID, palette_open flag)
2. Open palette → verify `[shell.palette.statusbar]` fires
3. Inspect items → count available
4. Close palette → verify `[shell.palette.statusbar]` fires
5. Document ABI gap

Key findings:
- `focus_changed=0` on both open and close → overlay doesn't change focus
- `[shell.palette.statusbar]` fires correctly (shell-local diagnostic)
- SilkBar cannot render palette state without new `UpdateKind`

### 3. ABI Gap Documentation (STOP FIRST)
Four documented blockers:
| Blocker | Reason |
|---------|--------|
| `palette_state` | No `UpdateKind` variant |
| `palette_visible` | No `UpdateKind` variant |
| `palette_selected` | No `UpdateKind` variant |
| `palette_available` | No `UpdateKind` variant |

The existing `OP_SILKBAR_FOCUS_STATE` → `SetWorkspaceUrgent` path only fires
on actual focus changes, not on overlay toggles.

To render palette state on SilkBar, silkbar-model would need new variants
(e.g., `SetPaletteVisible = 8`, `SetPaletteSelected = 9`) plus sexdisplay
render updates. Not done here (STOP FIRST).

### 4. Preserved Constraints
- No kernel edits
- No ABI/sex-pdx edits
- No USB/display edits
- No renderer redesign
- No `UpdateKind` additions
- silk-shell + docs only
- Zero behavior change when `SEXOS_SILKBAR_PALETTE_STATUS_PROOF` is unset

## Markers
| Marker | Meaning |
|--------|---------|
| `[shell.palette.statusbar] open=N selected=N available=N` | Palette state snapshot |
| `[silkbar.palette.status.proof] stage=N action=... ok=...` | Per-stage proof output |
| `[silkbar.palette.status.proof.fact] focus_unchanged=1 ...` | Proven: focus unchanged |
| `[silkbar.palette.status.proof.blocker] name=... reason=no_UpdateKind_variant` | ABI gap |
| `[silkbar.palette.status.proof.note] path=... gap=...` | Detailed gap note |
| `[silkbar.palette.status.proof.done] ok=1` | Proof complete |

## Build
```
SEXOS_SILKBAR_PALETTE_STATUS_PROOF=1 ./scripts/entrypoint_build.sh
./scripts/entrypoint_build.sh                              # baseline (zero change)
```

## Runtime
```
timeout 30s qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci -device usb-kbd,bus=xhci.0 \
  -serial file:/tmp/sexos_silkbar_command_palette_status_render_v1.log \
  -display none -no-reboot -no-shutdown || true
```

## Grep
```
grep -E "shell.palette.statusbar|silkbar.palette|shell.silkbar.status|silkbar.status|fault.kill|#PF|#GP|panic|KERNEL PANIC" \
  /tmp/sexos_silkbar_command_palette_status_render_v1.log | tail -2400
```

## Pass Criteria
- `[shell.palette.statusbar]` open=1 and open=0 markers
- `[silkbar.palette.status.proof.fact] focus_unchanged=1` (proven: overlay no focus switch)
- `[silkbar.palette.status.proof.blocker]` entries for all 4 palette attributes
- `[silkbar.palette.status.proof.done] ok=1`
- faults=0 (no fault.kill, #PF, #GP, panic, KERNEL PANIC)
