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

## USB_XHCI_EP0_MPS_EVALUATE_CONTEXT_PROOF_V1

### Changes
- Added constant: `TRB_TYPE_EVALUATE_CONTEXT_CMD: u32 = 14`.
- After GET_DESCRIPTOR(8) completes, reads bMaxPacketSize0 and validates:
  - `actual_mps` must be in `{8,16,32,64,512}` (valid EP0 MPS values per USB spec).
  - If `actual_mps == max_packet_size` (boot-guess from port speed): skip, park.
  - If `actual_mps != max_packet_size` and valid: run Evaluate Context command.
- Reuses existing `input_ctx_va`/`input_ctx_phys` page (controller done reading
  after Address Device). Zeroes ICC area.
- ICC: Drop=0, Add=bit 1 (EP0 only, context index 1). Slot context not evaluated.
- Copies EP0 context from output Device Context (`device_ctx_va + ctx_stride`) into
  Input Context (`input_ctx_va + ctx_stride * 2`), then patches only DW0 bits 31:16
  with `actual_mps`. Required by XHCI spec 6.2.3: fields not intended to be changed
  must be copied from current Device Context (preserves controller-updated TR
  Dequeue Pointer, DCS, etc.).
- Submits Evaluate Context Command TRB (type=14): d0/d1=input_context_phys, d2=0,
  d3=(slot_id<<24)|(14<<10)|cmd_cycle. Stop marker opposite cycle. DB[0]=0.
- Polls Command Completion Event (type=33) at current ev_idx. Validates cc==Success,
  slot_id matches. Consumes event, advances ev_idx, updates ERDP.
- Verifies output Device Context EP0 DW0 bits 31:16 reflect updated MPS. Logs
  verify ok/bad.
- Skip path is terminal for this proof phase only; future full18 phase continues
  from either skip-ok or eval-ok.

### Non-goals preserved
- No GET_DESCRIPTOR(18) full fetch
- No new EP0 transfer TRBs
- No HID parsing or routing
- No IRQ handler
- No kernel/ABI edits
- No sexinput/silk-shell/sexdisplay edits
- No sexlink

### Build
- Build gate passed: `./scripts/entrypoint_build.sh`.
- Zero warnings for sexusb.

### Next
- `USB_XHCI_GET_DESCRIPTOR_FULL_18_PLAN_V1`: plan only — design full 18-byte
  device descriptor fetch after MPS is validated/updated. No implementation, no HID.

## USB_XHCI_GET_DESCRIPTOR_FULL_18_PROOF_V1

### Changes
- Reads EP0 TR Dequeue Pointer from Device Context output at runtime
  (`device_ctx_va + ctx_stride`, DW2+DW3). Validates deq_dcs==1, phys within
  ring page, 16-byte alignment. Computes `deq_index = (phys - ep0_ring_phys)/16`.
  No hardcoded index. Bounds check: deq_index+3 < 256.
- Zeros descriptor data buffer (first 18 bytes) before TD, preventing stale
  8-byte data from masquerading as valid descriptor bytes on short/residual.
- Writes 3-TRB chain at verified `deq_index`:
  - Setup Stage (type=2): IDT=1, CH=1, TRT=IN, wLength=18,
    TRB Transfer Length=18 (verified per spec Table 6-34).
  - Data Stage (type=3): DIR=IN, CH=1, TRB Transfer Length=18.
  - Status Stage (type=4): DIR=OUT, CH=0, IOC=1.
  - Stop marker at deq_index+3 with `ep0_cycle ^ 1` (opposite cycle).
- Doorbell DB[slot_id] target=1 (EP0).
- Consumes Transfer Event (type=32) at current ev_idx. Validates cc==Success,
  slot_id matches, endpoint_id==1. Reads residual from d2[23:0].
- Residual policy: residue==0 → complete.ok; residue>0 → residue.warn + park
  (no complete.ok); residue>=18 → residue.full.bad + park.
- Logs raw 18 bytes + informational fields: bLength, bDescriptorType, bcdUSB,
  class/subclass/protocol, bMaxPacketSize0, idVendor, idProduct, bcdDevice,
  iManufacturer/iProduct/iSerial, bNumConfigurations. No routing/parsing.
- MPS consistency check: full18 bMaxPacketSize0 must match earlier 8-byte fetch;
  mismatch logs bad + park.
- Descriptor sanity warnings (non-fatal): bLength != 18, bDescriptorType != 1.

### Non-goals preserved
- No SET_CONFIGURATION
- No Configure Endpoint
- No config descriptor fetch
- No HID routing/parsing
- No sexinput routing
- No IRQ handler
- No kernel/ABI edits
- No sexlink

### Build
- Build gate passed: `./scripts/entrypoint_build.sh`.
- Zero warnings for sexusb.

