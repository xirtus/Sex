# CURSOR_REAL_INPUT_M1_DIAGNOSTIC

**Date:** 2026-05-03
**Status:** MERGED

## Symptom

Real USB mouse/cursor movement has no budgeted diagnostic markers. All existing markers (`[sexinput.usb_mouse.*]`, `[shell.pointer.usb_state.*]`, `[shell.cursor_surface.move.*]`) are unbudgeted — they fire on every USB mouse report and would flood the serial log with real mouse use. Without budgeted markers, it's impossible to confirm real deltas are flowing without overwhelming the log.

## Audit Results

| Question | Finding |
|----------|---------|
| 1. Real USB mouse deltas decoded where? | sexinput lines 102-106, silk-shell lines 603-607 (identical decode) |
| 2. Cursor x/y state lives where? | silk-shell `static mut POINTER_X/Y` (lines 194-195) |
| 3. Cursor position sent to display where? | shell line 684: `pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_CURSOR, x, y)` |
| 4. Real movement suppressed by synthetic proof? | **No** — different message types (OP_USB_MOUSE_REPORT vs OP_HID_EVENT), independent handlers, one-shot bounded synthetic |
| 5. x/y clamped to framebuffer bounds? | **Yes** — `clamp(0, max_x)` at shell lines 628-629 |
| 6. Cursor 0x90 remains nonfocusable? | **Yes** — `is_focusable_surface(0x90)` → false, `try_set_focus()` rejects, `point_in_surface()` rejects |
| 7. sexdisplay only renderer? | **Yes** — shell owns cursor position, sexdisplay renders bitmap at received coords |

## Root Cause

No budgeted diagnostic markers exist for the real USB mouse delta path. The unbudgeted markers that exist:
- `[sexinput.usb_mouse.recv]` — fires on every USB report from sexusb
- `[sexinput.usb_mouse.shell_send.ok]` — fires on every forward to shell
- `[shell.cursor_surface.move.start/ok]` — fires on every cursor surface update
- `[shell.pointer.usb_state.ok]` — fires on every coordinate update

These are useful for development but become a log flood with real mouse input. Budgeted markers are needed to observe real deltas without overwhelming the trace.

## Fix

### 1. Budgeted real-delta marker in sexinput (line ~134)

Added `[sexinput.mouse.real.delta]` with budget 16 in sexinput's real USB mouse path (inside the `pdx_try_listen_raw(0)` handler, after normalization). This marker fires only for messages arriving from sexusb through the kernel IPC ring — NOT for synthetic proof paths (which send OP_HID_EVENT or direct OP_USB_MOUSE_REPORT to the shell, bypassing sexinput's real path).

### 2. Budgeted cursor-move marker in silk-shell (line ~701)

Added `[shell.cursor.move]` with budget 16 in silk-shell's OP_USB_MOUSE_REPORT handler, right after the cursor surface is moved via `pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE)`. This captures the final cursor coordinates sent to sexdisplay.

### Budget summary

| Marker | Budget | Source | Triggers for |
|--------|--------|--------|--------------|
| `[sexinput.mouse.real.delta]` | 16 | sexinput | Real USB mouse deltas from sexusb |
| `[shell.cursor.move]` | 16 | silk-shell | All cursor moves (real + synthetic USB reports) |

## Changed Invariants

1. Real USB mouse deltas are observable via `[sexinput.mouse.real.delta]` (budget 16) — confirms sexusb→sexinput path is live.
2. Cursor position sent to sexdisplay is observable via `[shell.cursor.move]` (budget 16) — confirms cursor surface move.
3. Both markers are budgeted: after N fires they silently stop, preventing log flood.
4. Synthetic proof paths do NOT trigger `[sexinput.mouse.real.delta]` — they send HID_EVENT directly, bypassing sexinput's real USB decode path.
5. Synthetic proof paths DO trigger `[shell.cursor.move]` when they send OP_USB_MOUSE_REPORT (click-focus proof) — this is intentional: all cursor movement is observable.

## Marker List

| Marker | Type | Budget | When |
|--------|------|--------|------|
| `[sexinput.mouse.real.delta]` | accept | 16 | Real USB mouse delta decoded in sexinput |
| `[shell.cursor.move]` | accept | 16 | Cursor surface position updated in shell |

## Verification

```bash
./scripts/entrypoint_build.sh

SEXUSB_XHCI_TRACE=0 timeout 15 ./dev.sh run-nographic \
  2>/tmp/cursor-real-input.trace | tee /tmp/cursor-real-input.log

grep -c 'sexinput.mouse.real.delta' /tmp/cursor-real-input.log  # ≤16
grep -c 'shell.cursor.move' /tmp/cursor-real-input.log          # ≤16
grep -c 'shell.focus.reject' /tmp/cursor-real-input.log         # = 0
grep -cE 'fault|panic' /tmp/cursor-real-input.log               # = 0
grep -c 'sexinput.synthetic.silkbar_click' /tmp/cursor-real-input.log  # ≥ 7
grep -c 'drag_proof.done' /tmp/cursor-real-input.log            # = 1
grep -cE 'shell.drag' /tmp/cursor-real-input.log                # > 0
```

## Verified Results (2026-05-03)

```
sexinput.mouse.real.delta:    15 (budgeted, ≤16)  ✅
shell.cursor.move:            16 (budgeted, ≤16)  ✅
shell.focus.reject.*:          0                   ✅
fault/panic:                   0                   ✅
sexinput.synthetic.silkbar_click:  8              ✅
drag_proof.done:               1                   ✅
shell.drag:                    4                   ✅
click_focus:                   3                   ✅
```

## STOP FIRST Conditions

1. Changes to kernel/sex-pdx/sexdisplay
2. Removing or reducing synthetic proof budget
3. Adding framebuffer writes outside sexdisplay
4. Changing cursor surface ID or focusability
5. Broad input refactor or shared-memory redesign
6. Replacing budgeted markers with unbudgeted equivalents
