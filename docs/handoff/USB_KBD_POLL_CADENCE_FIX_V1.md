# USB_KBD_POLL_CADENCE_FIX_V1

## Root Cause

Short USB keyboard keytaps were missed because the sexusb keyboard polling loop
had a gap: the interrupt-IN TRB was only re-queued after the `pdx_call_checked()`
to sexinput completed. Since that call is a **synchronous IPC** that blocks sexusb
until sexinput replies, **no TRB was pending on the xHCI during the IPC window**.

A USB interrupt poll arriving during IPC (or between the TRB completion and
re-queue) would find no TRB, and the key data would be lost.

## Files Changed

Only `servers/sexusb/src/main.rs` was modified.

## Changes

### 1. Re-arm TRB Before IPC (primary fix)

In the keyboard decode path, after reading the report bytes into local variables
but **before** calling `pdx_call_checked()` to sexinput:

- Advance `intr_prod` (same logic as the bottom-of-loop advancement)
- Clear the report buffer
- Write a Normal TRB (Interrupt-IN) at the new producer slot
- Ring the doorbell
- Set `skip_advance = true` so the bottom-of-loop does not double-advance

This ensures an interrupt-IN TRB is always queued on the xHCI during the IPC,
closing the window where USB polls would be missed.

### 2. Post-IPC Event Spin (bonus)

After the IPC returns, a lightweight spin-loop (max ~3000 iterations with
`core::hint::spin_loop()`) checks the event ring for the re-arm TRB's completion.
If it completed during IPC (e.g., a key was pressed and the USB poll fired),
the report is read and forwarded immediately without waiting for the next
main-loop iteration.

### 3. `skip_advance` Flag

`let mut skip_advance = false;` at the top of each loop iteration. Set to `true`
in the keyboard re-arm path. Checked at the bottom-of-loop: if true, the
advancement code (which normally increments `intr_prod` and handles ring wrap)
is skipped, preventing double-advancement.

### 4. Bounded Keyboard Burst Poll

After the main IPC, a bounded burst loop does a spin-wait on the event ring
(no `sys_yield()`, just `core::hint::spin_loop()` with a timeout of 3000 iterations).
If the re-arm TRB completed during IPC, its data is processed and forwarded.
If not (timeout), the burst falls through silently and the main loop picks up
the pending TRB on the next iteration.

## Build Result

Full entrypoint build passed:
```
[SEXOS TRACE] deterministic sequence complete
[SEXOS ENTRYPOINT] success
```

## Manual Proof Markers

(From spindle_manual_verify.log, pre-fix baseline)
```
[sexusb.kbd.raw] b0=0x0 b2=0xb b3=0x0 actual=8    ← sustained key 'H' (0x0b)
[sexinput.kbd.recv] key=0xb mod=0x0                  ← received by sexinput
[sexusb.kbd.raw] b0=0x0 b2=0x8 b3=0x0 actual=8    ← key 0x08
[sexinput.kbd.recv] key=0x8 mod=0x0                  ← received
[sexusb.kbd.raw] b0=0x0 b2=0xf b3=0x0 actual=8    ← key 0x0f
[sexinput.kbd.recv] key=0xf mod=0x0                  ← received
```

Non-zero HID keycodes confirmed on sustained press. Short taps were the failure
mode (all-zero reports).

## Post-Fix Markers

New debug markers (budgeted, 128 each):
- `[sexusb.kbd.raw]` — raw keyboard report bytes (unchanged)
- `[sexusb.kbd.poll.burst]` — burst catch marker (new)
- `[sexusb.kbd.forward]` — non-zero forward marker (unchanged)

## Quick Tap Proof

QMP `input-send-event` injection was attempted but does not reach the `usb-kbd`
device when `-display none` is used (QMP events go to the input subsystem but
usb-kbd handler is not found with `con == NULL` in the second pass of
`qemu_input_find_handler`). Physical interaction with `-display sdl` is required
for definitive proof.

## Sustained Key Proof

The pre-fix baseline already proves the keyboard path works for sustained
keypresses. The fix does not change the fundamental path — only the timing
of TRB re-queue relative to IPC.

## Regressions

None observed:
- Tablet/mouse path unchanged (all changes gated by `is_keyboard_device`)
- No panic/#PF/#GP/fault.kill in any test log
- No new warnings from sexusb build
- All pre-existing static-mut warnings (18) unchanged

## Next Recommended Phase

1. **Physical interactive test** with `-display sdl`:
   - Run `./qemuX-kbd.sh -display sdl` (keyboard only)
   - Tap h/e/l/p quickly
   - Check for non-zero `[sexusb.kbd.poll.burst]` markers
   - Check for non-zero `[sexinput.kbd.recv]` markers
   
2. **Both-device test**: Restore `usb-tablet` alongside `usb-kbd` in qemuX.sh
   and verify both devices work without regression.

3. **QEMU probe cleanup**: Remove `[QEMU_KBD_DEBUG]` and `[QEMU_HID_DEBUG]`
   fprintf probes from `tools/qemu/hw/input/hid.c`.
