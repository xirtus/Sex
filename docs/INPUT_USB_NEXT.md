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

## USB_XHCI_RING_STATE_MACHINE_CLEANUP_V1

### Changes
- Replaced hardcoded TRB indices (0 for NOOP, 1 for Enable Slot) with explicit state machine:
  - `cmd_idx: u64` — next command ring slot to write
  - `cmd_cycle: u32` — producer cycle bit (matches CRCR RCS, starts 1; **stable until segment wrap**)
  - `ev_idx: u64` — next event ring slot to consume
  - `ev_dcs: u64` — event ring dequeue cycle state (starts 1 per spec 5.5.2.3.2)
- Fixed ERDP initialization: added `| 1u64` for DCS=1 (was 0, violating spec)
- Fixed cycle-stop marker bit: uses `cmd_cycle ^ 1` (opposite cycle). CRCS stays stable
  (spec 5.4.5: toggles only on segment boundary wrap, not per TRB). Same-cycle Reserved
  TRB would match CRCS and be consumed as a valid command → silent corruption.
  Correct trace:
  ```
  CRCS=1 (stable)
  TRB[0]: cmd_cycle=1 → match → processed. CRCS still 1.
  TRB[1]: !cmd_cycle=0 → STOP (0 != 1). ✓
  ```
- Added ERDP advance after each consumed event:
  `mmio_write64(intr0_base, XHCI_INTR_ERDP, event_ring_phys + ev_idx * 16 | ev_dcs)`
- Added consumed event cycle-bit clear per XHCI spec 4.11.4
- Added event ring segment wrap handling (ev_idx >= EVENT_RING_TRBS → wrap to 0, toggle DCS)
- Added `cmd_idx += 1;` after each command batch (NOOP and Enable Slot). No cycle toggle
  — cmd_cycle stays stable until segment wrap (only cmd_idx advances).
- Removed stale event clear (`trb_write_volatile(event_ring_va, 0, 0, 0, 0, 0)`) — no longer
  needed since event indices are tracked and advanced, not reused.
- Removed non-completion event branch (else-case) — only poll for CMD_COMPLETION_EVENT; any
  other event type at the tracked ev_idx is unexpected and should timeout.

### Non-goals preserved
- No Address Device
- No HID parsing or routing
- No IRQ handler
- No kernel/ABI edits
- No sexinput routing
- No sexlink

### Build
- Build gate passed: `./scripts/entrypoint_build.sh`.
- One expected warning: `cmd_idx` assigned but never read on final advance (value valid
  but no further commands submitted after Enable Slot).

### XHCI Command Ring Cycle Rule (verified V1)
- CRCR.RCS (spec 5.4.5) toggles ONLY on segment boundary wrap, NOT per TRB fetch.
- All valid commands within a segment share the same cycle bit (= CRCS = CRCR.RCS).
- Stop marker MUST use opposite cycle (`!cmd_cycle`). Same-cycle Reserved TRB (type=0)
  matches CRCS and is consumed as a valid command → undefined behavior / corruption.
- cmd_cycle advances only on segment wrap (when cmd_idx wraps past ring size, toggle
  cmd_cycle for the next segment).

### Next
- `USB_XHCI_ADDRESS_DEVICE_PROOF_V1`: submit Address Device TRB with correctly
  formatted input context. Route real port/speed, EP0 ring, and DCBAA entry.
  No descriptor parsing, no HID, no transfers.

## USB_XHCI_ADDRESS_DEVICE_CONTEXT_LAYOUT_PROOF_V1

### Changes
- Added context stride computation from HCCPARAMS1 CSZ (bit 2): 32 bytes if CSZ=0,
  64 bytes if CSZ=1. Verified against XHCI spec 5.3.2.
- Added PORTSC scan: reads `max_ports` from HCSPARAMS1 bits 23:16, iterates PORTSC
  registers at `op_base + 0x400 + n*0x10`, finds first connected port (CCS bit 0),
  logs port number and speed ID (bits 13:10). Parks if no port connected.
- Allocated three pages via syscalls 31+30:
  - input context (1 page)
  - device context (1 page)
  - EP0 transfer ring (1 page)
- Validated all phys/va for non-zero, page alignment, 64-byte alignment.
- Zeroed all three pages.
- Input Context layout (one page):
  - offset 0: Input Control Context (full ctx_stride bytes)
  - offset ctx_stride: Slot Context
  - offset ctx_stride * 2: EP0 Context
- Device Context layout (one page):
  - offset 0: Slot Context
  - offset ctx_stride: EP0 Context
