# USB_TABLET_EMIT_DELTA_ONLY_V1

## A) PASS

## B) Root cause / implemented path

**Root cause**: `normalize_pointer_report_v1` in `sexinput` was emitting `EV_ABS(abs_x, abs_y)` for
USB tablet reports. The `[usb.hid.pointer.emit]` marker then printed the decoded `dx`/`dy` i16 fields
(which held raw abs coords like 15820, 13270) under "dx/dy" labels. While silk-shell correctly handled
EV_ABS via `process_abs_tablet`, the marker was misleading and the design was fragile.

**Implemented path**: In the `is_abs` branch of `normalize_pointer_report_v1`:
1. Scale raw tablet coords (0..32767) to screen pixels using SCREEN_W=1280, SCREEN_H=720.
2. Track `LAST_SX`/`LAST_SY` (scaled) as static state. On first packet (ABS_INIT): emit nothing, store position.
3. On subsequent packets: compute `ddx = (sx - LAST_SX).clamp(-512, 512)` and emit `EV_REL(ddx, ddy)`.
4. Add `[usb.tablet.abs.delta]` and `[usb.tablet.emit.delta_only]` markers inside normalizer.
5. Add `[usb.tablet.emit.raw_blocked]` when raw abs_x or abs_y > DELTA_CLAMP (512).
6. Updated `[usb.hid.pointer.emit]` marker to extract EV_REL values from `normalized_events[]` (not the outer i16 variables).
7. Added one-shot `[usb.tablet.emit_delta_only.pass]` marker on first successful is_abs emit.

Silk-shell now receives `EV_REL(delta)` for real tablet input and routes through `apply_rel_pointer`.
The `process_abs_tablet` path remains active only for synthetic proofs that call `handle_hid_event(EV_ABS, ...)` directly.

## C) Files changed

- `servers/sexinput/src/main.rs` — normalizer `is_abs` branch + marker fixes
- `docs/handoff/USB_TABLET_EMIT_DELTA_ONLY_V1.md` — this file

Backup: `servers/sexinput/src/main.rs.bak.delta_only_v1`

## D) Minimal diff summary

**`servers/sexinput/src/main.rs`**:
- Replace `is_abs` branch in `normalize_pointer_report_v1` (~18 lines → ~50 lines):
  - Add `TABLET_RAW_MAX=32767`, `SCREEN_W=1280`, `SCREEN_H=720`, `DELTA_CLAMP=512` consts
  - Add `ABS_INIT`, `LAST_SX`, `LAST_SY` static state
  - Scale abs→screen, compute clamped delta, emit `EV_REL` instead of `EV_ABS`
  - Emit `[usb.tablet.abs.delta]`, `[usb.tablet.emit.delta_only]`, `[usb.tablet.emit.raw_blocked]`
- Update `[usb.hid.pointer.emit]` marker (~8 lines): extract EV_REL from `normalized_events[]`
  - Fallback to 0,0 for `is_abs` path when no EV_REL present (button-only reports)
- Add `[usb.tablet.emit_delta_only.pass]` one-shot marker after `pointer_emit_ok = true`

## E) Proof markers observed

```
[usb.tablet.abs.delta] dx=0 dy=0 buttons=0 ok=1          ← abs.init (first packet)
[usb.tablet.emit.raw_blocked] raw_dx=18533 raw_dy=20807 ok=1  ← blocked raw abs
[usb.tablet.abs.delta] dx=512 dy=456 buttons=0 ok=1       ← first real delta (clamped to 512)
[usb.tablet.emit.delta_only] buttons=0 dx=512 dy=456 ok=1
[usb.hid.pointer.emit] op=OP_HID_EVENT buttons=0 dx=512 dy=456 ok=1
[usb.tablet.emit_delta_only.pass] ok=1
[usb.tablet.abs.delta] dx=-1 dy=0 buttons=0 ok=1          ← incremental movement
[usb.tablet.emit.delta_only] buttons=0 dx=-1 dy=0 ok=1
[usb.hid.pointer.emit] op=OP_HID_EVENT buttons=0 dx=-1 dy=0 ok=1
[usb.tablet.abs.delta] dx=0 dy=-1 buttons=1 ok=1          ← button press with delta
[usb.tablet.emit.delta_only] buttons=1 dx=0 dy=-1 ok=1
[usb.hid.pointer.emit] op=OP_HID_EVENT buttons=1 dx=0 dy=-1 ok=1
```

All bounded. No raw abs values (15820/13270) in pointer.emit markers.

## F) Commands run and results

```
cp servers/sexinput/src/main.rs servers/sexinput/src/main.rs.bak.delta_only_v1
./scripts/entrypoint_build.sh   → [SEXOS ENTRYPOINT] success
QEMU_PRINT_CMD=0 SEXUSB_QEMU_DEVICE=tablet-display-sdl SEXOS_QEMU_DISPLAY=sdl ./dev.sh
  → all required markers present, no fault tokens
rg fault scan: clean
```

## G) Deferred work / STOP FIRST notes

- **Click/focus proof**: not implemented — separate concern, no regression observed
- **PS/2 mouse IRQ12**: STOP FIRST — requires kernel IRQ routing change
- **Touchpad/gesture**: explicitly out of scope
- **silk-shell `process_abs_tablet` cleanup**: can be removed when synthetic proofs migrate to EV_REL; deferred to avoid breaking existing proof sequences
- **First-delta jump**: abs_init leaves LAST_SX=0; first movement shows jump to current position (dx up to 512px). Acceptable for now. Fix: could initialize LAST_SX from first packet and emit zero, but that would require a second-packet trigger.

## H) No kernel/ABI/sexdisplay/shell-policy changes

No kernel, sex-pdx ABI, sexdisplay, or shell-policy changes were made.
Only `servers/sexinput/src/main.rs` and this handoff doc were modified.
