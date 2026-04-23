# SexOS Black Screen Debug Handoff

## Status
Init sequence fixed. PDs spawn. Scheduler ticks. **Black screen because syscall 27 is not dispatched.**

## Root Cause (verified by reading all files)

### Bug 1: Syscall 27 (`pdx_call`) not in dispatch table
`kernel/src/syscalls/mod.rs` dispatch only handles:
- 69 → serial_print
- 24 → park
- 100 → yield (but `sys_yield` in sex-pdx uses syscall **32**, not 100 — another mismatch)

Everything else returns `u64::MAX`.

`pdx_call()` in `crates/sex-pdx/src/lib.rs:219` uses `syscall` with `rax=27`. Returns `u64::MAX`.

**Effect in linen (`apps/linen/src/main.rs:65`):**
```rust
let canvas_addr = pdx_call(COMPOSITOR_SLOT, OP_WINDOW_CREATE, 0, 0, 0);
// canvas_addr = u64::MAX (not 0!) → enters draw block
// writes to 0xFFFF_FFFF_FFFF_FFFF → PAGE FAULT
```

**Effect in silk-shell (`servers/silk-shell/src/main.rs:36`):**
```rust
let canvas_addr = pdx_call(COMPOSITOR_SLOT, OP_WINDOW_CREATE, 0, 0, 0);
// canvas_addr = u64::MAX (not 0!) → passes `if canvas_addr == 0` check inverted
// draws to u64::MAX → PAGE FAULT
```

### Bug 2: page_fault_handler doesn't switch after fault
`kernel/src/interrupts.rs:233`:
```rust
sched.tick();       // returns Some() but result IGNORED
unsafe { send_eoi(); }
// iretq back to faulting instruction → infinite #PF loop silently
```

Result: faulting task loops forever in #PF, timer still fires → TICK spam.

### Bug 3: sexdisplay never runs (queue starvation)
Spawn order in `kernel/src/init.rs`: sexdisplay first (index 0), silk-shell (index 1), linen (index 2). Bottom=3.

`WorkStealingQueue::pop()` is LIFO. First pop returns linen (index 2). After steady state, bottom oscillates 1↔2, cycling only linen↔silk-shell. **sexdisplay at index 0 is never popped.**

Fix: push sexdisplay LAST (change spawn order so sexdisplay is spawned after the others).

### Bug 4: Even if sexdisplay ran, it can't get framebuffer
`servers/sexdisplay/src/main.rs` calls `pdx_call(0, PDX_GET_DISPLAY_INFO, ...)` → syscall 27 → returns u64::MAX → `info.virt_addr = 0` → no RED beacon.

## The Fix (minimum viable)

**All display IPC must be handled directly in syscall 27 in the kernel.**
No routing through sexdisplay's message ring. Kernel IS the display layer for now.

### Fix 1: Add syscall 27 to `kernel/src/syscalls/mod.rs`

