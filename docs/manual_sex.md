# SexOS System Developer Manual

**Version:** Phase 21 (Userland Handoff)
**Kernel:** sex-kernel v0.1.0
**Bootloader:** Limine 7.13.3
**Architecture:** x86_64 (no\_std, pure Rust)

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Repository Layout](#2-repository-layout)
3. [Boot Flow — Limine 7.x Protocol](#3-boot-flow--limine-7x-protocol)
4. [Kernel Init Sequence](#4-kernel-init-sequence)
5. [Hardware Abstraction Layer (HAL)](#5-hardware-abstraction-layer-hal)
6. [Memory Subsystem](#6-memory-subsystem)
7. [Protection Keys (PKU / Intel MPK)](#7-protection-keys-pku--intel-mpk)
8. [Protection Domains](#8-protection-domains)
9. [ELF Loader](#9-elf-loader)
10. [Ring-3 Handoff (IRETQ)](#10-ring-3-handoff-iretq)
11. [IPC System](#11-ipc-system)
12. [Display Server — sexdisplay](#12-display-server--sexdisplay)
13. [Build System](#13-build-system)
14. [QEMU Testing](#14-qemu-testing)
15. [Known Issues & Phase Status](#15-known-issues--phase-status)
16. [Critical File Index](#16-critical-file-index)

---

## 1. System Overview

SexOS is a **Single Address Space Operating System (SASOS)** built entirely in Rust (`no_std`). All kernel and userland processes share one virtual address space. Isolation between processes is enforced by **Intel Memory Protection Keys (PKU/MPK)** rather than separate page tables.

### Design Principles

| Principle | Mechanism |
|-----------|-----------|
| Isolation without page-table switching | Intel PKU — each PD gets a unique PKEY (1–15) |
| Zero kernel/userland address-space boundary | Single PML4 shared by all |
| Lock-free IPC | Per-PD atomic ring buffers (`RingBuffer<MessageType, 256>`) |
| No dynamic kernel modules | All drivers compile into server ELFs, loaded at boot |
| Capability-based access control | `CapabilityTable` per PD, CHERI-style bounds |

### Protection Key Model

```
PKEY 0  — kernel + default (no restriction)
PKEY 1  — sexdisplay / framebuffer (trusted display domain)
PKEY 2+ — future userland PDs
```

PKRU bits per key: `[AD, WD]` — Access Disabled, Write Disabled.
Ring-0 code ignores PKU. Ring-3 code is restricted by its thread's PKRU register.

---

## 2. Repository Layout

```
microkernel/
├── kernel/                  # sex-kernel crate (no_std binary + lib)
│   ├── src/
│   │   ├── main.rs          # _start(), panic handler
│   │   ├── lib.rs           # kernel_init(), Limine requests, global allocator
│   │   ├── init.rs          # PD bootstrap, pdx_spawn(), jump_to_userland()
│   │   ├── hal/
│   │   │   ├── mod.rs       # HardwareAbstractionLayer trait, hal::init()
│   │   │   └── x86_64.rs    # X86Hal impl — GDT/IDT/PKU init
│   │   ├── memory/
│   │   │   ├── mod.rs       # re-exports allocator, pku, manager
│   │   │   ├── manager.rs   # memory::manager::init(), GLOBAL_VAS, BitmapFrameAllocator
│   │   │   ├── allocator.rs # LockFreeBuddyAllocator, PageMetadata
│   │   │   └── pku.rs       # wrpkru(), tag_virtual_address(), init_pku_isolation()
│   │   ├── gdt.rs           # GDT, TSS, Selectors struct, get_selectors()
│   │   ├── interrupts.rs    # IDT, init_idt(), syscall_entry, page_fault_handler
│   │   ├── elf.rs           # load_elf_for_pd() — minimal ELF64 parser
│   │   ├── ipc.rs           # DOMAIN_REGISTRY, DomainRegistry, pdx_call_with_mask()
│   │   ├── ipc_ring.rs      # RingBuffer<T, N> — lock-free SPSC ring
│   │   ├── capability.rs    # ProtectionDomain, CapabilityTable, CapabilityData
│   │   ├── pku.rs           # is_pku_supported(), enable_pku(), Pkru struct
│   │   ├── scheduler.rs     # Task, SCHEDULERS
│   │   ├── graphics/
│   │   │   └── handoff.rs   # ship_to_sexdisplay() — framebuffer IPC
│   │   ├── pd/
│   │   │   └── create.rs    # create_protection_domain() (currently unused)
│   │   ├── apic.rs          # LAPIC, send_ipi()
│   │   ├── smp.rs           # AP bootstrap
│   │   └── serial.rs        # serial_println! macro
│   └── linker.ld            # Higher-half linker script
├── servers/                 # All ring-3 protection-domain servers
│   ├── sexc/                # Core execution / central services server
│   ├── sexdisplay/          # Display compositor server (ring-3 ELF) — completed
│   │   ├── src/
│   │   │   ├── main.rs      # _start(), main event loop, PDX dispatch
│   │   │   └── lib.rs       # SexCompositor, Window, OutputState, PDX syscall IDs
│   │   └── Cargo.toml
│   ├── sexdrive/            # Block-device / storage driver server
│   ├── sexfiles/            # Filesystem / VFS server (pairs with linen file manager)
│   ├── sexgemini/           # Gemini protocol / lightweight comms server
│   ├── sexinput/            # Input subsystem server (keyboard, mouse, touch, etc.)
│   ├── sexnet/              # Full networking stack server
│   ├── sexnode/             # Device/node management & discovery server
│   ├── sexstore/            # Persistent object / data store server
│   ├── sext/                # Terminal / text I/O / console server
│   └── tranny/              # IPC translator / compatibility bridge (was sextranny)
├── apps/                    # Userland applications (linen file manager, silk desktop environment + full COSMIC/Redox app suite, kaleidoscope, qupid, etc.)
├── crates/                  # Shared crates (silkbar, silknet, tatami, silk-client)
├── sex-rt/                  # Runtime library for ring-3 processes
│   └── src/lib.rs
├── tools/sex-forge/         # Project scaffolding tool
├── limine/                  # Limine 7.13.3 bootloader binaries + header
├── limine.cfg               # Bootloader configuration
├── build_payload.sh         # Master build script (kernel + all servers → iso_root/)
├── Makefile                 # iso, run-sasos, clean targets
└── x86_64-sex.json          # Custom Rust target spec (no_std, higher-half)
```

---

## 3. Boot Flow — Limine 7.x Protocol

### Limine 7.x Request Requirements

Limine 7.13.3 requires three mandatory objects in the kernel binary:

```rust
// kernel/src/lib.rs

#[used]
#[link_section = ".limine_requests_start"]
static _LIMINE_START: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[link_section = ".limine_requests"]
static _LIMINE_BASE: BaseRevision = BaseRevision::new();

// ... all Request statics must be in ".limine_requests" section ...

#[used]
#[link_section = ".limine_requests_end"]
static _LIMINE_END: RequestsEndMarker = RequestsEndMarker::new();
```

**Without these markers, Limine 7.x misbehaves** — it may double-scan the binary and report false "Conflict detected for request ID" panics.

### Linker Script Requirements

All Limine sections must be inside a **single PT_LOAD segment** (same ELF permissions). Splitting them across RO/RW segments causes `"PHDRs with different permissions sharing the same memory page"` panic.

```ld
/* kernel/linker.ld */
.data : ALIGN(4K) {
    KEEP(*(.limine_requests_start))   /* start marker    */
    KEEP(*(.limine_requests))         /* base + requests */
    KEEP(*(.limine_requests_end))     /* end marker      */
    *(.data .data.*)
}
```

### Active Limine Requests

| Static | Type | Location | Purpose |
|--------|------|----------|---------|
| `_LIMINE_BASE` | `BaseRevision` | `lib.rs` | Protocol negotiation |
| `FB_REQUEST` | `FramebufferRequest` | `lib.rs` | Framebuffer info |
| `MEMMAP_REQUEST` | `MemmapRequest` | `lib.rs` | Physical memory map |
| `HHDM_REQUEST` | `HhdmRequest` | `lib.rs` | HHDM offset |
| `MODULE_REQUEST` | `ModulesRequest` | `init.rs` | Boot modules (sexdisplay ELF) |

**Critical:** Never define two statics of the same Limine request type. Limine sees the request magic bytes twice and panics.

### limine.cfg

```
TIMEOUT=1

:SexOS
    PROTOCOL=limine
    KERNEL_PATH=boot:///sexos-kernel
    MODULE_PATH=boot:///servers/sexdisplay
    VIDEO_MODE=1280x720,32
```

Module paths must point to files that **actually exist in the ISO**. Missing files cause `"Failed to open module with path"` panic at boot.

### ISO Directory Layout

```
iso_root/
├── sexos-kernel           ← kernel ELF (from target/x86_64-sex/release/sex-kernel)
├── servers/
│   └── sexdisplay         ← display server ELF (statically linked, ring-3)
├── limine-bios.sys
├── limine-bios-cd.bin
├── limine-uefi-cd.bin
└── limine.cfg
```

---

## 4. Kernel Init Sequence

### Entry Point

```
Limine
  └→ _start()                          kernel/src/main.rs:7
       └→ kernel_init()                kernel/src/lib.rs:61
            ├→ hal::init()             kernel/src/hal/mod.rs:15
            │    └→ X86Hal::init()     kernel/src/hal/x86_64.rs:39
            │         ├→ CR3 read (SAS page table sync)
            │         ├→ pku::enable_pku() → CR4.PKE = 1
            │         ├→ gdt::init()   (GDT load, CS/TSS set)
            │         └→ interrupts::init_idt() (IDT + LSTAR/STAR/SFMASK/EFER)
            ├→ memory::manager::init(mmap, hhdm_offset)
            │    ├→ init_sexting()     (OffsetPageTable from CR3)
            │    ├→ BitmapFrameAllocator::init()
            │    ├→ allocator::init_heap()  (16 MiB at HEAP_START=0x4444_4444_0000)
            │    ├→ GLOBAL_ALLOCATOR.init_metadata()
            │    └→ GLOBAL_VAS = Some(GlobalVas { mapper, frame_allocator, offset })
            ├→ init::init()            kernel/src/init.rs:14
            │    ├→ MODULE_REQUEST.response() → iterate Limine modules
            │    ├→ pdx_spawn("sexdisplay", module_data)
            │    │    ├→ load_elf_for_pd(data, &mut GLOBAL_VAS, pkey=1)
            │    │    ├→ ProtectionDomain::new(id=1, pku_key=1)
            │    │    ├→ DOMAIN_REGISTRY.insert(1, pd_ptr)
            │    │    └→ store entry_point in pd.main_task
            │    └→ wrpkru(0b1100)    (sets PKEY1 restricted in PKRU — no effect in ring-0)
            └→ [ring-3 handoff]
                 └→ init::jump_to_userland(entry, pd.base_pkru_mask)
                      ├→ wrpkru(0xFFFFFFF3)  (PKEY1 allowed for sexdisplay)
                      └→ IRETQ → sexdisplay _start() in ring 3
```

### Expected Serial Output (Healthy Boot)

```
X86Hal: Initializing foundation (BSP)...
X86Hal: SAS Page Tables ready (CR3 = 0x...)
PKU: Protection Keys enabled in CR4.
X86Hal: Initializing GDT/IDT...
Sex: Memory init starting...
Sex: Initializing kernel heap...
Sex: Initializing buddy allocator...
Sex: Memory init complete.
init: Bootstrapping system Protection Domains...
Found 1 modules from Limine
✓ Found userland server: boot:///servers/sexdisplay
ELF: Valid header. Entry point: 0x...
ELF: Loading segment: vaddr=0x..., memsz=0x... (Key: 1)
PDX: boot:///servers/sexdisplay entry=0x... PKEY=1
PDX: Registered PD 1 in DOMAIN_REGISTRY (PKEY 1)
   → Spawning PD: boot:///servers/sexdisplay @ 0x... (N bytes) -> ID 1
init: Revoking kernel write access via PKU (Phase 18.5 complete)
init: All Protection Domains bootstrapped — handing off to sexdisplay + Silk
kernel: Handing off to sexdisplay @ 0x... (ring 3)
```

---

## 5. Hardware Abstraction Layer (HAL)

### Files

- `kernel/src/hal/mod.rs` — `HardwareAbstractionLayer` trait, `hal::init()`, `HAL` static
- `kernel/src/hal/x86_64.rs` — `X86Hal` struct implementing the trait
- `kernel/src/gdt.rs` — GDT, TSS, `Selectors` struct
- `kernel/src/interrupts.rs` — IDT, interrupt handlers, `init_idt()`
- `kernel/src/pku.rs` — `enable_pku()`, `is_pku_supported()`, `Pkru` struct

### GDT Layout

```
Index 0: null descriptor
Index 1: kernel_code_segment (RPL=0)   ← code_selector
Index 2: TSS (low 64 bits)             ← tss_selector  (2 slots for 64-bit TSS)
Index 3: TSS (high 64 bits)
Index 4: user_data_segment (RPL=3)     ← user_data_selector
Index 5: user_code_segment (RPL=3)     ← user_code_selector
```

`gdt::get_selectors()` returns the `Selectors` struct with all four selectors.

**Known Issue:** No explicit kernel data segment (index 2 is TSS, not kernel data). `Star::write()` currently receives `user_data_selector` (RPL=3) as the kernel_ss parameter, which causes `Star::write().unwrap()` to panic. See §15.

### IDT Handlers

| Vector | Handler | Source |
|--------|---------|--------|
| #BP (breakpoint) | `breakpoint_handler` | `interrupts.rs` |
| #DF (double fault) | `double_fault_handler` (IST 0) | `interrupts.rs` |
| #PF (page fault) | `page_fault_handler` | `interrupts.rs` |
| 0x20 (LAPIC timer) | `timer_interrupt_handler` | `interrupts.rs` |
| 0x21 (keyboard) | `keyboard_interrupt_handler` | `interrupts.rs` |
| 0x22 | `generic_irq_handler` | `interrupts.rs` |
| 0x40 (IPI revoke) | `revoke_key_handler` | `interrupts.rs` |
| SYSCALL (LSTAR) | `syscall_entry` | `interrupts.rs` |

### PKU Initialization

1. `pku::is_pku_supported()` — CPUID leaf 7, EBX bit 3
2. `pku::enable_pku()` — sets `CR4.PKE` (bit 21)
3. `pku::wrpkru(val)` — writes PKRU register via `WRPKRU` instruction

**WRPKRU requires CR4.PKE=1.** Calling it before `hal::init()` causes `#UD` → triple fault.

---

## 6. Memory Subsystem

### Files

- `kernel/src/memory/manager.rs` — `init()`, `GLOBAL_VAS`, `BitmapFrameAllocator`, `update_page_pkey()`
- `kernel/src/memory/allocator.rs` — `LockFreeBuddyAllocator`, `PageMetadata`
- `kernel/src/memory/pku.rs` — `wrpkru()`, `tag_virtual_address()`
- `kernel/src/lib.rs` — `ALLOCATOR: LockedHeap`, `HEAP_START`, `HEAP_SIZE`

### Address Space Map

```
0xffffffff80200000  ← KERNEL_LMA (kernel load address = HHDM_BASE + 0x200000)
0xffffffff80200000  ← .text
0xffffffff80206000  ← .rodata
0xffffffff80207000  ← .data (includes Limine requests)
0xffffffff80288xxx  ← .bss
0xffff800000000000  ← HHDM_BASE (typical; actual from HHDM_REQUEST.response())
0x4444_4444_0000    ← HEAP_START (16 MiB, linked_list_allocator)
```

### Initialization Order

**Must be called in this order or allocations crash:**

```rust
// 1. hal::init()         — must be first (IDT, PKU)
// 2. memory::manager::init(mmap, hhdm.offset)
//      → init_sexting()  — builds OffsetPageTable from CR3
//      → BitmapFrameAllocator::init(entries, offset)
//      → allocator::init_heap(&mut mapper, &mut frame_allocator)
//         → ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE)
//      → GLOBAL_ALLOCATOR.init_metadata(metadata_vaddr, total_pages)
//      → GLOBAL_VAS.lock() = Some(GlobalVas { ... })
// 3. init::init()        — can now Box::new(), allocate, map pages
```

### Global VAS

```rust
pub static ref GLOBAL_VAS: Mutex<Option<GlobalVas>> = Mutex::new(None);

pub struct GlobalVas {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: BitmapFrameAllocator,
    pub phys_mem_offset: VirtAddr,
}
```

`GlobalVas::map_pku_range(vaddr, size, flags, pku_key)` — maps pages with PKU tag. Used by ELF loader to map userland segments.

### Physical Frame Allocator

`BitmapFrameAllocator` wraps `BootInfoFrameAllocator` which iterates Limine's memory map entries (type `MEMMAP_USABLE`) sequentially. Not a bitmap in practice — it's a simple counter through usable frames.

### Heap

`linked_list_allocator::LockedHeap` — 16 MiB at virtual address `0x4444_4444_0000`. Maps 16 MiB of contiguous physical frames at boot.

---

## 7. Protection Keys (PKU / Intel MPK)

### Background

Intel Memory Protection Keys for User-space (PKU) allows tagging each 4KB page table entry with a 4-bit key (0–15). A per-thread `PKRU` register (32 bits, 2 bits per key) controls whether ring-3 accesses to pages with that key are allowed.

```
PKRU bits for key N:  [bit 2N+1 = WD, bit 2N = AD]
AD=1 → access disabled (no reads or writes from ring 3)
WD=1 → write disabled (no writes from ring 3, reads OK)
AD=0, WD=0 → full access
```

PKU **does not restrict ring-0 (kernel) access**.

### Page Tagging

PKU key is stored in bits [62:59] of the page table Level-1 entry (PTE).

```rust
// kernel/src/memory/manager.rs
pub fn update_page_pkey(page: Page, pku_key: u8, phys_mem_offset: VirtAddr) {
    // Walks PML4 → PDPT → PD → PT manually
    // Sets bits [62:59] of the PTE
    // Runs INVLPG
}
```

Also available: `memory::pku::tag_virtual_address(va, pkey)` in `kernel/src/memory/pku.rs`.

### PKRU Values

| Domain | PKRU | Meaning |
|--------|------|---------|
| Kernel (ring-0) | any | PKU not enforced |
| sexdisplay (PKEY 1) | `0xFFFFFFF3` | PKEY 0 denied, PKEY 1 allowed, all others denied |
| Default ring-3 | `0xFFFFFFFF` | all keys denied |

`ProtectionDomain::new(id, pku_key)` computes `base_pkru_mask`:
```rust
let mut pkru_mask: u32 = 0xFFFF_FFFF;  // deny all by default
let shift = pku_key * 2;
pkru_mask &= !(0b11 << shift);          // allow this PD's own key
```

---

## 8. Protection Domains

### Struct

```rust
// kernel/src/capability.rs
pub struct ProtectionDomain {
    pub id: u32,
    pub pku_key: u8,
    pub base_pkru_mask: u32,
    pub current_pkru_mask: AtomicU32,
    pub cap_table: *mut CapabilityTable,
    pub signal_handlers: AtomicPtr<BTreeMap<i32, u64>>,
    pub signal_ring: *mut RingBuffer<u8, 32>,
    pub message_ring: *mut RingBuffer<MessageType, 256>,
    pub main_task: AtomicPtr<crate::scheduler::Task>,
    pub trampoline_task: AtomicPtr<crate::scheduler::Task>,
}
```

In Phase 21: `main_task` is repurposed as an entry point carrier until the scheduler is wired. The entry point VirtAddr is stored as a raw `*mut Task` pointer.

### Registry

```rust
// kernel/src/ipc.rs
pub static DOMAIN_REGISTRY: DomainRegistry = DomainRegistry::new();

pub struct DomainRegistry {
    pub domains: [AtomicPtr<ProtectionDomain>; 1024],
}
```

`DOMAIN_REGISTRY.insert(id, ptr)` — O(1), lock-free, uses `id % 1024` as index.
`DOMAIN_REGISTRY.get(id)` — returns `Option<&'static ProtectionDomain>`.

### Creating a PD (Phase 21 path)

```rust
// kernel/src/init.rs — pdx_spawn()
fn pdx_spawn(name: &str, module_data: &[u8]) -> Result<u32, &'static str> {
    // 1. Load ELF into SASOS VAS with PKEY 1
    let entry = load_elf_for_pd(module_data, &mut GLOBAL_VAS.lock(), pku_key=1)?;
    // 2. Allocate PD struct on heap
    let pd = Box::new(ProtectionDomain::new(pd_id, pku_key=1));
    // 3. Store entry point in main_task (temporary)
    pd.main_task.store(entry.as_u64() as *mut _, Ordering::Release);
    // 4. Register
    DOMAIN_REGISTRY.insert(pd_id, Box::into_raw(pd));
}
```

---

## 9. ELF Loader

### File

`kernel/src/elf.rs`

### Function

```rust
pub fn load_elf_for_pd(
    elf_data: &[u8],
    vas: &mut GlobalVas,
    pku_key: u8,
) -> Result<VirtAddr, &'static str>
```

### Process

1. Validates ELF magic (`[0x7f, 'E', 'L', 'F']`)
2. Iterates `PT_LOAD` program headers
3. For each segment: calls `GlobalVas::map_pku_range(vaddr, memsz, flags, pku_key)` — allocates frames, maps pages with PKU tag
4. Copies segment data from `elf_data[offset..offset+filesz]` into mapped virtual address
5. Zero-fills BSS (`memsz > filesz`)
6. Returns `VirtAddr::new(header.entry)`

**Requirements:** `GLOBAL_VAS` must be initialized (`memory::manager::init()` must have run).

### ELF Struct Definitions

```rust
// kernel/src/elf.rs
pub struct ElfHeader { /* standard ELF64 header */ }
pub struct ProgramHeader { /* ELF64 Phdr */ }

pub const PT_LOAD: u32 = 1;
pub const PF_X: u32 = 1;   // execute
pub const PF_W: u32 = 2;   // write
pub const PF_R: u32 = 4;   // read
```

---

## 10. Ring-3 Handoff (IRETQ)

### Function

```rust
// kernel/src/init.rs
pub unsafe fn jump_to_userland(entry: u64, pkru: u32) -> !
```

### Mechanics

1. Allocates 16 KiB ring-3 stack on heap, `core::mem::forget`s it (keeps alive)
2. Calls `memory::pku::wrpkru(pkru)` — sets PKRU for the ring-3 execution context
3. Reads user CS and SS from `gdt::get_selectors()`
4. Pushes IRETQ frame: `[SS, RSP, RFLAGS=0x202, CS, RIP]`
5. Executes `IRETQ` — CPU transitions to ring 3 at `entry`

```rust
core::arch::asm!(
    "push {ss}",
    "push {rsp_val}",
    "push {rflags}",    // 0x202 = IF=1
    "push {cs}",
    "push {rip}",
    "iretq",
    ...
    options(noreturn)
);
```

### Calling Site

```rust
// kernel/src/lib.rs — kernel_init()
let pd_id = init::SEXDISPLAY_PD_ID;
if pd_id != 0 {
    if let Some(pd) = ipc::DOMAIN_REGISTRY.get(pd_id) {
        let entry = pd.main_task.load(Ordering::Acquire) as u64;
        if entry != 0 {
            init::jump_to_userland(entry, pd.base_pkru_mask);
        }
    }
}
```

---

## 11. IPC System

### Message Ring

Each PD has a `*mut RingBuffer<MessageType, 256>` at `pd.message_ring`.

```rust
// kernel/src/ipc_ring.rs
pub struct RingBuffer<T, const N: usize> {
    buf: UnsafeCell<[MaybeUninit<T>; N]>,
    read: AtomicUsize,
    write: AtomicUsize,
}

impl<T, const N: usize> RingBuffer<T, N> {
    pub fn enqueue(&self, val: T) -> Result<(), T>
    pub fn dequeue(&self) -> Option<T>
}
```

Lock-free SPSC ring. `N=256` messages per PD.

### Message Types

```rust
// kernel/src/ipc/messages.rs
pub enum MessageType {
    RawCall(u64),
    DisplayPrimaryFramebuffer { virt_addr, width, height, pitch },
    HIDEvent { ev_type, code, value },
    // ... more
}
```

### PDX Primitives (kernel side)

```rust
// kernel/src/ipc.rs
pub unsafe fn pdx_call_with_mask(
    target_pkru: u32,
    entry_point: VirtAddr,
    arg0: u64,
) -> u64
```

Temporarily sets PKRU, calls target, restores PKRU. Used for cross-PD calls.

### Syscall Entry

`LSTAR` MSR points to `syscall_entry` in `interrupts.rs`. Syscalls are dispatched by number in the syscall handler. Key syscalls:

| Number | Name | Purpose |
|--------|------|---------|
| 28 | pdx_listen | Block until message arrives on this PD's ring |
| 30 | MAP_MEMORY | Map physical memory into SASOS |
| 31 | ALLOCATE_MEMORY | Allocate anonymous pages |

### Framebuffer Handoff

```rust
// kernel/src/graphics/handoff.rs
pub fn ship_to_sexdisplay(fb: &Framebuffer, hhdm: u64)
```

1. Creates `MessageType::DisplayPrimaryFramebuffer` with framebuffer virt addr + dimensions
2. Calls `update_page_pkey()` on all FB pages → tags with `SEXDISPLAY_PD_ID`'s PKEY
3. Enqueues message into `pd.message_ring`
4. Unparks sexdisplay's trampoline task if sleeping

**Fails silently** if `DOMAIN_REGISTRY.get(SEXDISPLAY_PD_ID)` returns `None` (PD not registered).

---

## 12. Display Server — sexdisplay

### File

`servers/sexdisplay/src/main.rs`, `servers/sexdisplay/src/lib.rs`

### Entry Point

```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    let compositor = SexCompositor::new(global_width, global_height, stride);
    loop {
        let request = pdx_listen(0);   // block for next PDX message
        match message.msg_type {
            MessageType::RawCall(syscall_id) => { compositor.handle_pdx_call(...) }
            MessageType::HIDEvent { ev_type, code, value } => { ... forward to windows ... }
            _ => { /* unknown */ }
        }
        compositor.evaluate_and_render_frame();
        // commit each output to hardware
        for output in &compositor.outputs {
            pdx_call(0, PDX_COMPOSITOR_COMMIT, ...);
        }
    }
}
```

### SexCompositor

```rust
// servers/sexdisplay/src/lib.rs
pub struct SexCompositor {
    pub magic: u32,                     // 0x53455843 ('SEXC')
    pub global_fb_width: u32,
    pub global_fb_height: u32,
    pub outputs: Vec<OutputState>,
    pub internal_framebuffers: Vec<WindowBuffer>,
    pub all_windows: [Option<Window>; MAX_WINDOWS],  // 64 max
    pub current_tag_view_mask: u64,
    pub views: Vec<u64>,
    pub registered_hotkeys: Vec<Hotkey>,
    pub active_notifications: Vec<Notification>,
}
```

### PDX Syscall IDs (sexdisplay protocol)

| Constant | Value | Purpose |
|----------|-------|---------|
| `PDX_COMPOSITOR_COMMIT` | 0xDD | Commit framebuffer to display output |
| `PDX_SEX_WINDOW_CREATE` | 0xDE | Create new window |
| `PDX_SET_WINDOW_DECORATIONS` | 0xE2 | Set border/title/styling |
| `PDX_GET_DISPLAY_INFO` | 0xE3 | Query display resolution |
| `PDX_FOCUS_WINDOW` | 0xE4 | Set keyboard focus |
| `PDX_MOVE_WINDOW` | 0xEE | Move window |
| `PDX_RESIZE_WINDOW` | 0xEF | Resize window |

---

## 13. Build System

### Full Build (kernel + sexdisplay + ISO)

```bash
bash build_payload.sh   # compile release binaries, stage to iso_root/
make iso                # xorriso → sexos-v1.0.0.iso + Limine BIOS install
```

### `build_payload.sh` — What It Does

```bash
# 1. Compile kernel (release, custom target, custom linker script)
RUSTFLAGS="-C link-arg=-Tkernel/linker.ld" cargo build \
    -Z build-std=core,compiler_builtins,alloc \
    -Z build-std-features=compiler-builtins-mem \
    --package sex-kernel \
    --target x86_64-sex.json \
    --release

# 2. Compile sexdisplay (release, same target, no kernel linker script)
RUSTFLAGS="" cargo build \
    --manifest-path servers/sexdisplay/Cargo.toml \
    --target x86_64-sex.json \
    --release

# 3. Stage binaries
cp target/x86_64-sex/release/sex-kernel   iso_root/sexos-kernel
cp target/x86_64-sex/release/sexdisplay   iso_root/servers/
```

**Important:** `make iso` does NOT recompile. Always run `build_payload.sh` first after source changes, then `make iso`.

### Custom Target: `x86_64-sex.json`

Located at repo root. Key settings:
- `"os": "none"` — bare metal
- `"linker-flavor": "gnu-lld"` (or similar)
- Higher-half addressing enabled

### Makefile Targets

| Target | Action |
|--------|--------|
| `make iso` | Package `iso_root/` into `sexos-v1.0.0.iso`, run `limine-install` |
| `make run-sasos` | Launch QEMU with PKU + serial |
| `make clean` | Remove ISO and `iso_root/` |
| `make limine-install` | Build the `limine-install` C tool from `limine.c` |

---

## 14. QEMU Testing

### Standard Boot Command

```bash
qemu-system-x86_64 \
    -M q35 \
    -m 512M \
    -cpu max,+pku \
    -cdrom sexos-v1.0.0.iso \
    -serial stdio \
    -boot d
```

`-cpu max,+pku` — required for Intel PKU support. Without `+pku`, `is_pku_supported()` returns false and PKU is skipped, but `wrpkru` calls will fault.

### Display

Add `-display gtk` or `-display sdl` to see the framebuffer. The ISO boots to a black screen during Phase 21 because sexdisplay hasn't drawn anything yet.

### Debug Tips

| Symptom | Likely Cause |
|---------|-------------|
| Limine `Conflict detected for request ID` | Duplicate Limine request static, OR missing `RequestsStartMarker`/`RequestsEndMarker` |
| Limine `PHDRs with different permissions sharing same page` | Limine sections split across RO/RW PT_LOAD segments in linker script |
| Limine `Failed to open module with path` | Module path in `limine.cfg` doesn't exist in ISO |
| Silent halt after "Bootstrapping system Protection Domains..." | `wrpkru` without `CR4.PKE`, or `hal::init()` not called before `init::init()` |
| Silent halt after "Initializing GDT/IDT..." | `Star::write().unwrap()` panics — wrong RPL on kernel_ss selector |
| Black screen, serial stops at memory init | Heap init failed (OOM or wrong HHDM offset) |
| Triple fault (QEMU restarts) | Page fault with no IDT loaded |

---

## 15. Known Issues & Phase Status

### Phase 21 Status (as of this session)

| Component | Status |
|-----------|--------|
| Limine 7.x request table | ✅ Fixed — start/end markers added |
| `hal::init()` call ordering | ✅ Fixed — now called before memory/PD init |
| `memory::manager::init()` call ordering | ✅ Fixed — heap ready before `Box::new()` |
| `wrpkru` without `CR4.PKE` | ✅ Fixed — `hal::init()` enables PKU first |
| Duplicate `HhdmRequest` | ✅ Fixed — removed from `init.rs`, canonical in `lib.rs` |
| `pdx_spawn()` scaffold | ✅ Fixed — real ELF load + PD registration |
| `MODULE_PATH` in `limine.cfg` | ✅ Fixed — `servers/sexdisplay` added, `initrd.sex` removed |
| Ring-3 IRETQ handoff | ✅ Implemented |
| `Star::write().unwrap()` panic | ⚠️ **OPEN** — kernel_ss passed as `user_data_selector` (RPL=3); causes halt after "Initializing GDT/IDT..." |
| sexdisplay rendering | ⏳ Phase 22 — needs framebuffer handoff + compositor init |
| Scheduler + task switching | ⏳ Phase 22+ |
| SYSCALL-based IPC from ring-3 | ⏳ Phase 22+ — blocked on GDT restructure |

### GDT Restructure (Required for SYSCALL)

The GDT must be restructured to have a kernel data segment for `Star::write()` to succeed:

```
Required layout for SYSCALL/SYSRET:
  0x00: null
  0x08: kernel code       ← SYSCALL CS (STAR[47:32] = 0x08)
  0x10: kernel data       ← SYSCALL SS = CS+8 = 0x10
  0x18: [compat32 pad]    ← SYSRET base (STAR[63:48] = 0x18)
  0x20: user data         ← SYSRET SS = base+8 = 0x20
  0x28: user code 64-bit  ← SYSRET CS = base+16 = 0x28
  0x30: TSS (2 entries)
```

**Fix needed in `kernel/src/gdt.rs`:** Add `kernel_data_segment()`, add `kernel_data_selector` to `Selectors`, update `Star::write()` call in `interrupts.rs` to pass correct selectors.

---

## 16. Critical File Index

### Kernel Core

| File | Key Contents |
|------|-------------|
| `kernel/src/main.rs` | `_start()`, panic handler |
| `kernel/src/lib.rs` | `kernel_init()`, `ALLOCATOR`, `HEAP_START/SIZE`, all Limine requests, `strlen`/`memcpy`/`memset`/`memcmp` |
| `kernel/src/init.rs` | `init()`, `pdx_spawn()`, `jump_to_userland()`, `SEXDISPLAY_PD_ID`, `SEXINPUT_PD_ID` |
| `kernel/src/hal/x86_64.rs` | `X86Hal::init()` — PKU, GDT, IDT init sequence |
| `kernel/src/hal/mod.rs` | `hal::init()`, `HAL` static, `HardwareAbstractionLayer` trait |
| `kernel/src/gdt.rs` | `GDT` lazy_static, `TSS`, `Selectors`, `init()`, `get_selectors()` |
| `kernel/src/interrupts.rs` | `IDT`, `init_idt()`, `syscall_entry`, all interrupt handlers |
| `kernel/src/pku.rs` | `enable_pku()`, `is_pku_supported()`, `Pkru::read/write`, `init_pd_pkru()` |
| `kernel/src/elf.rs` | `load_elf_for_pd()`, `ElfHeader`, `ProgramHeader` |
| `kernel/linker.ld` | Higher-half sections, Limine request sections in `.data` |

### Memory

| File | Key Contents |
|------|-------------|
| `kernel/src/memory/manager.rs` | `init()`, `GLOBAL_VAS`, `GlobalVas`, `BitmapFrameAllocator`, `update_page_pkey()`, `init_sexting()` |
| `kernel/src/memory/allocator.rs` | `LockFreeBuddyAllocator`, `GLOBAL_ALLOCATOR`, `PageMetadata`, `CoreShardedLists` |
| `kernel/src/memory/pku.rs` | `wrpkru()`, `tag_virtual_address()`, `init_pku_isolation()` |

### IPC & Capabilities

| File | Key Contents |
|------|-------------|
| `kernel/src/capability.rs` | `ProtectionDomain`, `CapabilityTable`, `CapabilityData` enum, all cap types |
| `kernel/src/ipc.rs` | `DOMAIN_REGISTRY`, `DomainRegistry`, `pdx_call_with_mask()` |
| `kernel/src/ipc_ring.rs` | `RingBuffer<T, N>` |
| `kernel/src/ipc/messages.rs` | `MessageType` enum |
| `kernel/src/graphics/handoff.rs` | `ship_to_sexdisplay()` |

### Build & Config

| File | Key Contents |
|------|-------------|
| `build_payload.sh` | Full build automation (kernel + sexdisplay → iso_root/) |
| `Makefile` | `iso`, `run-sasos`, `clean` targets |
| `limine.cfg` | Boot entry, `MODULE_PATH=boot:///servers/sexdisplay` |
| `x86_64-sex.json` | Custom Rust target spec |
| `kernel/linker.ld` | Linker script with Limine sections |

### Display Server

| File | Key Contents |
|------|-------------|
| `servers/sexdisplay/src/main.rs` | `_start()`, main PDX event loop |
| `servers/sexdisplay/src/lib.rs` | `SexCompositor`, `Window`, `OutputState`, `WindowBuffer`, PDX syscall IDs |

---

*Manual generated from Phase 21 boot debugging session. Covers all subsystems explored during silent-halt diagnosis and userland handoff implementation.*