### Next
- `USB_XHCI_CONFIG_DESCRIPTOR_PLAN_V1`: plan only — design GET_DESCRIPTOR(CONFIGURATION)
  header TD and full config TD, then parse descriptor walk to discover HID boot mouse
  interface (class=0x03, subclass=0x01, protocol=0x02) and its interrupt IN endpoint.
  No implementation, no SET_CONFIGURATION, no HID report fetch, no interrupt transfers.

## USB_XHCI_CONFIG_DESCRIPTOR_PLAN_V1

### Purpose
Discover the USB Configuration descriptor to find a HID boot mouse interface (class=0x03,
subclass=0x01, protocol=0x02) and its interrupt IN endpoint. This is a prerequisite for
later HID report fetching and interrupt transfers. Plan only — no implementation.

### Two-Phase EP0 Transfer Design

**TD1 — Config Header (GET_DESCRIPTOR(CONFIGURATION, 0, wLength=9)):**
- Allocated desc_data_va page from GET_DESCRIPTOR(8) already has room for >9 bytes.
- Zero first 9 bytes before TD to prevent stale data on short/residual.
- Setup Stage: GET_DESCRIPTOR(CONFIGURATION, index=0, wLength=9)
  - d0 = 0x0200_0680 (bmReqType=0x80, bReq=0x06, wValue_lo=0x00, wValue_hi=0x02)
  - d1 = (9u32 << 16) (wIndex=0, wLength=9)
  - TRB Transfer Length = 9, IDT=1, CH=1, TRT=IN