```rust
27 => { // pdx_call(slot, opcode, arg0, arg1, arg2)
    let slot   = regs.rdi as u32;
    let opcode = regs.rsi;
    let arg0   = regs.rdx;
    // arg1 = regs.r10, arg2 = regs.r8

    match (slot, opcode) {
        // PDX_GET_DISPLAY_INFO (slot=0, opcode=0x03)
        (0, 0x03) => {
            // arg0 = pointer to DisplayInfo struct in userland
            let fb_resp = crate::FB_REQUEST.response();
            if let Some(fb) = fb_resp {
                if let Some(fb0) = fb.framebuffers().next() {
                    let hhdm = crate::HHDM_REQUEST.response().map(|r| r.offset).unwrap_or(0);
                    let info_ptr = arg0 as *mut DisplayInfoKernel;
                    unsafe {
                        (*info_ptr).virt_addr = fb0.addr() + hhdm;
                        (*info_ptr).width     = fb0.width() as u32;
                        (*info_ptr).height    = fb0.height() as u32;
                        (*info_ptr).pitch     = fb0.pitch() as u32;
                    }
                    return 0;
                }
            }
            u64::MAX
        },
        // OP_SET_BG (slot=5, opcode=0x100) — fill FB with color in arg0
        (5, 0x100) => {
            do_fill_fb(arg0 as u32);
            0
        },
        // OP_WINDOW_CREATE (slot=5, opcode=0xDE) — return shared canvas addr
        (5, 0xDE) => {
            ensure_shared_canvas_mapped();
            0x4000_0000u64
        },
        // OP_WINDOW_COMMIT_FRAME (slot=5, opcode=0xDD) — blit canvas to FB
        (5, 0xDD) => {
            do_canvas_blit();
            0
        },
        // OP_WINDOW_PAINT (slot=5, opcode=0xDF) — full canvas blit
        (5, 0xDF) => {
            do_canvas_blit();
            0
        },
        _ => 0,
    }
},
32 => 0, // sys_yield stub (sex-pdx uses syscall 32)
28 => 0, // pdx_listen stub — return empty PdxEvent
29 => 0, // pdx_reply stub
```

Helper functions to add (or inline in the match):
- `do_fill_fb(color: u32)`: get FB from `FB_REQUEST`, get HHDM offset, fill all pixels
- `ensure_shared_canvas_mapped()`: map 0x4000_0000, 1280×768×4 bytes, PKEY 15 (shared), once via `lazy_static` or atomic flag
- `do_canvas_blit()`: copy 1280×32 pixels from 0x4000_0000 to FB addr