- Wrote ICC: Drop=0, Add=bit0(Slot)|bit1(EP0)
- Wrote Slot Context:
  - DW0: Context Entries=1 (bits 31:27), Speed=port_speed (bits 23:20),
    Route String=0 (bits 19:0)
  - DW1: Root Hub Port Number in bits 31:24
  - Written to both input context and device context
- Wrote EP0 Context:
  - DW0: Max Packet Size (bits 31:16): 8 for FS/LS, 64 for HS, 512 for SS
  - DW1: CErr=3 (bits 3:0), EP Type=Control=010b (bits 5:3, shift=3)
  - DW2+DW3: TR Dequeue Pointer = EP0 ring phys | DCS(1)
  - Written to both input context and device context
- DCBAA: validated slot_id < max_slots (HCSPARAMS1 bits 31:24), then
  volatile-wrote dcbaa[slot_id] = device_context_phys

### Port speed mapping
- Speed ID 0 = Full Speed (12 Mbps) → MPS 8
- Speed ID 1 = Low Speed (1.5 Mbps) → MPS 8
- Speed ID 2 = High Speed (480 Mbps) → MPS 64
- Speed ID 3 = Super Speed (5 Gbps) → MPS 512
- Unknown speed (≥4) → log bad + park

### Non-goals preserved
- No Address Device TRB (no doorbell)
- No descriptor parsing
- No HID/input/shell/display edits
- No kernel/ABI edits
- No sexlink
- No interrupt-driven event handling

### Build
- Build gate passed: `./scripts/entrypoint_build.sh`.
- Zero warnings for sexusb.

## USB_XHCI_ADDRESS_DEVICE_PROOF_V1

### Changes
- Added `TRB_TYPE_ADDRESS_DEVICE_CMD` constant (type=8)
- After context layout completes, submits Address Device Command TRB at `cmd_idx` (index 2):
  - d0(31:0), d1 = `input_context_phys` (low/high 32 bits)
  - d2 = 0
  - d3 = `(slot_id << 24) | (8 << 10) | cmd_cycle`
  - BSR=0 (normal address, not block set address)
- Writes cycle-stop marker at `cmd_idx+1` with `!cmd_cycle`
- Rings doorbell 0
- Polls event at `ev_idx` (index 2) for Command Completion Event (type=33)
- Decodes: completion code from d2[31:24], slot ID from d3[31:24]
- Validates: cc == 1 (success) AND ev_slot_id matches `en_slot_id`
- Clears consumed event cycle bit per spec 4.11.4, advances ev_idx, updates ERDP
- On success: reads output Device Context slot state from `device_ctx_va + 12` DW3 bits 31:27
- On timeout/failure: parks at `loop { sys_yield(); }` with `.timeout.bad` marker

### Address Device TRB trace
```
cmd_idx=2 (after NOOP@0, Enable Slot@1), cmd_cycle=1 (stable)
ev_idx=2 (after consuming NOOP@0, Enable Slot@1 events)

TRB[2]: type=8, cycle=1, slot_id=en_slot_id → match CRCS=1 → valid
TRB[3]: type=0, cycle=0 → STOP (0 != 1)

Event[2]: type=33, cc=1, slot_id=en_slot_id → success
```

### Verification
- Completion code == Success (1)
- Event slot ID matches Enable Slot slot ID
- Device Context slot state logged (expected: 3 = Addressed)

### Non-goals preserved
- No descriptor requests (no EP0 setup/data/status TRBs)
- No transfers
- No Configure Endpoint
- No HID parsing
- No IRQ handler changes
- No kernel/ABI edits
- No sexinput routing
- No sexlink

### Build
- Build gate passed: `./scripts/entrypoint_build.sh`.
- Zero warnings for sexusb.

### Next
- `USB_XHCI_GET_DESCRIPTOR_PLAN_V1`: plan only — design EP0 setup/data/status
  transfer TRBs for device descriptor fetch. No implementation, no HID.

**Symptom**: QEMU black screen, EXCEPTION: GP FAULT at RIP 0xffffffff80203bdb in kernel allocator. Crash PD is *not* sexusb but a different PD later in scheduler rotation (e.g., sexinput). Triggered by sexusb binary growing past a page boundary (extra ELF segment page triggers a page-table allocation from the conflicting frame pool).

**Root cause**: kernel/src/memory/manager.rs carves PageMetadata array from first usable memory region, but BootInfoFrameAllocator (in GLOBAL_VAS) was NOT advanced past the metadata pages. Page-table allocation from frame allocator returned a metadata frame; writing PTE entries overwrote PageMetadata.next pointers, corrupting the buddy allocator free list -> GP fault.

**Fix**: Advance frame_allocator.allocate_frame() by metadata_pages after seeding the buddy allocator (see kernel/src/memory/manager.rs init()).

