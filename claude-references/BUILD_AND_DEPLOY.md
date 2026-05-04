# Build & Deploy Reference

> Referenced from CLAUDE.md (offloaded reference).
> See CREW.md for canonical team policy.

---

## Build Pipeline

```bash
./build_payload.sh && make iso && make run-sasos
```

QEMU flags: `-M q35 -m 512M -cpu max,+pku -cdrom sexos-v1.0.0.iso -serial stdio`

### QEMU Environment Variables

| Variable | Purpose |
|----------|---------|
| `QEMU_BIN` | Override QEMU binary path for testing different versions |
| `QEMU_PRINT_CMD=1` | Print exact QEMU argv, no launch |
| `SEXUSB_QEMU_DEVICE=tablet` | Use usb-tablet instead of usb-mouse |
| `SEXUSB_QEMU_DEVICE=tablet-display-sdl` | Add `display=sdl` to usb-tablet |
| `SEXUSB_QEMU_DEVICE=kbd` | Use usb-kbd instead of mouse |
| `SEXOS_QEMU_NODEFAULTS=1` | Add `-nodefaults` (disables PS/2 input) |
| `SEXOS_QEMU_DISPLAY=none` | Headless mode |
| `SEXOS_PROOFS_DISABLED` | Set env var at build time to disable synthetic proofs for interactive use |
| `SEXOS_KEYBOARD_CURSOR=1` | Enable arrow/WASD → EV_REL cursor movement (8px step) |
| `SDL_VIDEO_DRIVER=x11` | Force SDL X11 backend (required for window discovery via xdotool) |

### Proof Verification

```bash
grep "silk.render_proof" /tmp/silk-render-proof.log          # Top-strip hash
grep "shell.silkbar.click" /tmp/silkbar-click.log            # SilkBar click targets
grep -E "shell.drag.start|shell.drag.move|shell.drag.end" /tmp/drag-proof.log  # Drag proof
grep "shell.click_focus" /tmp/click-focus-proof.log          # Click-focus proof
```

---

## Workspace Layout

```
kernel/          — sex-kernel crate (ring 0)
servers/
  sexdisplay/    — framebuffer/compositor server (PDX)
  silk-shell/    — shell server (PDX)
  silkbar/       — top bar server (PDX)
  sexinput/      — HID input server (PDX)
  sexusb/        — USB/xHCI driver (PDX)
apps/
  linen/         — first userland app (PDX)
crates/
  sex-pdx/       — shared PDX calling convention crate
  silkbar-model/ — shared SilkBar types and logic
```

**Cargo resolver:** workspace uses resolver = "2".

---

## Workspace Cargo Warnings (expected, non-fatal)

These warnings appear on every build and are harmless:
- "profiles for the non root package will be ignored" (silk-shell, sexinput, silkbar)
- `lib.no_std` unused manifest key in `sex-pdx/Cargo.toml`

Do not attempt to fix these without understanding the full workspace layout.
