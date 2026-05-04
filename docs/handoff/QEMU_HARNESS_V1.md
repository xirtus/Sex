# QEMU_HARNESS_V1

**Status:** Active  
**Purpose:** Canonical QEMU launch path for SexOS development.  
**Scope:** Host-side scripts only. No kernel/servers/ABI changes.

---

## Canonical QEMU Command

```
qemu-system-x86_64 \
    -M q35 \
    -m 512M \
    -cpu max,+pku \
    -cdrom sexos-v1.0.0.iso \
    -device nec-usb-xhci,id=xhci \
    -device usb-tablet,bus=xhci.0 \
    -serial stdio \
    -display none
```

This is the **only** launch configuration validated for USB/input development.
Any deviation (different USB controller, PS/2 mouse, missing PKU flag, wrong
machine type) invalidates test results.

---

## Using the Harness

### Build + Run (60-second smoke test)

```bash
./scripts/entrypoint_build.sh
./scripts/qemu_harness.sh --timeout 60 || true
```

### Extract markers from the log

```bash
./scripts/qemu_markers.sh
# or
./scripts/qemu_harness.sh --markers  # re-reads the latest log
```

### Print the canonical command

```bash
./scripts/qemu_harness.sh --print-cmd
```

### Run with display (for visual debugging)

```bash
SEXOS_QEMU_DISPLAY=sdl ./scripts/qemu_harness.sh --timeout 120
```

### Run with USB keyboard

```bash
SEXUSB_QEMU_DEVICE=kbd ./scripts/qemu_harness.sh --timeout 60
```

---

## Log Paths

| Log | Path | Description |
|-----|------|-------------|
| Canonical harness log | `logs/qemu-latest.log` | Always overwritten on each `qemu_harness.sh` run |
| Old debug log | `qemu_debug.log` | Legacy, from `run_qemu.sh` (may be stale) |

The canonical log captures **only** serial output (`-serial stdio`). QEMU stderr
(machine warnings, device errors) is also included since the harness redirects
both stdout and stderr.

---

## Why Old Launch Configs Failed

### Wrong USB device model

Old scripts used `usb-mouse` (boot-protocol HID, relative coordinates) instead
of `usb-tablet` (absolute coordinates). The XHCI driver in the kernel may only
enumerate/forward absolute HID devices.

| Device | Protocol | Coordinates | Status |
|--------|----------|-------------|--------|
| `usb-mouse` | Boot HID | Relative | ❌ May not enumerate |
| `usb-tablet` | Report HID | Absolute | ✅ Canonical |
| `usb-kbd` | Boot/Report | N/A | ✅ Supported |

### Missing or wrong PKU flag

Without `+pku`, `WRPKRU` instructions fault and memory isolation (PKEYs)
silently breaks. The kernel hangs or panics during init.

### Wrong machine type

`-M pc` (default) lacks Q35's XHCI routing. The `nec-usb-xhci` device may not
work correctly. `-M q35` is mandatory.

### Custom QEMU binary vs system QEMU

`run_qemu.sh` used a custom QEMU binary at
`tools/qemu/build/qemu-system-x86_64`. The canonical harness uses the system
`qemu-system-x86_64` (or `QEMU_BIN` override). Both work if the custom binary
is newer; the system binary is the canonical reference.

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SEXOS_QEMU_DISPLAY` | `none` | Display mode: `none`, `sdl`, `gtk` |
| `SEXUSB_QEMU_DEVICE` | `tablet` | USB device: `tablet`, `mouse`, `kbd` |
| `QEMU_BIN` | `qemu-system-x86_64` | QEMU binary path override |
| `SEXOS_QEMU_NODEFAULTS` | unset | Set to `1` to disable PS/2 defaults |
| `SEXOS_QEMU_I8042` | unset | Set to `off` to disable i8042/PS2 controller |
| `SEXOS_QEMU_QMP` | unset | Set to `1` to enable QMP socket |
| `SEXUSB_XHCI_TRACE` | unset | Set to `1` to enable XHCI trace events |

---

## Marker Reference

Expected markers from a healthy boot with USB/input:

```
[usb.host.controller.found]          — XHCI controller discovered by DevMgr
[sexusb.forward.mouse]               — USB mouse events forwarded to input server
[shell.ready]                        — Silk shell initialized
[chrome.template.swap]              — Window chrome template active
```

Marker extraction uses `scripts/qemu_markers.sh` or `--markers` flag.

---

## Verification Checklist

After `./scripts/qemu_harness.sh --timeout 60`, check:

- [ ] QEMU exits with code 0 (clean exit) or 124 (timeout — expected for smoke test)
- [ ] Log contains `[usb.host.controller.found]` or equivalent XHCI discovery
- [ ] Log contains `[sexusb.forward.mouse]` or equivalent HID input markers
- [ ] Log does NOT contain `panic`, `ERROR`, or `FAIL`
- [ ] `logs/qemu-latest.log` is non-empty and contains serial output

---

## Next Phase

After harness is verified, resume:

**USB_HID_POINTER_PROOF_V1** — Prove that USB tablet → XHCI → sexusb → sexinput
produces real cursor movement in the window manager.

Prerequisite: QEMU_HARNESS_V1 produces reliable USB/XHCI markers.

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Initial canonical harness | QEMU_HARNESS_V1 |