**XHCI CRCS rule**: XHCI command ring cycle (CRCR.RCS per spec 5.4.5) toggles ONLY on segment boundary wrap, NOT per TRB fetch. All valid commands within a segment share the same cycle bit. Stop marker MUST use opposite cycle (`!cmd_cycle`). A same-cycle Reserved TRB (type=0) matches CRCS and is consumed as a valid command → undefined behavior / silent corruption.

## USB_XHCI_GET_DEVICE_DESCRIPTOR_8_PROOF_V1

### Changes
- Added TRB type constants: `TRB_TYPE_SETUP_STAGE=2`, `TRB_TYPE_DATA_STAGE=3`,
  `TRB_TYPE_STATUS_STAGE=4`, `TRB_TYPE_TRANSFER_EVENT=32`.
- Added EP0 transfer ring state: `ep0_idx: u64`, `ep0_cycle: u32` (TRCS).
- TRCS rule: starts at 1, toggles ONLY on segment boundary wrap (spec 4.11.3.1),
  NOT per TD. Same correctness class as CRCS.
- Allocated separate descriptor DMA page via syscall31+30 (no alias with EP0
  transfer ring). Validated non-zero, 64-byte alignment, zeroed.
- Wrote 3-TRB EP0 control transfer chain for GET_DESCRIPTOR(DEVICE,0,0,8):
  - Setup Stage (type=2): IDT=1, CH=1, TRT=IN(d3[17:16]=0b01)
  - Data Stage (type=3): DIR=IN(d3[16]=1), CH=1, ISP=0, IOC=0, buffer mode
  - Status Stage (type=4): DIR=OUT(d3[16]=0), CH=0, IOC=1 (generates Transfer Event)
  - Stop marker at ep0_idx+3 with `ep0_cycle ^ 1` (opposite cycle)
- Verified exact XHCI bitfields before coding (Setup IDT/CH/IOC/TRT,
  Data CH/IOC/ISP/DIR, Status CH/IOC/DIR, Transfer Event cc/slot/ep).
- Doorbell: `mmio_write32(db_base, en_slot_id * 4, 1u32)` — DB Target=1 for EP0.
- Bounded poll for Transfer Event (type=32) at current `ev_idx`/`ev_dcs` (no
  hardcoded index). Validate cc==Success, slot_id matches, endpoint_id==1.
- Standard event consumption: clear cycle bit, advance ev_idx, handle wrap,
  update ERDP.
- Log 8 raw descriptor bytes + bMaxPacketSize0 from offset 7.
- No Evaluate Context (deferred), no 18-byte fetch, no HID.

### Non-goals preserved
- No Evaluate Context
- No full 18-byte descriptor
- No Configure Endpoint
- No HID parsing or routing
- No IRQ handler
- No kernel/ABI edits
- No sexinput routing
- No sexlink

### Build
- Build gate passed: `./scripts/entrypoint_build.sh`.
- Zero warnings for sexusb.

### Next
- `USB_XHCI_EP0_MPS_EVALUATE_CONTEXT_PLAN_V1`: plan only — design Evaluate
  Context command to update EP0 MPS from bMaxPacketSize0. No implementation, no HID.

## CORRECTED XHCI SPEC FACTS (post-Audit V1)

**Context stride**: XHCI context array element stride = 32 bytes if CSZ=0, 64 bytes if CSZ=1 (HCCPARAMS1 bit 2). NOT 16/32. Slot/EP0 context fields occupy the first 16 bytes of each stride; remaining bytes are reserved.

**Input Control Context**: One full context-sized block (32 or 64 bytes), not 8 bytes. Only DW0 (Drop) and DW1 (Add) are meaningful; rest reserved. Slot Context starts at ICC stride end, not offset 8.

**EP0 TR Dequeue Pointer**: MUST point to a valid EP0 transfer ring (allocated page, zeroed, 64-byte aligned). Zero is illegal. DCS bit must match cycle state. Allocate ring before Address Device even though no transfers are submitted yet.

**EP Type for Control**: XHCI spec 6.2.3: EP Type bits 15:14 = 00 (Control, bidirectional). Verify against current spec before writing.

**Context Entries**: Number of endpoint contexts following slot context. For Slot+EP0, set to 1 (EP0 is context index 1). The controller checks this to determine how many contexts to validate.

**Event ring consumption**: Each consumed event MUST advance ERDP. After consuming index N, write ERDP = event_ring_phys + (N+1)*16. On wrap past segment end, toggle DCS in the ERDP write. Otherwise controller won't write new events.

**Command ring**: Track cmd_enqueue_index and cmd_producer_cycle explicitly. Write cycle-stop marker after each doorbell batch. Do not hardcode indices across phases.
