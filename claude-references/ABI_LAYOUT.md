# ABI & Memory Layout Reference

> Referenced from CLAUDE.md (offloaded reference).
> Do not contradict invariants in CREW.md or CLAUDE.md.

---

## GDT & TSS (The 16-Byte Rule)

The `x86_64` crate has a strict 8-slot GDT limit. A Task State Segment (TSS) in long mode
is a "System Segment" requiring **two contiguous 8-byte slots** (16 bytes).

**MANDATORY GDT ORDER:**
| Slot | Content       |
|------|---------------|
| 0    | Null          |
| 1    | Kernel Code   |
| 2    | Kernel Data   |
| 3 & 4| TSS (MUST be here, before User segments, to prevent array overflow) |
| 5    | User Data (SS)|
| 6    | User Code (CS)|

### SYSRET Mathematical Offsets

`x86_64::registers::model_specific::Star::write` will throw a `SysretOffset` panic if
indices violate:
- Kernel SS Index MUST be `Kernel CS + 1` (Index 2 = 1 + 1).
- User CS Index MUST be `User SS + 1` (Index 5 = 4 + 1 — hardware confirmed).
- *Never* pass `user_data_selector` as the Kernel SS parameter.

### Ring-3 Context Switch (IRETQ)

- **The RPL Drop:** User selectors MUST explicitly be bitwise-OR'd with RPL3 (`| 3`).
  - `User CS` must evaluate to `0x2B` (GDT index 5 | RPL3). **NOT 0x33** — index 6 is TSS → `#GP(0x30)`.
  - `User SS` must evaluate to `0x23` (GDT index 4 | RPL3).
- **Actual GDT user segment layout (hardware-confirmed):**
  - Index 4: User Data (SS) → selector `0x20`, with RPL3 = `0x23`
  - Index 5: User Code (CS) → selector `0x28`, with RPL3 = `0x2B`
  - Index 6-7: TSS (system segment, 2 slots)
  - SYSRET math: User CS Index (5) = User SS Index (4) + 1 ✓
- **The Stack Bomb:** If using a custom stub before `iretq`, `Task::new()` must push
  exactly 15 dummy zeros on top of the hardware frame. Otherwise the stub's `pop r15 ... pop rdi`
  sequence eats the `iretq` frame.

---

## Memory Layout (SASOS Map)

All components share one PML4 (GLOBAL_VAS). Authority model: ARCHITECTURE.md §0.

| Component          | Virtual Address          | Notes |
|--------------------|--------------------------|-------|
| Kernel binary      | Higher-half (linker.ld)  | `no_std` core + Limine requests |
| Userland stubs     | `0x4000_0000`            | Mock entry point for `sex-ld` library mapping |
| Translated native  | `0x4000_1000`            | Target entry for translated ELFs via `sexnode` |
| System heap        | `0x4444_4444_0000`       | 128 MiB (HEAP_SIZE in lib.rs), mapped at boot |
| sexdisplay FB      | Dynamic (passed via IPC as OP_PRIMARY_FB) | Framebuffer pages tagged PKEY 1 |

- HHDM offset: `0xffff800000000000`
- All userland segments mapped via `GlobalVas::map_pku_range` which applies PD key to Level-1 page table entries
- Page tables: `OffsetPageTable` via x86_64 crate
- PD structs live on system heap at `0x4444_4444_0000`
- PD load bases: `0x4000_0000 + ((domain_id - 1) * 0x0100_0000)`
  - domain 1 (sexdisplay): 0x40000000
  - domain 2 (sexdrive):   0x41000000
  - domain 3 (silk-shell): 0x42000000
  - domain 4 (sexinput):   0x43000000
  - domain 5 (silkbar):    0x44000000
- User stacks: `0x7000_0000_0000 + (pku_key * 0x100_0000)`, 64KB each

---

## Protection Keys (PKU/MPK)

PKU is enabled in CR4. Every PDX domain has an assigned PKEY.

**Known PKEY assignments (from init.rs):**
| PDX          | PKEY | PKRU value (computed)      |
|--------------|------|----------------------------|
| sexdisplay   | 1    | `0x3FFF_FFF0` (allows 0,1,15) |
| silk-shell   | 2    | `0x3FFF_FFCC` (allows 0,2,15) |
| linen        | 3    | `0x3FFF_FF0F` (allows 0,3,15) |

**PKRU formula (from ProtectionDomain::new):**
```rust
let mut pkru_mask: u32 = 0xFFFF_FFFF;
pkru_mask &= !(0b11 << (pku_key * 2));  // allow own key
pkru_mask &= !0b11;                      // allow PKEY 0 (kernel/default)
pkru_mask &= !(0b11 << 30);             // allow PKEY 15 (shared IPC)
```

- Kernel heap (PKEY 0) is accessible from ALL PKRUs.
- Kernel entry (syscall/interrupt): `xor eax,eax; xor edx,edx; xor ecx,ecx; wrpkru` (opens ALL keys).
- **Never use** `core::arch::x86_64::_wrpkru` directly — use `crate::pku::wrpkru`.
- `kernel/src/memory/pku.rs` was deleted. The only PKU file is `kernel/src/pku.rs`.

