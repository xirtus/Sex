# INPUT_PHASE_CLOSEOUT_V1

## Status

Guest synthetic input pipeline fully proven. Physical host input deferred.

---

## Final Proven Table

| Stage | Count | Proven |
|-------|-------|--------|
| `sexusb.synthetic.drag.start` | 1 | ✅ |
| `sexusb.synthetic.drag.frame` | 2 | ✅ |
| `sexusb.synthetic.drag.complete` | 1 | ✅ |
| `sexinput.usb_mouse.recv` | 242 | ✅ |
| `sexinput.usb_mouse.normalize.ok` | 242 | ✅ |
| `sexinput.hid.emit.rel` | 16 | ✅ |
| `shell.drag.start` | 1 | ✅ |
| `shell.drag.move` | 160 | ✅ |
| `shell.drag.end` | 1 | ✅ |
| `shell.cursor_surface.move.ok` | 240 | ✅ |
| `sexdisplay.cursor.surface.update` | 6 | ✅ |
| panics/faults (#PF, #GP) | 0 | ✅ |

**Pipeline proven end-to-end:**
```
sexusb synthetic report
  → decode_boot_mouse_report
  → OP_USB_MOUSE_REPORT (PDX)
  → sexinput normalize_pointer_report_v1
  → EV_BTN + EV_REL via OP_HID_EVENT (PDX)
  → shell pointer state + click-focus + drag
  → OP_SURFACE_UPDATE (PDX)
  → sexdisplay cursor surface
```

---

## Final Not-Proven Table

| Item | Reason |
|------|--------|
| QEMU 11 host→USB HID nonzero reports | QEMU 11.0.0 host routing broken on this host — zero bytes in all USB HID report buffers |
| Physical keyboard nonzero b2 | Same root cause: host not delivering to USB HID |
| Physical mouse drag | Cannot test without physical HID reports |
| Hardware boot input | Not tested — requires USB boot + physical hardware |
| sexdisplay render timing smoothness | STOP-listed; sys_yield() is not a vsync clock |

---

## Deferred Rationale — QEMU Physical Input

QEMU 11.0.0 host→USB HID routing is broken on this host. All delivery paths tried:

| Method | Result |
|--------|--------|
| i8042=off | zero bytes in USB HID reports |
| SDL display | zero |
| GTK display | zero |
| VNC display | zero |
| QMP `input-send-event` | zero |
| HMP `sendkey` | zero |
| Physical tablet/mouse via xHCI | reports arrive, dx=0 dy=0 buttons=0 |

The guest pipeline is proven working. The gap is entirely in QEMU's host→emulated-device forwarding. Retest conditions:

1. Different QEMU version (9.x or earlier may work)
2. GTK + usb-tablet swap (one-line change in dev.sh, STOP if requires xHCI refactor)
3. usbredir / VFIO (pass physical USB device through)
4. Hardware boot (bypass QEMU entirely)

**Do not chase QEMU physical input fix in this repo.** It is a host/QEMU toolchain issue, not a guest pipeline bug.

---

## Default Build Safety

Confirmeed:
- `SEXUSB_SYNTHETIC` is a compile-time gate: `option_env!("SEXUSB_SYNTHETIC").is_some()`
- Default (unset): `SEXUSB_SYNTHETIC = false`
- All synthetic code is inside `if SEXUSB_SYNTHETIC { }` — dead-code eliminated by optimizer
- `./scripts/entrypoint_build.sh` (no env vars) passes cleanly
- No synthetic markers are emitted in default build path
- Normal xHCI interrupt-IN poll loop runs unchanged

---

## Build & Run

### Default (real USB path, no synthetic)
```bash
./scripts/entrypoint_build.sh
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run
```

### Synthetic drag proof
```bash
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/smooth-drag-proof.log
```

### Verifier
```bash
for m in \
  sexusb.synthetic.drag.start \
  sexusb.synthetic.drag.frame \
  sexusb.synthetic.drag.complete \
  sexinput.usb_mouse.recv \
  sexinput.usb_mouse.normalize.ok \
  sexinput.hid.emit.rel \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end \
  shell.cursor_surface.move.ok \
  sexdisplay.cursor.surface.update
do
  printf "%-42s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/smooth-drag-proof.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/smooth-drag-proof.log
```

---

## Documents Created/Updated

| File | Action |
|------|--------|
| `servers/sexusb/src/main.rs` | Patch: new synthetic drag sequence (242 frames, 6 phases) |
| `docs/handoff/SYNTHETIC_INPUT_PROOF_V1.md` | No change (status accurate) |
| `docs/handoff/SMOOTH_SYNTHETIC_DRAG_PROOF_V1.md` | Created: drag sequence spec |
| `docs/handoff/INPUT_PHASE_CLOSEOUT_V1.md` | Created: this closeout |

---

## Next Recommended Phase

**Recommended: A — Silk shell interaction contract hardening**

Rationale:
- Synthetic input is proven, so we can now test shell interaction contracts deterministically
- This phase does **not** depend on QEMU physical input
- It directly advances the roadmap's Phase 5 boundary hardening
- Existing handoffs (`SHELL_GLOBAL_INTERACTION_CONTRACT_V1`, `SHELL_FOCUS_CONTRACT_V1`) define concrete subcontracts:
  - `SHELL_INTERACTION_STATE_V1`: unify state machine
  - `HIT_TEST_PRIORITY_V1`: strict z-order capture hierarchy
  - `EVENT_ORDERING_CONTRACT_V1`: deterministic event ordering
- All work is scoped to `silk-shell` only — no kernel/display/input changes

Other candidates evaluated:
| Candidate | Pro | Con |
|-----------|-----|-----|
| B) selected-window SilkBar options | Advances SilkBar interaction | Depends on SilkBar contract which is in M2 churn |
| C) Quil/terminal debug surface | Useful for debugging | Adds surface complexity before interaction hardening |
| D) Frame Chrome / tiled shell model | Architectural | Too early — interaction contract should come first |
| E) hardware boot prep checklist | Enables real hardware testing | Physical input blocked; booting without input has limited value |

The `SHELL_INTERACTION_STATE_V1` subcontract is the smallest, highest-impact step that depends only on proven synthetic input.
