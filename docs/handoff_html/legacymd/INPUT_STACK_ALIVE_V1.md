# INPUT_STACK_ALIVE_V1

Status: PASS

## Proven
- Serial log capture works via QEMU `-serial file:/tmp/sexos-input.log`.
- QMP input injection works.
- USB/tablet pointer events reach sexinput.
- Pointer events forward from sexinput to silk-shell.
- Cursor path updates from pointer events.
- PS/2 keyboard IRQ1 now fires.
- Keyboard events reach silk-shell.

## Track B fixes
- Pointer log budgets raised.
- Tablet absolute coordinates preserved instead of crushed into i8 deltas.
- Motion-only pointer events now forward.
- silk-shell cursor EV_ABS path triggers visible update/redraw.

## Track A fixes
- IOAPIC IOWIN offset fixed to `0x10 / 4`.
- PIC masked to avoid APIC conflict.
- `keyboard::init()` implemented.
- PS/2 keyboard scanning enabled.
- Keyboard init called during kernel boot.

## Validation
Active input probe passed:
- Pointer → silk-shell: PASS
- Keyboard → silk-shell: PASS

## Next phase
Move from input plumbing to interactive shell behavior:
1. Cursor visual movement persistence.
2. Click/focus ownership.
3. Basic window target selection.
4. Key event routing to focused surface.
5. Minimal interactive demo window.