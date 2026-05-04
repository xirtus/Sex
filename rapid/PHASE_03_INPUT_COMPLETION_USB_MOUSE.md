# PHASE 03: Input Completion + USB Mouse

## Goal
Complete the input pipeline from physical device through kernel HID to shell actions. Two independent subpasses that can proceed in parallel.

## Ownership

**Phase 3A: Shell Input Policy** — silk-shell (exclusive)
**Phase 3B: USB Host + HID** — sexusb (exclusive) → sexinput (normalization) → silk-shell (consumer)

## What Already Exists

**Phase 3A (~60% done):**
- Synthetic mouse input works (SEXUSB_SYNTHETIC=1 sends OP_USB_MOUSE_REPORT, silk-shell receives it)
- Keyboard input works (scancode → SurfaceAction dispatch with 15+ bindings)
- Focus/drag FSM exists (ShellInteractionState, click_hit_test_and_focus(), rim drag markers)
- Click hit-test dispatch: FrameChrome → light/tab/rim dispatch works
- `selected_frame_id()` and `click_hit_test_and_focus()` wired

**Phase 3B (~5% done):**
- `sexusb` server exists, boots, listens on PDX slot
- USB XHCI controller init exists (docs mention XHCI)
- Synthetic proof infrastructure exists (SEXUSB_SYNTHETIC env var)
- `sexinput` server exists, forwards events to silk-shell
- No real USB mouse enumeration or HID report parsing

## Bundle 3A — Shell Input Policy

| Task | Detail | Effort | Priority |
|------|--------|--------|----------|
| Physical click proof (real mouse) | Button-down/up from USB reaches silk-shell as OP_HID_EVENT | 1h (after 3B) | High |
| Button-down/up normalization | Event ordering contract enforced (down before up, no duplicates) | 2h | High |
| Click edge guarantee | Click → hit-test → focus change → drag start is atomic in one dispatch cycle | 1h | High |
| Drag FSM hardening | Edge cases: drag off window, drag over light, drag + keyboard interrupt | 2h | Medium |
| Multi-monitor click (future) | Coordinate translation if >1 framebuffer | Defer | Low |

## Bundle 3B — USB HID Path

| Task | Detail | Effort | Priority |
|------|--------|--------|----------|
| USB_HOST_DISCOVERY_V1 | XHCI MMIO base, controller init, root hub ports | 8h | High |
| USB_XHCI_MINIMAL_ENUM_V1 | Device descriptor, configuration, interface descriptor for HID | 12h | High |
| USB_HID_BOOT_MOUSE_REPORT_V1 | Interrupt endpoint, HID report decode (buttons + x/y) | 6h | High |
| USB_HID_POINTER_PRODUCER_V1 | Normalize to `OP_HID_EVENT` type=EV_REL/EV_KEY → silk-shell | 2h | Medium |

## Smallest First Step (3B)
Enumerate the root hub and detect a device connect event. Before any HID parsing, before any endpoint configuration — just detect that a USB device was plugged in. This proves the XHCI MMIO mapping and interrupt delivery work. The remaining tasks all build on this foundation.

## Dependencies
- **Phase 3A blocks on**: Nothing (synthetic input already works; real click is optional until 3B delivers)
- **Phase 3B blocks on**: Nothing (USB is its own hardware domain)
- **Phase 3A blocked by**: Phase 2 (click dispatch needs frame model) — but Phase 2 is already 70% done, so 3A can start immediately on the remaining items
- **Phase 3B blocked by**: Nothing — pure hardware driver work
- **Key insight**: 3A and 3B are **fully parallelizable**. They don't share a single file. 3A is shell policy, 3B is USB driver.

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| XHCI MMIO mapping fails (ACPI/PCI enumeration missing) | Medium | HIGH (blocks all USB) | Use PCI configuration space directly (port-mapped I/O). Don't depend on ACPI. QEMU provides known-good PCI config. |
| USB interrupt endpoint timing (polling vs interrupts) | Medium | Medium | Start with polling at 1ms interval in sexusb's main loop. Switch to MSI/MSI-X later. Polling is simpler and reliable for mouse. |
| HID report descriptor parsing complexity | Low | Medium | Boot protocol mouse has fixed report format (3 bytes: buttons + x + y). Only support boot protocol in V1. Reject non-boot HID devices with clear log message. |
| USB descriptor parsing overflows | Low | High | Bounded buffer per descriptor stage. Max 64 bytes for first 8 bytes of device descriptor. Reject malformed descriptors. |
| Drag FSM state confusion under rapid clicks | Medium | Medium | Add `CLICK_COOLDOWN` counter. Ignore down events within N scheduler ticks of last down. Prevents double-click storm issues. |

