# USB_HOST_DISCOVERY_V1

A) RESULT: DISCOVERY COMPLETE
- No code changes. Docs-only audit of current USB/XHCI reality.
- Proof scan included for completeness; USB lanes are out of scope for this discovery pass.

B) REPO USB/XHCI REALITY

**sexusb server** (`servers/sexusb/src/main.rs`, 4535 lines):
- Fully implemented XHCI host controller driver for QEMU `nec-usb-xhci`.
- Full init: BAR0 map (syscall 43 MAP_PCI_BAR), CAPLENGTH/HCIVERSION/HCSP1/HCSP2/HCC1 probe,
  stop/reset (HCRST), ring memory alloc (cmd ring 1 page, event ring 1 page, DCBAA 1 page,
  ERST 1 segment), DCBAAP/CRCR/ERSTBA/ERSTSZ/ERDP register programming,
  CONFIG.MaxSlotsEnabled write, Run/Stop.
- Full command ring: NOOP (type=23), Enable Slot (type=9), Address Device (type=11),
  Evaluate Context (type=13). Cycle-bit semantics per XHCI spec §5.4.5 (CRCS toggles on
  segment boundary only, not per TRB). QEMU nec-xhci quirks: lower-dword-first writes
  for CRCR/DCBAAP/ERSTBA/ERDP.
- Full EP0 control: GET_DESCRIPTOR(DEVICE, 8 bytes), Evaluate Context for MPS,
  GET_DESCRIPTOR(DEVICE, 18 bytes), GET_DESCRIPTOR(CONFIGURATION, header+full),
  GET_DESCRIPTOR(HID_REPORT), SET_CONFIGURATION.
- Full descriptor walk: INTERFACE/HID/ENDPOINT parsing, detects HID boot mouse
  (class=0x03, subclass=0x01, protocol=0x02) and stores interrupt IN endpoint
  (addr, MPS, interval, DCI).
- HID report descriptor shape scan: looks for mouse shape (05 01, 09 02, A1 01,
  09 30, 09 31) and tablet shape (05 01, 09 01, 09 30, 09 31).
- Interrupt endpoint: Configure Endpoint command, Normal TRB arm (IOC=1),
  doorbell DB[slot_id][DCI], bounded poll for Transfer Event (type=32).
- Report decode: `decode_boot_mouse_report` (3+ bytes) and `decode_tablet_report` (5 bytes).
- PDX route: `send_report_to_sexinput(OP_USB_MOUSE_REPORT, 0, buttons, packed_axes)`
  to slot 9 (SLOT_USB_SEXINPUT).
- Synthesis: `SEXUSB_SYNTHETIC = false` (forced off); synthetic mouse frame helper exists.
- Single-device limitation: `SingleHidBind` holds one device; port scan breaks on first
  CCS=1. Multi-device requires multi-slot + per-device ring state (see
  SEXUSB_SINGLE_DEVICE_GUARD_V1.md).

**sexinput USB handler** (`servers/sexinput/src/main.rs`):
- Receives `OP_USB_MOUSE_REPORT = 0x260` via `pdx_try_listen_raw(0)`.
- Decodes packed report (buttons, dx, dy, wheel, is_abs) from arg1/arg2.
- Calls `normalize_pointer_report_v1` — emits EV_REL/EV_ABS + EV_BTN edges.
- Forwards normalized `OP_HID_EVENT = 0x202` tuples to silk-shell.
- USB keyboard report path (OP_USB_KEYBOARD_REPORT = 0x261) is defined but
  implementation is skeletal (map USB HID usage→PS/2 scancode, then EV_KEY route).

**silk-shell USB handler** (`servers/silk-shell/src/main.rs`):
- Receives `OP_USB_MOUSE_REPORT = 0x260` via main listen loop.
- Maintains local USB pointer state (x, y, buttons, wheel_accum).
- Nonzero movement proof markers fire when USB dx/dy is nonzero.
- Full click-focus/drag path can consume USB pointer events.

C) PCI / MMIO / IRQ / DMA CAPABILITY REALITY