---

## Syscall ABI

- Entry via `SYSCALL`/`SYSRET` (LSTAR/STAR configured in GDT init)
- SFMask clears IF on SYSCALL → interrupts disabled throughout syscall handler
- Syscall number in rax; arguments: rdi=slot, rsi=opcode, rdx=arg0, r10=arg1, r8=arg2
- **CRITICAL: `syscall_entry` `pop rax` restores original rax (syscall number), NOT the handler return value.** Kernel modifies `regs.rax` via the regs pointer if it wants to return a value to userland.
- **Never reference `opcode` in syscalls/mod.rs** — it does not exist. Use `num` (bound from rsi at dispatch entry via `let rsi = regs.rsi`).
- Arguments in dispatch: `let rdi=regs.rdi; let rsi=regs.rsi; ...`

**SyscallRegs layout (kernel/src/interrupts.rs):**
```
Push order: r11,rcx (1st save), r9,r8,r10,rdx,rsi,rdi,rax (SyscallRegs)
Memory:     [rsp+0]=rax, [+8]=rdi, [+16]=rsi, [+24]=rdx, [+32]=r10,
            [+40]=r8, [+48]=r9, [+56]=rcx, [+64]=r11
Then:       r15..rbp (callee-saved), then rax=pkru
sysretq:    rcx→RIP, r11→RFLAGS (restored from 2nd pop of first-save copies)
```

---

## PDX (Protection Domain eXtension) ABI

### Calling Convention (5-argument arity)

```rust
pdx_call(slot: u32, syscall: u64, arg0: u64, arg1: u64, arg2: u64) -> u64
```

**Register mapping:**
| Argument | Register | Notes |
|----------|----------|-------|
| slot     | rdi      | Capability slot index |
| syscall  | rsi      | Opcode (bound as rsi in dispatch) |
| arg0     | rdx      | |
| arg1     | r10      | |
| arg2     | r8       | |

- **Never use 4-argument arity** — causes stack misalignment on `sysretq`/`iretq`.
- `serial_println!` in PDX context: uses direct `syscall` with rax=69. **NOT** a null deref. Kernel handles rax=69 at top-level dispatch AND at slot=0/num=69.

### IPC Slot Convention

| Slot | Service |
|------|---------|
| 0    | Kernel direct (handled inline in dispatch, no safe_pdx_call) |
| 1    | sexfiles VFS |
| 2    | sext (demand pager) |
| 3    | sexinput (HID input ring) |
| 4    | Audio server |
| **5** | **sexdisplay (compositor)** — also silkbar's SLOT_DISPLAY target |
| 6    | silk-shell orchestration |

### Capability Table Structure

Each `ProtectionDomain` struct contains a `CapabilityTable` with `CapabilityData` entries.
Cap table is accessed via raw pointer in `init.rs` — `unsafe` is intentional.
`init.rs` inserts `CapabilityData::Domain(sexdisp_id)` at slot 5 for PDs 1..=4.

### Pixel Format

`0x00RRGGBB` (32-bit RGB, alpha ignored).

---

## Critical ABI Facts (Discovered in Session)

1. **No GOT relocation in ELF loader.** `kernel/src/elf.rs` copies segments but does NOT apply `.rela.dyn` relocations. Cross-crate `pub static` references produce stale GOT entries.
2. **Fix: use `const` not `static`** for shared data across PDX crates. Const values are inlined, no GOT involved.
3. **OP_PRIMARY_FB (0x11) message format:** arg0=fb_addr, arg1=(width | height<<32), arg2=pitch (pixels/row). Sent by kernel to sexdisplay's message ring before scheduler runs sexdisplay.
4. **OP_SILKBAR_UPDATE (0xF2) message format:** arg0=kind(4=SetClock), arg1=(index<<32 | a), arg2=b.
5. **silkbar PDX call:** `pdx_call(SLOT_DISPLAY, OP_SILKBAR_UPDATE, 4, (0<<32)|10, 44)` sends SetClock(10:44) to sexdisplay.

---

## Surface Opcodes (shell → sexdisplay)

| Opcode | Name             | Arguments |
|--------|------------------|-----------|
| `0xEC` | create surface   | arg0=id, arg1=(y<<32)|x, arg2=(h<<32)|w |
| `0xEB` | move surface     | arg0=id, arg1=x, arg2=y |
| `0xED` | focus surface    | arg0=id |
| `0xEE` | destroy surface  | arg0=id |
| `0xEF` | fill rect        | arg0=id, arg1=(sy<<32)|sx, arg2=(color<<32)|(sh<<16)|sw |

**Cursor surface:** `SURFACE_ID_CURSOR = 0x90` (144). Created first at boot (slot 0 in SURFACES array). `draw_cursor_z_top()` renders arrow bitmap unconditionally after all other passes.
