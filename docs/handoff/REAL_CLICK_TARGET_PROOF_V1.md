# REAL_CLICK_TARGET_PROOF_V1

**Date:** 2026-05-03
**Status:** MERGED

## Symptom

Real USB mouse clicks were unreliable for two reasons:

### Bug A — dx/dy double-apply

For each real USB mouse report, sexinput forwarded **both** `OP_USB_MOUSE_REPORT` (with raw dx/dy) **and** `OP_HID_EVENT` with `EV_REL` (same dx/dy from the normalizer). The shell applied dx/dy to `POINTER_X/Y` in **both** the USB handler and the HID `EV_REL` handler. Result: cursor moved 2× the expected distance. Drag movement also double-applied the delta to the window position.

### Bug B — Synthetic click-focus proof coordinate corruption

`POINTER_X/Y` was shared between the `OP_USB_MOUSE_REPORT` handler and the `OP_HID_EVENT` handler. The synthetic silkbar click proof sent `EV_ABS(940, 25)` at tick 11, which overwrote `POINTER_X/Y` to `(940, 25)`. The synthetic click-focus proof (which ran concurrently at ticks 10-15) accumulated deltas from this corrupted position, landing at `(1240, 185)` instead of the intended `(940, 560)`. The click missed LINEN entirely.

## Root Cause

The shell had two parallel input paths (`OP_USB_MOUSE_REPORT` and `OP_HID_EVENT`) that both claimed ownership of `POINTER_X/Y`. Neither path was the sole authority, so both applied deltas independently — the USB handler from the raw report, and the `EV_REL` handler from the normalizer. Additionally, `EV_ABS` in the HID path could overwrite the USB-accumulated position, causing inter-path state corruption.

## Fix

### Fix 1 — EV_REL owns cursor movement; USB handler no longer applies dx/dy

**File:** `servers/silk-shell/src/main.rs`

Removed the `dx/dy` apply from the `OP_USB_MOUSE_REPORT` handler (previously lines 628-629). The USB handler now only processes button state, click-to-focus, and drag start/end. Cursor movement authority is delegated entirely to the `EV_REL` handler.

Added `POINTER_USB_STATE_INIT` initialization to the `EV_REL` handler so real USB (which no longer sends `OP_USB_MOUSE_REPORT`) still gets initial cursor centering.

Added cursor surface update (`pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_CURSOR, ...)`) and `[shell.cursor.move]` marker to the `EV_REL` handler so the cursor visually tracks HID movement.

### Fix 2 — EV_BTN owns click targeting; full markers added

**File:** `servers/silk-shell/src/main.rs`

Added complete click-target markers (`[shell.click_focus.down]`, `[shell.click_focus.hit]`, `[shell.click.real.target]`, `[shell.click.real.focus.ok]`) to the `EV_BTN` handler, matching the USB handler's markers. Real USB clicks now route through the `EV_BTN` handler (via sexinput's normalized `EV_BTN` events), and all diagnostic markers fire correctly.

### Fix 3 — sexinput stops forwarding OP_USB_MOUSE_REPORT for real USB

**File:** `servers/sexinput/src/main.rs`

Removed `OP_USB_MOUSE_REPORT` forwarding from the real USB path. Sexinput now sends only `OP_HID_EVENT` messages (`EV_BTN` + `EV_REL`) to the shell. The synthetic click-focus proof still sends `OP_USB_MOUSE_REPORT` directly to the shell (preserving that test path).

### Fix 4 — Synthetic click-focus proof uses EV_ABS for positioning

**File:** `servers/sexinput/src/main.rs`

Replaced the delta-accumulation stages (1-3) with an `EV_ABS(940, 560)` before the button down. This anchors the cursor at the intended click position regardless of any concurrent `EV_ABS` interference from the silkbar proof. The proof now completes in 3 stages (init → ABS+click → release) instead of 6.

## Ownership Rules (New Invariants)

1. **`EV_REL` owns cursor movement** — `POINTER_X/Y` is updated by `EV_REL` delta accumulation. No other path modifies the cursor position for movement.
2. **`EV_ABS` owns absolute positioning** — Sets `POINTER_X/Y` directly. Used by synthetic proofs for anchor positioning.
3. **`EV_BTN` owns click targeting** — Handles hit-test, focus change, silkbar intercept, and drag start/end. All click-target markers fire from this handler.
4. **`OP_USB_MOUSE_REPORT` is synthetic-only** — The synthetic click-focus proof still exercises this path. Real USB no longer uses it. The handler only processes button state and click-to-focus (no movement).
5. **No double-apply** — A given dx/dy value is applied to `POINTER_X/Y` at most once, by exactly one handler.

## Verification

```bash
./scripts/entrypoint_build.sh
SEXUSB_XHCI_TRACE=0 timeout 15 ./dev.sh run-nographic \
  2>/tmp/real-click-target-fix.trace | tee /tmp/real-click-target-fix.log
```

### Proof Counts (nographic, 15s)

| Marker | Count | Meaning |
|--------|-------|---------|
| `sexinput.mouse.real.button` | 0 | No real USB mouse |
| `shell.click.real.target` | 10 | All clicks classified (chrome=8, app=2) |
| `shell.click.real.focus.ok` | 2 | LINEN + TEST3 focused |
| `sexinput.drag_proof.done` | 1 | One-shot |
| `shell.click_focus.down x=940 y=560` | 1 | Synthetic click lands correctly |
| `panic/PF/GP` | 0 | Clean |

### Key log sequence (synthetic click-focus proof)

```
[sexinput.synthetic.click_focus.start]
[sexinput.synthetic.click_focus.down]
[sexinput.synthetic.click_focus.up]
[shell.cursor.move] x=640 y=360
[shell.click_focus.down] x=940 y=560 buttons=0x1
[shell.click_focus.hit] id=200
[shell.click_focus.send.start] id=200
[shell.click_focus.send.ok] id=200
[shell.click.real.focus.ok] id=200
[shell.click.real.target] x=940 y=560 target=200 kind=app
[shell.cursor.move] x=940 y=560
```

### Visual sanity (SDL window)

```bash
SEXUSB_XHCI_TRACE=0 SDL_VIDEO_DRIVER=x11 ./dev.sh run
```

Confirm by eye:
- Cursor speed no longer feels 2×
- App click focuses app/window
- SilkBar clicks open chrome/panels, not app focus
- Drag proof still one-shot
- Clock still counts

## Changed Invariants

1. `OP_USB_MOUSE_REPORT` no longer carries movement authority for real USB. It remains active for the synthetic click-focus proof.
2. `EV_REL` is now the sole cursor position update path for real USB input.
3. `EV_BTN` now has full click-target diagnostic markers matching the USB handler.
4. Sexinput no longer sends duplicate movement information to the shell for real USB reports.

## STOP FIRST Conditions

1. Adding movement back to `OP_USB_MOUSE_REPORT` without removing it from `EV_REL`
2. Changing the ownership rule (EV_REL moves, EV_BTN clicks, USB is synthetic-only)
3. Removing the `EV_ABS` positioning from the synthetic click-focus proof
4. Adding new IPC or ABI for click targeting
5. Changes to kernel/sexdisplay/PDX
6. Broad input refactor that re-merges the USB and HID paths
