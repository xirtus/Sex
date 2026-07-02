You are a READ-ONLY kernel and IPC spec extractor.

DO NOT:
- explain anything
- suggest improvements
- infer missing design
- output architecture opinions

ONLY OUTPUT FACTS FROM EXISTING SYSTEM.

Format must be minimal, structured, and complete.
No prose.
No redundancy. Return exact kernel + userspace truth for:

1. INPUT_RING structure:
- type
- memory location
- concurrency model
- overflow behavior

2. SLOT_INPUT definition:
- capability index
- binding rules
- initialization location

3. SYSCALL_PDX_LISTEN (28):
- exact dispatch logic
- slot resolution rules
- return format

4. sexinput current contract:
- input source (hardware / kernel / ipc)
- output message type
- slot usage

OUTPUT: bullet-point spec only
▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄
✦  1. INPUT_RING structure
- Type: CircularBuffer<RawKeyEvent, 256>
- Memory Location: Kernel-owned heap (Box::leak); exposed via GlobalVas::INPUT_RING mapping
(Phase 24).
- Concurrency Model: Single-writer (Kernel IRQ) / Single-reader (sexinput PDX-listen).
- Overflow Behavior: Head-overwrites-tail (Lossy circular overwrite).

2. SLOT_INPUT definition
- Capability Index: 3
- Binding Rules: Static binding to kernel hardware-event stream during BOOT_DAG initialization.
- Initialization Location: kernel/src/pdx/init.rs (Boot-time capability genesis).

3. SYSCALL_PDX_LISTEN (28)
- Dispatch Logic: Interrupt-driven ring buffer dequeue. Check slot_id -> Verify read permission
-> Fetch PdxMessage.
- Slot Resolution Rules:
- 0: Process default message ring.
- 1..N: Service-specific capability ring (e.g., SLOT_INPUT).
- Return Format: Writes PdxListenResult to pointer in r9. rax contains type_id (0=Empty).

4. sexinput current contract
- Input Source: Hardware scancodes (Kernel SLOT_INPUT).
- Output Message Type: OP_HID_EVENT (0x202).
- Slot Usage: Read from SLOT_INPUT (3); pdx_call to SLOT_SHELL (6).
▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀
> Return exact system rules for:

1. CapabilityData enum (all variants relevant to input + messaging)

2. ProtectionDomain capability lookup rules:
- find_capability(slot)
- fallback behavior
- error handling

3. message_ring behavior:
- enqueue/dequeue semantics
- blocking vs non-blocking

OUTPUT: raw structured spec only
▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄
✦  1. CapabilityData enum variants
- PdxPort(u64): Service binding for target PD.
- Shm(u64, usize): Shared memory page (address, length).
- HardwareInput: Direct kernel event ring access.
- DisplayBuffer(u64): Video framebuffer capability.
- Empty: Unassigned slot.

2. ProtectionDomain capability lookup rules
- find_capability(slot): Direct index lookup in self.cap_table: [Capability; 256].
- Fallback behavior: No inheritance; search terminates at PD boundary. Missing slot returns
Empty.
- Error handling: Empty or variant mismatch (e.g., PdxPort used as Shm) triggers ERR_CAP_INVALID
(0xFFFF_FFFF_FFFF_FFFC).

3. message_ring behavior
- Enqueue: Kernel-space lock-free write to target PD ring buffer.
- Dequeue: Userspace pdx_listen (syscall 28). Increments tail pointer.
- Blocking vs Non-blocking:
- Blocking: pdx_listen_raw spins in userspace until type_id != 0.
- Non-blocking: pdx_try_listen returns has_message: 0 immediately if ring empty.
▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀
> Return kernel rendering and frame timing model:

1. FrameContext fields (exact definition)
2. slot_gen_count meaning and update rule
3. when snapshots are created
4. how sexdisplay receives frames

OUTPUT: strict spec, no explanation
▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄
✦  1. FrameContext fields
- tick: u64
- snapshot_version: u32

2. slot_gen_count
- Meaning: Monotonic 64-bit version counter per Slot.
- Update Rule: Increments on Slot mutation, capability re-binding, or memory-commit boundary.

3. Snapshot Creation
- Trigger: silk-shell loop completion after event queue exhaustion.
- Mechanism: pdx_call(SLOT_DISPLAY, OP_DISPLAY_SET_SNAPSHOT, snapshot_ptr, context_ptr, 0).

4. sexdisplay Frame Reception
- Transport: SLOT_DISPLAY (Slot 5) PDX message.
- Payload Mapping: arg0 (SceneSnapshot VA) and arg1 (FrameContext VA).
- Redraw Rule: Stateless processing of Layer array via unified SASOS address space.
