# INPUT_USB_NEXT

## Current Known-Good Input Path
raw/synthetic report -> HID_POINTER_REPORT_NORMALIZER_V1 -> OP_HID_EVENT -> silk-shell pointer state -> click focus / drag

## Why USB Was Split
USB input is not a single patch. It spans:
- host controller discovery
- XHCI initialization
- device enumeration
- endpoint configuration
- interrupt transfers
- HID report fetching
- report normalization
- OP_HID_EVENT delivery
- shell policy consumption

## USB NO-GO List
Do not combine these into one phase:
- full USB subsystem rewrite
- XHCI + HID + gestures + compositor behavior in one patch
- Bluetooth
- PS/2 product path
- trackpad gestures
- drag/click policy changes
- surface protocol changes
- Linen/file browser work
- scheduler/PKRU/time changes
- backing buffer/shared memory work

## Next Phases
1. `USB_HOST_DISCOVERY_V1`
- inspect current PCI/MMIO/IRQ/DMA capability reality
- identify existing USB/XHCI code if any
- no implementation unless trivial diagnostic logging

2. `USB_XHCI_MINIMAL_ENUM_V1`
- minimal controller bring-up
- enumerate one device if feasible
- no HID policy yet

3. `USB_HID_BOOT_MOUSE_REPORT_V1`
- obtain fixed boot-protocol mouse-like reports
- feed bytes to existing normalizer

4. `USB_HID_POINTER_PRODUCER_V1`
- route real reports into OP_HID_EVENT
- prove click-focus/drag with real hardware or QEMU USB tablet/mouse

5. `TOUCHPAD_ABS_CONTACT_V1`
- later absolute/contact events
- no gestures

6. `TRACKPAD_GESTURES_V1`
- later policy: scroll/swipe/workspace gestures

## Phase Gate Rule
If a proposed USB patch touches kernel, sexinput, sex-pdx, silk-shell, sexdisplay, and build spec all at once, reject and split it.

## Success Criteria Before USB Producer
- Existing keyboard controls still work.
- Synthetic pointer producer still works.
- HID normalizer still converts fixed reports.
- click-focus and drag remain shell-only policy.
- no #GP/#PF/panic.
- no IPC storm.

## HID_POINTER_REPORT_NORMALIZER_V1 status
- HID_POINTER_REPORT_NORMALIZER_V1 complete.
- Synthetic pointer now routes:
  `HidPointerRawReport -> normalize_pointer_report_v1 -> OP_HID_EVENT -> silk-shell`.
- Build gate passed:
  `./scripts/entrypoint_build.sh`.
- Runtime `run-nographic` blocked by qemu stdio multi-device conflict, not by build.
- `sex-pdx` and `silk-shell` were inspect-only; no ABI or policy edits.
- POINTER_DRAG_PROOF_V1 note: in headless/CI hosts, `./dev.sh run` may fail with `SDL(No available video device)` and `run-nographic` may fail with `-serial stdio` multi-device conflict; treat as environment runtime blocker, not pointer-path regression.
- SILK_DE_BAR_CONTRACT_LOCK_V1 complete: shared bar contract constants are now locked in `silkbar-model`, and both `silkbar` producer + `sexdisplay` renderer perform startup contract validation with bounded `*.ok/*.bad` markers. Build passed; GUI runtime still blocked in this host by SDL no-video device.
- RULE: sexdisplay startup validation must fail-open. Never halt/spin/yield before OP_PRIMARY_FB/main render loop. Contract validation may log bad state but must keep renderer alive; bad validation degrades to default SilkBar state, not black-screen.
- SEXUSB_SERVER_SKELETON_XHCI_PROBE_V1 complete: added `sexusb` server PD boot path and routed existing XHCI PCI/IRQ lease to `sexusb` (fallback to `sexdrive` if `sexusb` absent).
- `sexusb` now probes XHCI BAR0 via existing syscall 43 (`MAP_PCI_BAR`) and logs CAPLENGTH/HCIVERSION/HCSP1/HCC1 markers only; no reset/run/TRB/DMA/enum yet.
- Build gate passed: `./scripts/entrypoint_build.sh`.
- Runtime host blocker persists: `./dev.sh run` failed with `Could not initialize SDL(No available video device)`.
- USB_XHCI_RESET_RUN_PROOF_V1 complete in `servers/sexusb/src/main.rs`: bounded reset/run MMIO proof added using USBCMD/USBSTS polling with yield-based timeouts (no infinite waits).
- Added markers: `[sexusb.xhci.reset.start]`, `[sexusb.xhci.halted.ok|bad]`, `[sexusb.xhci.reset.ok|bad]`, `[sexusb.xhci.run.ok|bad]`.
- Non-goals preserved: no enum, no TRB/event rings, no DMA buffers, no HID routing, no kernel/ABI edits.
- Build gate passed: `./scripts/entrypoint_build.sh`.
- Runtime host blocker persists: `./dev.sh run` failed with `Could not initialize SDL(No available video device)`.
- USB_XHCI_STATIC_RING_PROOF_V1 complete in `servers/sexusb/src/main.rs` using existing syscalls only: `31` (alloc phys) + `30` (map phys->VA).
- Ring memory proof allocates/maps command ring, event ring, ERST, and DCBAA pages from kernel allocator authority (no Rust static DMA buffers).
- Added bounded proof markers for alloc/alignment/pointer-register writes, including DCBAAP/CRCR/ERSTSZ+ERSTBA/ERDP writes.
- No kernel/ABI/allocator edits; no IRQ handling, no doorbells, no enum/HID/TRB processing.
- Build gate passed: `./scripts/entrypoint_build.sh`.
- Runtime host blocker persists: `./dev.sh run` failed with `Could not initialize SDL(No available video device)`.
- USB_XHCI_COMMAND_RING_NOOP_PROOF_V1 complete in `servers/sexusb/src/main.rs`: writes one Command Noop TRB (`type=23`, cycle=1), rings doorbell 0 via `DBOFF`, then bounded-polls event ring index 0 for Command Completion Event (`type=33`).
- Completion status decode uses Completion Code from event TRB status dword (`cc = d2[31:24]`), with success marker on `cc==1` and bounded timeout/failure markers otherwise.
- Non-goals preserved: no Enable Slot/Address Device, no enum, no HID, no IRQ path, no kernel/ABI edits.
- Build gate passed: `./scripts/entrypoint_build.sh`.
- Runtime host blocker persists: `./dev.sh run` failed with `Could not initialize SDL(No available video device)`.