**SHARED_CANVAS = 0x4000_0000 must be mapped WRITABLE for all user PKEYs.** Use PKEY 15 (already open in every PD's base PKRU mask via the `!= !(0b11 << 30)` bit clear in `capability.rs:156`).

### Fix 2: Spawn order in `kernel/src/init.rs`
Collect all modules, spawn non-sexdisplay first, sexdisplay last. This ensures sexdisplay is at the highest index in WorkStealingQueue → popped first.

```rust
// Two-pass: non-display first, sexdisplay last
for module in modules.modules() {
    let path = module.path();
    if !path.contains("sexdisplay") && (path.contains("silk-shell") || path.contains("linen")) {
        // spawn it
    }
}
for module in modules.modules() {
    let path = module.path();
    if path.contains("sexdisplay") {
        // spawn it last
    }
}
```

### Fix 3: page_fault_handler must call switch_to
`kernel/src/interrupts.rs:226`:

```rust
// After forwarding the fault, call switch_to if tick() returns Some
if let Some((old_ctx, next_ctx)) = sched.tick() {
    unsafe {
        send_eoi();
        crate::scheduler::Scheduler::switch_to(old_ctx, next_ctx);
    }
}
unsafe { send_eoi(); }
```

### Fix 4: linen — check canvas_addr correctly
`apps/linen/src/main.rs`: pdx_call now returns 0x4000_0000 (not 0, not u64::MAX). The `if canvas_addr != 0` check is correct. No change needed once syscall 27 is fixed.

`servers/silk-shell/src/main.rs:38`: `if canvas_addr == 0` is INVERTED — should be `if canvas_addr == 0 { /* error */ } else { /* draw */ }`. Current code has the draw block inside the 0-check. **Must invert this condition.**

## Key File Map (don't re-read, already verified)

| File | Key facts |
|------|-----------|
| `kernel/src/syscalls/mod.rs` | Dispatch: only 69/24/100. Add 27/28/29/32. |
| `kernel/src/init.rs` | Spawn order: sexdisplay first → starvation. Fix: spawn last. |
| `kernel/src/interrupts.rs:172` | timer_interrupt_handler — `switch_to` called correctly on tick. |
| `kernel/src/interrupts.rs:226` | page_fault_handler — `sched.tick()` result IGNORED, no switch_to. |
| `kernel/src/scheduler.rs:144` | `tick()` — LIFO pop, returns Option<(old_ctx, next_ctx)>. |
| `kernel/src/scheduler.rs:84` | `push()` — LIFO push at bottom. |
| `crates/sex-pdx/src/lib.rs:219` | `pdx_call` uses syscall 27. `pdx_listen` uses 28. `pdx_reply` uses 29. `sys_yield` uses 32. |
| `crates/sex-pdx/src/lib.rs:50` | `SLOT_DISPLAY=5`, `OP_WINDOW_CREATE=0xDE`, `OP_WINDOW_COMMIT_FRAME` (not in lib — defined inline in apps as 0xDD), `OP_SET_BG=0x100`, `PDX_GET_DISPLAY_INFO=0x03`. |
| `kernel/src/lib.rs:77` | `MODULE_REQUEST` — already in .limine_requests section. ✓ |
| `kernel/src/memory/manager.rs:70` | `init_heap()` already called. ✓ |
| `apps/linen/src/main.rs` | 2M spin, pdx_call slot 5, draw WHITE if canvas != 0, commit. |
| `servers/silk-shell/src/main.rs` | 1M spin, OP_SET_BG, OP_WINDOW_CREATE, **WRONG: `if canvas_addr == 0` wraps draw block — must invert**. |
| `servers/sexdisplay/src/main.rs` | Calls pdx_call(0, 0x03) for FB info, paints RED then DARK GREY, then pdx_listen loop. |
| `kernel/src/capability.rs:150` | `ProtectionDomain::new()` clears PKRU bits for own PKEY AND PKEY 15 (shared). |
| `kernel/src/gdt.rs:22` | TSS.RSP0 = static KERNEL_STACK (5 pages). Used for Ring3→Ring0 interrupt stack. |
| `kernel/src/core_local.rs:51` | Both GsBase AND KernelGsBase set to CoreLocal ptr (intentional — both hold kernel ptr). |

## ipc.rs safe_pdx_call behavior
- `CapabilityData::Domain(id)` → enqueues async message, returns `Ok(0)` — NOT a synchronous call
- `CapabilityData::IPC(data)` → PKRU flip + direct function call — synchronous
- Slot 5 in cap_table is `Domain(sexdisp_id)` — so currently returns 0, BUT syscall 27 returns u64::MAX before even reaching safe_pdx_call

## TaskContext layout (C-ABI offsets)
```
+0x00 r15, +0x08 r14, +0x10 r13, +0x18 r12
+0x20 rbx, +0x28 rbp
+0x30 pkru (u32), +0x34 pd_id (u32)
+0x38 rip, +0x40 cs, +0x48 rflags, +0x50 rsp, +0x58 ss
+0x60 pd_ptr
```
switch_to naked fn uses these offsets directly. Correct.

## ELF load addresses
- sexdisplay: load_base=0x20200000, entry=0x20201340
- silk-shell: load_base=0x20400000, entry=0x20401280
- linen: load_base=0x20600000, entry=0x20601280

## Expected boot sequence after fix
1. sexdisplay runs first → syscall 27 + PDX_GET_DISPLAY_INFO → gets FB addr → paints RED then DARK GREY
2. sexdisplay enters pdx_listen loop (syscall 28 → returns empty event → sys_yield → back to scheduler)
3. silk-shell runs → syscall 27 + OP_SET_BG → kernel fills FB purple
4. silk-shell → OP_WINDOW_CREATE → kernel returns 0x4000_0000
5. silk-shell writes HOT PINK strip to 0x4000_0000 canvas
6. silk-shell → OP_WINDOW_COMMIT_FRAME → kernel blits canvas to FB → HOT PINK visible
7. linen runs → OP_WINDOW_CREATE → 0x4000_0000 → writes WHITE strip → commit → WHITE visible

## Build/run commands
```
cargo build --package sex-kernel   # verify compile
./build_payload.sh && make iso && make run-sasos
```
QEMU: 512MB RAM, GTK display, serial to stdio. Kernel pkg name = `sex-kernel`.