- Data Stage: DIR=IN, CH=1, TRB Transfer Length=9, buffer=desc_data_phys
- Status Stage: DIR=OUT, CH=0, IOC=1
- Poll Transfer Event. Decode residue.
- **residue > 0 → [sexusb.config.header_residue.bad] → fatal park** (can't trust wTotalLength)
- Read wTotalLength from desc buffer bytes 2(lo),3(hi). Validate:
  - **wTotalLength < 9 → [sexusb.config.total_len.bad] → fatal park** (corrupt descriptor)
  - wTotalLength == 9 → valid but no descriptors beyond header → no-HID will park later
- Log wTotalLength.

**TD2 — Full Config (GET_DESCRIPTOR(CONFIGURATION, 0, wLength=wTotalLength)):**
- Read EP0 dequeue pointer from Device Context at runtime, compute deq_index.
- Verify deq_index + 3 < 256 (fits in EP0 ring page).
- Zero desc_data_va first wTotalLength bytes before TD.
- Setup Stage: GET_DESCRIPTOR(CONFIGURATION, index=0, wLength=wTotalLength)
  - d0 = 0x0200_0680
  - d1 = (wTotalLength as u32) << 16  ← NOT .to_le(), correct LE packing via shift
  - TRB Transfer Length = wTotalLength, IDT=1, CH=1, TRT=IN
- Data Stage: DIR=IN, CH=1, TRB Transfer Length=wTotalLength, buffer=desc_data_phys
- Status Stage: DIR=OUT, CH=0, IOC=1
- Poll Transfer Event. Decode residue.
  - **residue >= wTotalLength → [sexusb.config.full_residue_full.bad] → fatal park**
  - **residue > 0 → [sexusb.config.full_residue_partial.warn] → walk only received_len = wTotalLength - residue**

### Descriptor Walk Design

Walk received buffer from offset 0, tracking `offset < received_len`:
- Each descriptor: bLength at offset, bDescriptorType at offset+1.
- If bLength == 0 → malformed → [sexusb.config.desc_zero_len.bad] → fatal park.
- If bLength > received_len - offset → truncation → fatal park (can't safely parse).
- Advance offset += bLength.

**bDescriptorType values:**
- type=2 (CONFIGURATION): skip (header already consumed, but walk skips these fields)
- type=4 (INTERFACE): parse bInterfaceClass(offset+5), bInterfaceSubClass(offset+6),
  bInterfaceProtocol(offset+7). If class=0x03, subclass=0x01, protocol=0x02 → set
  `inside_hid_mouse = true`. Otherwise clear `inside_hid_mouse = false`.
- type=0x21 (HID): only if `inside_hid_mouse == true`.
  Validate: bLength >= 9, bDescriptorType == 0x21.
  Read bDescriptorType[0] at offset+6, must be 0x22 (HID Report).
  Read wDescriptorLength at bytes offset+7(low), offset+8(high).
  If any HID field invalid → **[sexusb.config.hid_desc.bad]** → fatal park.
- type=5 (ENDPOINT): only if `inside_hid_mouse == true`.
  bEndpointAddress at offset+2: bit 7 = direction (1=IN), bits 3:0 = endpoint number.
  bmAttributes at offset+3: bits 1:0 = type (0x03=Interrupt).
  wMaxPacketSize at bytes offset+4(low), offset+5(high): mask with 0x07FF for packet size.
  - **If wMaxPacketSize & 0x07FF == 0 → [sexusb.config.intr_ep_mps.bad]** → fatal park.
  - If direction=IN, type=Interrupt, MPS > 0 → found interrupt IN endpoint.
  - Log endpoint address, MPS, interval (bInterval at offset+6).

Only the first HID boot mouse interface and its first interrupt IN endpoint are recorded.
If walk completes without finding HID boot mouse → **[sexusb.config.no_hid.park]** → park.

### Non-goals preserved
- No SET_CONFIGURATION
- No HID report fetch (GET_DESCRIPTOR(HID report) deferred)
- No interrupt endpoint transfers
- No HID routing to sexinput
- No IRQ handler changes
- No kernel/ABI edits
- No sexlink

### Next
- `USB_XHCI_HID_REPORT_DESCRIPTOR_PLAN_V1`: plan only — design GET_DESCRIPTOR(HID report)
  to obtain report descriptor and determine report size. Implementation deferred until
  config descriptor phase proves HID boot mouse interface + interrupt IN endpoint exist.

### Build
- Build gate passed: `./scripts/entrypoint_build.sh`.
- Zero warnings for sexusb.

## USB_XHCI_CONFIG_DESCRIPTOR_PROOF_V1

### Changes
- After full18 completes, initiates two-phase config descriptor discovery:
- **TD1 — Config Header (wLength=9):** reads EP0 dequeue pointer from Device Context,
  zeros first 9 bytes of descriptor buffer, writes 3-TRB chain for GET_DESCRIPTOR
  (CONFIGURATION, 0, 9). Validates Transfer Event residue == 0 (fatal park if >0).
  Reads wTotalLength from bytes 2(lo),3(hi). Rejects <9, allows ==9 (no-HID parks).
- **TD2 — Full Config (wLength=wTotalLength):** reads EP0 dequeue pointer again (after
  TD1 consumption), zeros first wTotalLength bytes, writes 3-TRB chain with
  `d1 = (wTotalLength as u32) << 16` (correct LE packing). Validates residue: >=
  wTotalLength → fatal park; >0 → partial.warn, walk only received_len; 0 → complete.
- **Descriptor walk:** iterates received buffer, parsing each descriptor by type.
  Tracks `inside_hid_mouse` flag set on INTERFACE descriptor matching
  class=0x03/subclass=0x01/protocol=0x02, cleared on next INTERFACE.
  HID descriptor (type=0x21): validates bDescriptorType[0]==0x22 at offset+6, reads
  wDescriptorLength at offsets +7/+8 (not +8/+9).
  Endpoint descriptor (type=5): masks wMaxPacketSize with 0x07FF, rejects 0.
  Records first interrupt IN endpoint found within matched HID interface.
- Markers: `config.header_residue.bad`, `config.total_len.bad`,
  `config.full_residue_full.bad`, `config.full_residue_partial.warn`,
  `config.desc_zero_len.bad`, `config.desc_truncated.bad`,
  `config.hid_desc.bad`, `config.intr_ep_mps.bad`, `config.no_hid.park`,
  `config.complete.ok`.

### Non-goals preserved
- No SET_CONFIGURATION
- No Configure Endpoint command
- No HID report fetch
- No interrupt endpoint transfers
- No HID routing to sexinput
- No IRQ handler changes
- No kernel/ABI edits
- No sexlink

### Setup d1 packing note
For GET_DESCRIPTOR(CONFIGURATION) setup packet:
- d0 = 0x0200_0680 (bmReqType=0x80, bReq=0x06, wValue=0x0200)
- d1 = (wLength as u32) << 16 (wIndex=0, wLength in bytes 6-7 LE)
  Verified against USB spec Table 9-2: wLength occupies bytes 6(low) and 7(high)
  of the 8-byte setup packet. With wIndex=0, the u32 LE representation is
  (wLength_hi << 24) | (wLength_lo << 16) | 0 | 0 = wLength << 16.

### XHCI Spec Facts Verified
- **Setup Stage wLength packing**: USB setup packet bytes [6..7] = wLength in little-endian.
  d1 = wLength << 16 (since wIndex=0, this places wLength in bytes 6-7 of the 8-byte packet).
  Verified against USB spec Table 9-2 (Standard Device Requests) and XHCI spec Table 6-34
  (Setup Stage TRB).
- **Config descriptor wTotalLength**: offset 2(lo), 3(hi) in u16 little-endian.
- **HID descriptor wDescriptorLength**: offset 7(lo), 8(hi). Not 8,9.
- **Endpoint wMaxPacketSize**: u16 at offset 4(lo),5(hi). Only bits 10:0 = packet size.
  Mask with 0x07FF. Upper bits: 12:11 = transactions/burst, 15:13 = reserved.
- **Interface association rule**: Each INTERFACE descriptor starts a new interface context.
  HID and ENDPOINT descriptors belong to the most recently parsed INTERFACE descriptor.
  Track state via `inside_hid_mouse` flag, cleared on next INTERFACE.

## CORRECTED XHCI SPEC FACTS (post-Audit V1)

**Context stride**: XHCI context array element stride = 32 bytes if CSZ=0, 64 bytes if CSZ=1 (HCCPARAMS1 bit 2). NOT 16/32. Slot/EP0 context fields occupy the first 16 bytes of each stride; remaining bytes are reserved.

**Input Control Context**: One full context-sized block (32 or 64 bytes), not 8 bytes. Only DW0 (Drop) and DW1 (Add) are meaningful; rest reserved. Slot Context starts at ICC stride end, not offset 8.

**EP0 TR Dequeue Pointer**: MUST point to a valid EP0 transfer ring (allocated page, zeroed, 64-byte aligned). Zero is illegal. DCS bit must match cycle state. Allocate ring before Address Device even though no transfers are submitted yet.

**EP Type for Control**: XHCI spec 6.2.3: EP Type bits 15:14 = 00 (Control, bidirectional). Verify against current spec before writing.

**Context Entries**: Number of endpoint contexts following slot context. For Slot+EP0, set to 1 (EP0 is context index 1). The controller checks this to determine how many contexts to validate.

**Event ring consumption**: Each consumed event MUST advance ERDP. After consuming index N, write ERDP = event_ring_phys + (N+1)*16. On wrap past segment end, toggle DCS in software state (not ERDP). ERDP low bits are reserved per spec 5.3.8.3 (bits 2:0 = 0, bit 3 = EHB). Do NOT encode DCS into ERDP.

**ERDP low bits (spec 5.3.8.3)**: ERDP bits 2:0 are reserved (must be 0). Bit 3 is EHB (Event Handler Busy, RW1C). Event ring DCS is a software-only variable that tracks the expected cycle bit on incoming event TRBs. It is NOT stored in ERDP. Writing `event_ring_phys | 1` sets reserved bit 0, which may cause undefined controller behavior including command ring rejection. Initial ERDP must be `event_ring_phys` with no low bits set. When advancing ERDP after event consumption, use `next_event_phys` only, never `next_event_phys | dcs`.

**Command ring**: Track cmd_enqueue_index and cmd_producer_cycle explicitly. Write cycle-stop marker after each doorbell batch. Do not hardcode indices across phases.

**XHCI init order (spec 4.6.6)**: DCBAAP and CRCR must be programmed BEFORE Run/Stop (USBCMD.RS=1). The correct order is:
  1. Halt controller (clear RS, wait HCHalted)
  2. Reset controller (set HCRST, wait HCRST=0 + CNR=0)
  3. Allocate/zero ring memory pages (cmd ring, event ring, ERST, DCBAA)
  4. Program CONFIG.MaxSlotsEnabled = MaxSlots from HCSPARAMS1[31:24] (op_base+0x08)
  5. Program DCBAAP (operational register 0x30)
  6. Program CRCR (operational register 0x18) with RCS=1
  7. Program ERSTSZ/ERSTBA/ERDP (interrupter registers)
  8. Optionally set IMAN.IE=1
  9. Set USBCMD.RS=1 (Run/Stop), wait HCHalted=0
  10. Ring doorbell 0 to submit commands
  Violating this order causes the controller to ignore commands silently (NOOP appears written to ring but event never arrives). Missing CONFIG.MaxSlotsEnabled (step 4) leaves MaxSlotsEnabled=0 after reset, causing QEMU xhci to refuse command ring processing (CRR stays 0, HCE=0x1000).

**runtime_base+0x00 is MFINDEX (read-only)**: Interrupter 0 registers start at runtime_base+0x20, not runtime_base+0x00. Writing to runtime_base+0x00 through +0x1C hits MFINDEX and reserved space. Correct base for interrupter registers:
  - IMAN at intr_base+0x00 (runtime_base+0x20)
  - ERSTSZ at intr_base+0x08 (runtime_base+0x28)
  - ERSTBA at intr_base+0x10 (runtime_base+0x30)
  - ERDP at intr_base+0x18 (runtime_base+0x38)

**BAR mapping must be UC (Uncacheable)**: XHCI MMIO registers must not be cached. PTE must have PCD=1 (bit 4) and PWT=1 (bit 3). Without these, CPU caches MMIO reads/writes and register writes silently disappear. Verified: PTE raw=0xfebd401f shows bits 3 and 4 set.

**mmio_write64 must write upper dword first**: XHCI spec 3.2.4 requires 64-bit register writes to be upper-dword-first. Writing lower dword first on some implementations causes the controller to latch a partial value. Correct sequence: write offset+4, then write offset.

**QEMU nec-xhci CRCR/DCBAAP/ERSTBA/ERDP low-first quirk**: QEMU's `nec-usb-xhci` model latches the internal command ring pointer on the upper dword write of CRCR (offset 0x1c), using the CURRENT lower dword value. Writing upper dword first (per spec 3.2.4) while the lower dword is still 0 from reset causes the internal pointer to latch as 0. The fix:
- Write CRCR lower dword (offset 0x18) first, then upper dword (offset 0x1c).
- Same quirk applies to DCBAAP (offsets 0x30/0x34), ERSTBA (runtime+0x10/+0x14), and ERDP (runtime+0x18/+0x1c).
- All 64-bit xHCI controller registers in sexusb use lower-dword-first writes for QEMU compatibility.
- Do NOT generalize to all MMIO without proof. Only the four xHCI control registers are affected.
- ERDP advance after event consumption also uses lower-dword-first.

**BAR size cap**: QEMU nec-usb-xhci BAR0 spans 64KB (0x10000). Kernel MAP_PCI_BAR must not clamp to 4KB or runtime/operational registers past offset 0x1000 will alias or read back zero.

## XHCI_SLOT_CONTEXT_DW1_ROOT_PORT_FIX_V1

### Problem
Address Device command fails with QEMU CC_TRB_ERROR (cc=5). Prior "mirror bits 31:24 and 23:16" compat patch wrote `slot_dw1 = (target_port << 24) | (target_port << 16) = 0x05050000`, but bits 31:24 is Number of Ports (not alternative root port).

### Diagnosis
xHCI spec Slot Context DW1:
- bits 23:16 = Root Hub Port Number
- bits 31:24 = Number of Ports (must remain 0 for non-hub device)

Setting Number of Ports=5 (non-zero for non-hub) causes QEMU to reject Address Device.

### Fix
Change slot_dw1 to `let slot_dw1 = (target_port as u32) << 16;` → 0x00050000.

### Verification
- Slot DW1 serial print shows 0x00050000

## XHCI_COMMAND_TRB_TYPE_FIX_V1

### Problem
Address Device command returns CC_TRB_ERROR (5) despite correct input context and slot context. QEMU trace shows our TRB displayed as "TR_NOOP" instead of "CR_ADDRESS_DEVICE".

### Diagnosis
xHCI spec defines COMMAND TRB type values that differ from transfer-ring types in the shared 6-bit type field (xHCI 1.2 §4.11.3). The old constants used transfer-ring names/values:
- `TRB_TYPE_ADDRESS_DEVICE_CMD = 8` — this is TRANSFER NO-OP (type 8), not Address Device Command (type 11)
- `TRB_TYPE_EVALUATE_CONTEXT_CMD = 14` — this is Reset Endpoint Command (type 14), not Evaluate Context (type 13)

Correct command TRB type values per command-ring encoding:
- Enable Slot = 9 (not transfer value 1)
- Address Device = 11 (not transfer value 8)
- Evaluate Context = 13 (not transfer value 10)
- Noop Command = 23

### Fix
Change `TRB_TYPE_ADDRESS_DEVICE_CMD` from 8 → 11, `TRB_TYPE_EVALUATE_CONTEXT_CMD` from 14 → 13. Add comment explaining command-ring vs transfer-ring namespace.

### Verification
- QEMU trace shows `CR_ADDRESS_DEVICE` (not `TR_NOOP`) for our Address Device TRB
- `usb_xhci_slot_address` trace fires for our TRB
- `[sexusb.xhci.address_device.complete.ok]` with cc=1 slot=1

### Remaining (desc8 timeout)
GET_DESCRIPTOR(Device) transfer on EP0 ring times out. TRBs are fetched (SETUP/DATA/STATUS visible in QEMU trace) but no Transfer Event arrives. Doorbell or cycle bit issue on transfer ring.
- QEMU trace `usb_xhci_slot_address` fires for our TRB (was previously absent)
- `[sexusb.xhci.address_device.complete.ok]` appears with cc=1

## XHCI_ADDRESS_DEVICE_CC5_CONTEXT_AUDIT_V1

### Problem
Address Device command fails with QEMU CC_TRB_ERROR (cc=5). CONFIG.MaxSlotsEnabled write applied but did not fix. QEMU compat root port fix (bits 23:16 + 31:24) applied but did not fix.

### Diagnosis
QEMU nec-xhci uses shifted completion codes: CC_SUCCESS=1, CC_TRB_ERROR=5, CC_SLOT_NOT_ENABLED_ERROR=11. So cc=5 is NOT "Slot Not Enabled Error" but generic CC_TRB_ERROR.

QEMU source indicates `xhci_address_slot` returns CC_TRB_ERROR at:
1. ICC check: ictl_ctx[0] != 0x0 || ictl_ctx[1] != 0x3
2. Port lookup: uport == NULL
3. Duplicate port check: xhci->slots[i].uport == uport for i != slotid-1
4. Device context pointer validation in DCBAA

### Changes
- Added `[sexusb.xhci.addr_ctx.audit.start]` section before Address Device doorbell with full context dump:
  - target_port, portsc_raw, port_ccs, slot_id, ctx_stride
  - input_ctx_phys, device_ctx_phys, dcbaa_slot_value, dcbaa_match
  - ICC dwords 0..1, Slot DW0..DW3, EP0 DW0..DW3
  - Address TRB d0..d3, trb_ptr_match, bsr_zero, spec_port, qemu_port
- Full event TRB dump on Address Device completion (d0..d3, type, cc, slot)
- Removed generic 4-entry event ring dump on timeout
- Added env-gated QEMU trace support: `SEXUSB_XHCI_TRACE=1 ./dev.sh run-nographic`
- QEMU traces: `usb_xhci_slot_address`, `usb_xhci_queue_event`, `usb_xhci_fetch_trb`

### Verification
```
./scripts/entrypoint_build.sh
SEXUSB_XHCI_TRACE=1 ./dev.sh run-nographic > /tmp/sexusb-addr-cc5-serial.log 2> /tmp/sexusb-addr-cc5-trace.log
grep -E "addr_ctx.audit|cc=5|complete|slot.ok|dcbaa_match|trb_ptr_match|bsr|spec_port|qemu_port|event.dump" /tmp/sexusb-addr-cc5-serial.log
grep -E "usb_xhci_slot_address|usb_xhci_queue_event|usb_xhci_fetch_trb" /tmp/sexusb-addr-cc5-trace.log
```

### Required answers
1. QEMU slot_address trace: which port number?
2. All audit invariants pass? (dcbaa_match=true, trb_ptr_match=true, bsr=0, spec/qemu port match target, port CCS=1)
3. Event dump: full dwords, type, cc, slot
4. Does completion still cc=5 or did something change?

### Non-goals preserved
- No 64-entry event ring dump
- No random port swapping
- No QEMU auto-slot theory chase
- No vendor op+0x38 writes
- No ERSTSZ experiment
- No doorbell value changes
- No HID/config descriptor changes
- No kernel/ABI edits

## USB_XHCI_HID_BOOT_MOUSE_CONFIG_WALK_V1

### Changes
- Added `hid_interface_number` capture from INTERFACE descriptor offset+2 (bInterfaceNumber).
- Changed marker from `[sexusb.xhci.config.hid_intf]` to `[sexusb.xhci.config.hid_boot_mouse.found]`
  with interface number logged.
- Endpoint detail logging unchanged: `[sexusb.xhci.config.intr_ep]` with addr, mps, interval.
- Existing `[sexusb.xhci.config.no_hid.park]` path preserved for non-boot-HID devices (usb-tablet).
- No new TRB submissions, commands, transfers, or IRQ changes.

### Verification
```
SEXUSB_QEMU_DEVICE=mouse SEXUSB_XHCI_TRACE=1 ./dev.sh run-nographic \
  > /tmp/sexusb-boot-mouse-config-serial.log \
  2> /tmp/sexusb-boot-mouse-config-trace.log

grep -E "desc8|desc18|config|hid|boot|mouse|endpoint|complete.ok|no_hid|timeout|cc=|fault|panic|GP|PF" \
  /tmp/sexusb-boot-mouse-config-serial.log | head -360
```

Expected:
```
[sexusb.xhci.desc8.event.ok]
[sexusb.xhci.desc8.complete.ok]
[sexusb.xhci.eval_ctx.event.ok]
[sexusb.xhci.eval_ctx.ss_mps_512.ok] port_speed=3 ss_mps=512 descriptor_bMaxPacketSize0=64
[sexusb.xhci.full18.complete.ok]
[sexusb.xhci.config.header.event.ok]
[sexusb.xhci.config.full.event.ok]
[sexusb.xhci.config.hid_boot_mouse.found] intf=0 off=9
[sexusb.xhci.config.intr_ep] off=27 addr=0x81 mps=4 interval=7
[sexusb.xhci.config.complete.ok]
```
No `[sexusb.xhci.config.no_hid.park]`, no #PF/#GP/panic.

### Result
- HID boot mouse interface found: class=0x03, subclass=0x01, protocol=0x02
- Interrupt IN endpoint: addr=0x81, MPS=4, interval=7
- Config walk parks at `loop { sys_yield(); }` after `complete.ok`
- QEMU usb-tablet path preserved via `SEXUSB_QEMU_DEVICE=tablet`

### Non-goals preserved
- No HID report descriptor fetch
- No SET_CONFIGURATION
- No Configure Endpoint command
- No interrupt transfers
- No sexinput routing
- No IRQ handler (stay with polling)

## USB_XHCI_EVALUATE_CONTEXT_MPS_AUDIT_V1

### Result
MPS=512 is valid for this configuration. Port speed ID=3 (SuperSpeed per xHCI
PSI mapping); USB 3.0 &sect;9.6.1 fixes EP0 MPS at 512 for SS ports. The controller
correctly ignores Evaluate Context MPS update requests for SS ports. The device
descriptor bMaxPacketSize0=64 is a USB 2.0 encoding that does not apply to SS
context.

### Changes
- Added `[sexusb.xhci.mps.audit]` report-only markers dumping EP0 context
  DW0..DW3 before and after Evaluate Context, with speed/descriptor/context sources.
- Added `[sexusb.xhci.eval_ctx.ss_mps_512.ok]` path: acknowledges MPS=512 is
  valid for SuperSpeed ports, replacing the false `verify.bad` marker.
- Retained `[sexusb.xhci.eval_ctx.verify.bad]` for non-SS mismatches only.

### Verification
```
SEXUSB_QEMU_DEVICE=mouse SEXUSB_XHCI_TRACE=1 ./dev.sh run-nographic \
  > /tmp/sexusb-mps-audit-serial.log \
  2> /tmp/sexusb-mps-audit-trace.log

grep -E "mps.audit|eval_ctx.ss_mps_512|verify.bad|desc8|desc18|config.complete|hid_boot_mouse" \
  /tmp/sexusb-mps-audit-serial.log | head -40
```

Expected markers:
```
[sexusb.xhci.mps.audit] port_speed=3
[sexusb.xhci.mps.audit] device_desc_bMaxPacketSize0=64
[sexusb.xhci.mps.audit] output_ep0_mps_before=512
[sexusb.xhci.mps.audit] output_ep0_mps_after=512
[sexusb.xhci.eval_ctx.ss_mps_512.ok] port_speed=3 ss_mps=512 descriptor_bMaxPacketSize0=64
```
No `[sexusb.xhci.eval_ctx.verify.bad]`.

### USB_XHCI_HID_REPORT_DESCRIPTOR_PROOF_V1
- Implemented in `servers/sexusb/src/main.rs` after config walk:
  captures `hid_interface_number` + HID `wDescriptorLength`, bounds report
  length to `1..=256`, issues EP0 `GET_DESCRIPTOR(HID_REPORT)` with
  `bmRequestType=0x81`, `bRequest=0x06`, `wValue=0x2200`,
  `wIndex=hid_interface_number`, `wLength=hid_report_desc_len`.
- Preserved transfer invariants:
  status IOC at DW3 bit5 and SETUP DW3 bit6 inline marker retained.
- Added markers:
  `[sexusb.xhci.hid.report_desc.start]`,
  `[sexusb.xhci.hid.report_desc.event.ok]`,
  `[sexusb.xhci.hid.report_desc.bytes]` (first 64 bytes max),
  `[sexusb.xhci.hid.report_desc.complete.ok]`,
  plus mouse-shape scan result marker.
- Minimal bounded shape scan only (no full HID parser): detects
  `05 01`, `09 02`, `A1 01`, `09 30`, `09 31`.

### USB_XHCI_SET_CONFIGURATION_PROOF_V1
- Implemented in `servers/sexusb/src/main.rs` directly after HID report proof.
- Uses `bConfigurationValue` read from config descriptor offset `5`.
- Issues EP0 `SET_CONFIGURATION` request with:
  `bmRequestType=0x00`, `bRequest=0x09`, `wValue=bConfigurationValue`,
  `wIndex=0`, `wLength=0`.
- Transfer shape is setup+status only (no data stage), preserving:
  SETUP DW3 bit6 inline marker and STATUS DW3 bit5 IOC.
- Bounded transfer-event wait and strict residue check (`residue==0` required).
- Added markers:
  `[sexusb.xhci.set_config.start]`,
  `[sexusb.xhci.set_config.event.ok] actual=0 residue=0`,
  `[sexusb.xhci.set_config.complete.ok]`.
- QEMU/xHCI quirk captured: no-data control transfer status stage uses `DIR=IN`.

### USB_XHCI_INTERRUPT_IN_POLL_PROOF_V1
- Implemented in `servers/sexusb/src/main.rs` after `SET_CONFIGURATION`.
- Uses config-walk captured endpoint tuple:
  `addr=0x81`, `dci=3`, `mps=4`, `interval=7`.
- Sends HID class `SET_IDLE` (`duration=1`, `report_id=0`) before poll arm
  to force periodic interrupt-IN reports on otherwise idle virtual mouse models.
- Allocates/maps:
  one interrupt-IN transfer ring and one bounded report buffer.
- Configures EP1 IN only via Input Context + Configure Endpoint command:
  ICC Add flags include Slot bit `0` and endpoint context bit `3`.
- Endpoint context values used:
  `EP Type=Interrupt IN`, `CErr=3`, `Max Packet Size=4`,
  `TR Dequeue=intr_ring_phys|DCS=1`, `Avg TRB Len=4`,
  `Max ESIT Payload=4`.
- Queues one Normal TRB (`len=4`, IOC=1), rings DB target `3`,
  bounded-polls one Transfer Event, logs raw 4-byte report only.
- Added markers:
  `[sexusb.xhci.intr_in.config_ep.start]`,
  `[sexusb.xhci.intr_in.config_ep.ok]`,
  `[sexusb.xhci.intr_in.poll.start]`,
  `[sexusb.xhci.intr_in.event.ok] actual=N residue=N`,
  `[sexusb.xhci.intr_in.report.bytes]`,
  `[sexusb.xhci.intr_in.poll.complete.ok]`.

### USB_HID_MOUSE_LOCAL_DECODE_PROOF_V1
- Local-only decode in `servers/sexusb/src/main.rs`; no PDX send.
- Added bounded helper:
  `decode_boot_mouse_report(buf: &[u8], len: usize) -> Option<BootMouseReport>`.
- Decode contract:
  `buttons=byte0`, `dx=byte1 as i8`, `dy=byte2 as i8`,
  `wheel=byte3 as i8 when len>=4 else 0`.
- Validation:
  reject `len < 3`, accept `len >= 3`, no allocation.
- Added markers:
  `[sexusb.hid.mouse.decode.start]`,
  `[sexusb.hid.mouse.decode.ok]`,
  `[sexusb.hid.mouse.decode.zero.ok]`,
  `[sexusb.hid.mouse.decode.bad]`.

### USB_SEXUSB_TO_SEXINPUT_CAP_ROUTE_V1
- Minimal capability route enabled in `kernel/src/init.rs`:
  grant `sexusb -> sexinput` one domain capability at slot `9`.
- `sexusb` sends one decoded boot-mouse report via PDX:
  `type_id/op=0x260`, `arg1=buttons`, `arg2=packed(dx,dy,wheel)`.
- `sexinput` receives on domain listen path (`slot 0`) and logs decode proof only.
- No cross-PD pointers; integer packing only.
- Markers:
  `sexusb`: `[sexusb.hid.mouse.pdx_send.start]`,
  `[sexusb.hid.mouse.pdx_send.ok]`,
  `[sexusb.hid.mouse.pdx_send.fail]`.
  `sexinput`: `[sexinput.usb_mouse.recv]`,
  `[sexinput.usb_mouse.decode.ok]`.

### SEXINPUT_USB_MOUSE_NORMALIZER_TO_SHELL_PROOF_V1
- `sexinput` now converts received USB mouse report to existing normalized HID
  event shape using `normalize_pointer_report_v1`.
- `sexinput` emits proof markers for normalize/send stages:
  `[sexinput.usb_mouse.normalize.start]`,
  `[sexinput.usb_mouse.normalize.ok]`,
  `[sexinput.usb_mouse.shell_send.start]`,
  `[sexinput.usb_mouse.shell_send.ok]`,
  `[sexinput.usb_mouse.shell_send.fail] err=N`.
- `sexinput` forwards normalized events over the existing shell PDX path and
  adds a proof tap (`op=0x260`) for shell-side decode logging.
- `silk-shell` logs receive/decode markers:
  `[shell.recv.usb_mouse]`,
  `[shell.recv.usb_mouse.decode.ok]`.

### SILK_SHELL_USB_MOUSE_RECEIVE_UNPARK_PROOF_V1
- Shell receive-path block was from intentional containment park before
  `pdx_listen_raw(0)` (`spin_loop(); continue;`).
- Added a minimal local gate in `silk-shell` to keep park logic available while
  allowing proof-mode receive loop execution.
- No kernel/ABI/capability/display/policy changes in this step.

### SILK_SHELL_USB_MOUSE_POINTER_STATE_PROOF_V1
- In `silk-shell` USB report receive branch (`op=0x260`), local USB pointer
  state is now maintained and logged:
  `x: i32`, `y: i32`, `buttons: u8`, `wheel_accum: i32`.
- First USB report initializes pointer to desktop center (`P.width/2`,
  `P.height/2`), then applies clamped/saturating `dx/dy` updates.
- Markers:
  `[shell.pointer.usb_state.start]`,
  `[shell.pointer.usb_state.ok] x=N y=N buttons=0x.. wheel=N`.
- No sexdisplay/framebuffer/compositor/focus/drag policy changes in this step.

### Non-goals preserved
- No sexinput routing
- No IRQ handler (stay with polling)

### Next
- Promote from one-shot poll proof to stable repeated polling policy (still no IRQ path).