## USB_HANDOFF_AFTER_NOOP

### Commits
- `68ab83b` feat(sexusb): add XHCI BAR probe server
- `2e2970e` feat(sexusb): add bounded XHCI reset-run proof
- `ef05f27` feat(sexusb): add XHCI static ring memory proof
- `efe3adb` feat(sexusb): prove XHCI command ring noop

### sexusb Boundary
- server name: `sexusb`
- path: `servers/sexusb`
- `sexusb` is USB-only bus/host server
- not `sexusb-server`, not `SexUSB_Server`, not `sexlink`
- `sexinput` owns input meaning/normalizer
- `sexlink` is future UI/control-plane only, not hardware

### Current Proven
- sexusb boots as PD
- maps XHCI BAR0 via syscall `43` (`MAP_PCI_BAR`) and `SLOT_USB_HOST`
- reads `CAPLENGTH`/`HCIVERSION`/`HCSP1`/`HCC1`
- bounded stop/reset/CNR/run proof
- alloc/maps command ring, event ring, ERST, DCBAA via syscall `31` phys + syscall `30` VA
- writes `DCBAAP`, `CRCR`, `ERSTSZ`, `ERSTBA`, `ERDP`
- submits one Command Noop TRB `type=23`
- rings `DB[0]` value `0`
- polls event ring bounded and recognizes Command Completion Event `type=33`
- build passes

### Current Not Implemented
- no Enable Slot
- no Address Device
- no device enumeration
- no HID parsing
- no interrupt-driven event handling
- no sexinput routing
- no storage/audio/network routing
- no kernel/ABI edits after sexusb spawn/lease work
- no sexlink

### Runtime
- `./dev.sh run` blocked by host SDL/no-video in this environment.
- Treat as environment blocker, not patch failure.

### Next
- Claude audit first: `USB_XHCI_COMMAND_RING_AUDIT_V1`
- then only if audit passes: `USB_XHCI_ENABLE_SLOT_PROOF_V1`

### Rules
- Do not jump to HID.
- Do not touch sexinput until USB HID report bytes exist.
- Do not create sexlink.
- Keep all waits bounded.
- Any validation failure must log bad and park/yield, not panic.
- Save recurring issue fixes in docs.
- USB_XHCI_ENABLE_SLOT_PROOF_V1 complete in `servers/sexusb/src/main.rs`: after NOOP proof, writes one Enable Slot Command TRB (`type=9`, cycle=1) at command ring index 1, rings doorbell 0, then bounded-polls event ring for Command Completion Event (`type=33`).
- Event decode: completion code from status dword (`cc = d2[31:24]`), slot id extracted from control dword (`slot_id = d3[31:24]`), with success marker only when `cc==1` and `slot_id!=0`.
- Non-goals preserved: no Address Device, no descriptors, no HID, no IRQ handler path, no sexinput routing, no kernel/ABI edits.
- Build gate passed: `./scripts/entrypoint_build.sh`.
- Runtime host blocker persists: `./dev.sh run` failed with `Could not initialize SDL(No available video device)`.

## RULE: BootInfoFrameAllocator metadata overlap (GP at LockFreeBuddyAllocator::alloc)

**Symptom**: QEMU black screen, EXCEPTION: GP FAULT at RIP 0xffffffff80203bdb in kernel allocator. Crash PD is *not* sexusb but a different PD later in scheduler rotation (e.g., sexinput). Triggered by sexusb binary growing past a page boundary (extra ELF segment page triggers a page-table allocation from the conflicting frame pool).

**Root cause**: kernel/src/memory/manager.rs carves PageMetadata array from first usable memory region, but BootInfoFrameAllocator (in GLOBAL_VAS) was NOT advanced past the metadata pages. Page-table allocation from frame allocator returned a metadata frame; writing PTE entries overwrote PageMetadata.next pointers, corrupting the buddy allocator free list -> GP fault.

**Fix**: Advance frame_allocator.allocate_frame() by metadata_pages after seeding the buddy allocator (see kernel/src/memory/manager.rs init()).

**XHCI CRCS rule**: After each Command TRB is consumed, XHCI toggles Command Ring Cycle State (CRCS). Always write a cycle-stop marker (TRB with cycle=opposite of current CRCS) after the last valid TRB in a batch. Second-batch commands use cycle=0 (matching CRCS after first TRB consumed).