**PCI enumeration** (`kernel/src/devmgr.rs`):
- Discovers XHCI controller by PCI class code: class=0x0c, subclass=0x03, prog-if=0x30.
- Reads BAR0 base address, IRQ line from PCI config space.
- Grants PCI Capability at `SLOT_USB_HOST = 8` (sex-pdx constant) to sexusb PD.
- Grants Interrupt Capability with irq_line from PCI config.
- IRQ route NOT registered for sexusb (sexusb uses polling, not interrupts).

**MMIO mapping** (`kernel/src/syscalls/mod.rs`, syscall 43):
- `MAP_PCI_BAR(cap_slot, bar_index, map_size)` → maps BAR0 as uncacheable (PCD=1, PWT=1).
- sexusb maps 64KB (0x10000) — QEMU nec-usb-xhci BAR0 span.
- Verified BAR size not capped to 4KB.

**DMA / physical memory** (syscalls 30, 31):
- syscall 31 `ALLOCATE_MEMORY` — allocates physical pages from kernel allocator.
- syscall 30 `MAP_MEMORY` — maps physical pages to the caller's VA space.
- sexusb uses these for: command ring page, event ring page, DCBAA page, EP0 transfer
  ring page, interrupt transfer ring page, descriptor data buffer, input/output device
  context pages.
- No lent memory (syscall 51) used in current sexusb.

D) EXISTING INPUT BRIDGE READINESS

| Component | Status | Notes |
|-----------|--------|-------|
| sexusb→sexinput route | READY | Kernel grants domain cap at slot 9. Route proven. |
| OP_USB_MOUSE_REPORT (0x260) | READY | Defined in both sexusb and sexinput. |
| normalize_pointer_report_v1 | READY | In sexinput. Consumes HidPointerRawReport, emits EV_REL/EV_ABS/EV_BTN. |
| Click-focus via USB | READY | silk-shell processes USB→normalized HID events through handle_hid_event. |
| Keyboard HID→PS/2 bridge | SKELETAL | OP_USB_KEYBOARD_REPORT defined (0x261) but decoder not fully wired. |
| USB tablet abs→rel bridge | READY | sexusb tracks prev position, computes clamped delta for sexinput. |

E) QEMU / BUILD / PROOF REALITY

**QEMU args** (`dev.sh`):
- Controller: `-device nec-usb-xhci,id=xhci`
- Device: controlled by `SEXUSB_QEMU_DEVICE` env var:
  - `mouse` (default): `-device usb-mouse,bus=xhci.0`
  - `tablet`: `-device usb-tablet,bus=xhci.0`
  - `kbd`: `-device usb-kbd,bus=xhci.0`
  - `kbd+tablet`: both devices
- Optional: `SEXUSB_XHCI_TRACE=1` enables QEMU xHCI trace events
- Optional: `SEXOS_QEMU_I8042=off` disables PS/2 controller (USB-only input)

**Proof runner** (`scripts/run_daily_driver_proof.sh`):
- QEMU: `-device nec-usb-xhci,id=xhci -device usb-kbd,bus=xhci.0`
- No `SEXUSB_QEMU_DEVICE` override — hardcoded to usb-kbd.
- No USB-specific proof env vars exported beyond the standard set.
- sexusb is built and spawned as part of the standard server set.

**Build**:
- sexusb in `servers/sexusb/Cargo.toml`, listed in root `Cargo.toml` workspace.
- Builds with `cargo build -p sexusb` (or via entrypoint_build.sh).
- Zero build warnings for sexusb.

F) BLOCKERS FOR USB_XHCI_MINIMAL_ENUM_V1

NOTE: "minimal enum" (NOOP → Enable Slot → Address Device → descriptors) is ALREADY
IMPLEMENTED in sexusb. All these stages complete with `.ok` markers. The actual blockers
for functional USB HID input are:

1. **Keyboard HID report descriptor not recognized**:
   - sexusb HID shape scan only matches mouse (09 02) or tablet (09 01).
   - QEMU usb-kbd produces keyboard descriptors (usage page 0x07, usage 0x06).
   - Shape scan returns `mouse=false tablet=false` → `shape.warn`.
   - Need keyboard shape detection (05 01 09 06 A1 01 05 07).