## Exit Criteria (Done Checklist)

**Phase 3A:**
- [ ] Button-down/up event ordering contract documented and enforced
- [ ] Click → hit-test → focus → drag is atomic (single dispatch cycle, no yield between)
- [ ] Drag FSM handles: on-window, off-window, over-chrome, keyboard interrupt
- [ ] `[shell.click.*]` and `[shell.drag.*]` markers fire at expected counts
- [ ] No panic in drag state transitions (test with synthetic rapid click/drag)

**Phase 3B:**
- [ ] XHCI controller detected and initialized (log: `[sexusb.xhci.init]`)
- [ ] Root hub ports detected (log: `[sexusb.roothub.port] N connected=1`)
- [ ] Device descriptor read (log: `[sexusb.device.descriptor] vendor=XXXX product=XXXX`)
- [ ] HID boot mouse report received (log: `[sexusb.hid.report] buttons=X dx=X dy=X`)
- [ ] Normalized event reaches silk-shell (log: `[silk-shell.input.event] type=X code=X value=X`)
- [ ] Real mouse click triggers hit-test and focus change
- [ ] Boot + synthetic build both pass
- [ ] Zero panic/#PF/#GP

## Testing Strategy
- **3A**: Use synthetic mouse (SEXUSB_SYNTHETIC=1) to generate controlled click sequences. Verify FSM transitions with marker grep.
- **3B**: Test with QEMU's `-usb -device usb-mouse`. Log every USB descriptor byte received. Compare against known-good QEMU USB mouse descriptor.
- **Integration**: Combine 3A + 3B by running with SEXUSB_SYNTHETIC=0 and a real emulated mouse. Verify the full pipeline: physical mouse → USB → HID → OP_HID_EVENT → hit-test → action.

## Efficiency Opportunity
**The biggest time save is not waiting for 3B to complete 3A.** 3A can be fully tested and hardened using synthetic input (which already works). 3B can be developed independently. When 3B delivers the first real mouse event, 3A should already be proven — the integration test is just swapping the event source.

Additionally: **Merge 3B's smallest steps into Phase 0's gate infrastructure.** A `gate_usb_init.sh` that boots QEMU with a USB mouse and checks for the init log line would be the first reliable "USB works" signal.

## Completeness Gain
Input: **60% existing (3A) + 40% new (3B) → 100%** for mouse, **20–30% → 80–90%** overall input (trackpad/keyboard enhancements deferred)

## Files Changed
- `servers/silk-shell/src/main.rs` (click normalization, drag hardening)
- `servers/sexusb/src/main.rs` (XHCI, enumeration, HID)
- `servers/sexinput/src/main.rs` (HID normalization if needed)
- `scripts/gate_usb_init.sh` (new)

## Forbidden
- Trackpad gestures (deferred)
- Bluetooth HID (deferred)
- Shell policy rewrite (Phase 3A is additive, not rewriting)
- Compositor changes
- Linen/storage integration
- Scheduler/PKRU changes
- Shared-buffer redesign

## Next Phase
PHASE_04_LINEN_FILE_OBJECT_BROWSER.md

## Parallel Note
3A and 3B can be done simultaneously by different people/agents. They share no files and have no ordering dependency.