2. **Interrupt IN polling timeout**:
   - After endpoint configure + Normal TRB arm + doorbell, polls for Transfer Event.
   - All polls return timeout (phase=RING, ok=0).
   - QEMU usb-kbd is idle (no keypresses); SET_IDLE may not apply to keyboard.
   - Possible issues: doorbell target wrong, cycle bit on transfer ring, or
     QEMU usb-kbd needs an initial interrupt OUT report (LED state) before
     sending IN reports.

3. **Keyboard report decode path incomplete**:
   - `decode_boot_mouse_report` and `decode_tablet_report` exist but no
     `decode_boot_keyboard_report`.
   - OP_USB_KEYBOARD_REPORT (0x261) defined but decoder not implemented.
   - USB HID usage→PS/2 scancode translation table needed.

4. **Single-device limitation**:
   - Only one HID device per boot. Can't have mouse+keyboard simultaneously.
   - Port scan breaks on first CCS=1; needs multi-slot allocation.

5. **IRQ not used**:
   - Interrupt capability granted but sexusb uses polling only.
   - Polling wastes scheduler budget and has latency.
   - Future phases should register IRQ handler.

G) STOP FIRST ITEMS

Any of these trigger STOP FIRST before implementation:
1. Kernel edits (syscalls, ABI, scheduler, memory manager)
2. sex-pdx ABI edits (opcodes, slot constants, type IDs)
3. sexdisplay changes (framebuffer, compositor, render tokens)
4. silk-shell input policy redesign (focus, drag, click, route)
5. USB/XHCI changes that touch sexinput normalizer contract
6. New ABI opcodes beyond existing OP_USB_MOUSE_REPORT/OP_USB_KEYBOARD_REPORT
7. Multi-device support (requires ring state/alloc redesign)
8. IRQ handler registration path

H) PROOF COMMAND / LOG PATH

Proof run (NOT required for discovery; included for completeness):
- Command: `./scripts/run_daily_driver_proof.sh /tmp/usb_host_discovery_v1.log`
- Log: `/tmp/input_route_negative_tests_v4.log` (reused from AP10 — USB irrelevant to AP10 gate pass)
- Not rerun for this discovery phase; USB markers were captured from existing AP10 log.

Sexusb lifecycle summary from proof log:
- 1124 sexusb markers total.
- All init stages PASS: map, probe, reset, rings, DCBAAP/CRCR, Run/Stop,
  NOOP, Enable Slot (slot=1), Address Device (state=2), descriptor 8,
  Evaluate Context MPS, descriptor 18 (12 01 00 02 00 00 00 40...),
  config header+full, HID report descriptor (63 bytes),
  SET_CONFIGURATION, Configure Endpoint (interrupt IN).
- Shape scan: `mouse=false tablet=false` (keyboard descriptor).
- Continuous poll: timeout loop — no keyboard transfer events.
- Zero faults, zero panics, zero #PF/#GP in sexusb path.

I) FAULT SCAN (from AP10 log)

- `#PF`: 0
- `#GP`: 0
- `panic`: 0
- `fault.kill`: 0
- `null-jump`: 0
- `IPC storm`: 0
- `ring overflow`: 0
- `sexusb.*.bad`: 0 (all init stages ok)
- `sexusb.bootgraph.proof_report` present (sexusb→sexinput route proven)

J) FILES CHANGED
- `docs/handoff/USB_HOST_DISCOVERY_V1.md` (new)

K) NEXT REQUIRED AUTOPILOT

The "minimal enum" phase already exists in the codebase. The next real implementation
phase should be:

**USB_HID_KEYBOARD_REPORT_V1** — prove one USB HID keyboard report reaches silk-shell:
1. Add keyboard shape detection to HID report descriptor scan (usage page 0x07).
2. Implement `decode_boot_keyboard_report` (8-byte boot protocol keyboard report).
3. Fix interrupt IN polling timeout for QEMU usb-kbd.
4. Wire keyboard report through OP_USB_KEYBOARD_REPORT → sexinput → silk-shell EV_KEY path.
5. Prove one keypress reaches Quil/Linen text surface.

Scope: sexusb + sexinput + silk-shell. No kernel/ABI/sexdisplay edits.
Preserves single-device limitation.
