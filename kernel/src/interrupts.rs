use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::serial_println;
use crate::gdt;
use crate::ipc_ring::RingBuffer;
use lazy_static::lazy_static;

use core::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use crate::ipc::messages::MessageType;
static TIMER_TICK_LOG_BUDGET: AtomicU64 = AtomicU64::new(64);

/// The event structure for a page fault.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PageFaultEvent {
    pub addr: u64,
    pub error_code: u64,
    pub task_id: u32,
}

lazy_static! {
    /// Global Uptime Ticks (incremented by LAPIC timer).
    pub static ref TICKS: AtomicU64 = AtomicU64::new(0);

    /// The global asynchronous queue for Page Faults (sext pager).
    pub static ref SEXT_QUEUE: RingBuffer<PageFaultEvent, 256> = RingBuffer::new();

    /// Global lock-free input ring buffer for Ring-3 consumption
    pub static ref INPUT_RING: RingBuffer<u8, 256> = RingBuffer::new();
}

static VECTOR_OWNERS: [AtomicU32; 256] = [const { AtomicU32::new(0) }; 256];

pub fn register_irq_route(vector: u8, pd_id: u32) {
    VECTOR_OWNERS[vector as usize].store(pd_id, Ordering::Release);
    serial_println!("IRQ: Registered Vector {:#x} to PD {}", vector, pd_id);
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX as u16 + 1);
            
            // critical stubs (naked)
            idt.page_fault.set_handler_addr(x86_64::VirtAddr::new(page_fault_stub as *const () as u64));
            idt.general_protection_fault.set_handler_addr(x86_64::VirtAddr::new(general_protection_fault_stub as *const () as u64));
            idt[0x20].set_handler_addr(x86_64::VirtAddr::new(timer_interrupt_stub as *const () as u64));
        }

        idt[0x40].set_handler_fn(revoke_key_handler);
        idt[0x21].set_handler_fn(keyboard_interrupt_handler);
        idt[0x22].set_handler_fn(generic_irq_handler); 
        idt
    };
}

use x86_64::registers::model_specific::{LStar, SFMask, Efer, EferFlags};
use x86_64::registers::control::{Cr0, Cr3, Cr4};
use x86_64::VirtAddr;

pub fn init_idt() {
    serial_println!("   → Loading IDTR...");
    IDT.load();
    serial_println!("   → IDTR loaded successfully");

    unsafe {
        serial_println!("   → Setting up LSTAR (Syscall Entry)...");
        LStar::write(VirtAddr::new(syscall_entry as *const () as u64));
        serial_println!("   → Setting up STAR...");
        let selectors = crate::gdt::get_selectors();

        use x86_64::registers::model_specific::Star;

        // Intel/AMD Contract: STAR[47:32] = SYSCALL CS/SS, STAR[63:48] = SYSRET CS/SS
        Star::write(
            selectors.user_cs,
            selectors.user_ss,
            selectors.kernel_cs,
            selectors.kernel_ss,
        ).expect("Failed to write STAR MSR");

        serial_println!("→ STAR MSR locked");
        SFMask::write(x86_64::registers::rflags::RFlags::INTERRUPT_FLAG);
        Efer::write(Efer::read() | EferFlags::SYSTEM_CALL_EXTENSIONS);
        serial_println!("   → Syscall setup COMPLETE");    }
}

fn log_exec_control_state(tag: &str) {
    let cr0 = Cr0::read().bits();
    let (cr3, _) = Cr3::read();
    let cr3 = cr3.start_address().as_u64();
    let cr4 = Cr4::read().bits();
    let efer = Efer::read().bits();
    let nxe = (efer & (1 << 11)) != 0;
    let smep = (cr4 & (1 << 20)) != 0;
    let smap = (cr4 & (1 << 21)) != 0;
    let pke = (cr4 & (1 << 22)) != 0;
    let pae = (cr4 & (1 << 5)) != 0;
    serial_println!(
        "cpu.exec {} cr0={:#x} cr3={:#x} cr4={:#x} efer={:#x} nxe={} smep={} smap={} pke={} pae={}",
        tag, cr0, cr3, cr4, efer, nxe, smep, smap, pke, pae
    );
}

#[repr(C)]
pub struct SyscallRegs {
    pub rax: u64, pub rdi: u64, pub rsi: u64, pub rdx: u64,
    pub r10: u64, pub r8:  u64, pub r9:  u64, pub rcx: u64, pub r11: u64,
}

static SYSCALL_STUB_ENTER_RAW: [u8; 23] = *b"syscall.stub.enter.raw\n";

#[no_mangle]
pub extern "C" fn syscall_stub_after_swapgs() {
    serial_println!("syscall.stub.enter.raw");
}

#[no_mangle]
pub extern "C" fn syscall_stub_after_kstack_switch() {
    serial_println!("syscall.stub.after.kstack.switch");
}

#[no_mangle]
pub extern "C" fn syscall_stub_before_dispatch() {
    serial_println!("syscall.stub.before.dispatch");
}

#[unsafe(naked)]
pub unsafe extern "C" fn syscall_entry() {
    core::arch::naked_asm!(
        "swapgs",
        "mov gs:[24], rsp", // Save User RSP
        "mov rsp, gs:[16]",  // Load Kernel RSP

        // 1. SAVE ALL STATE BEFORE CLOBBERING
        "push r11",         // Save User RFLAGS
        "push rcx",         // Save User RIP
        "push rax",         // Save Syscall Number (rax)
        "push rdi",         // Save Arg 0 (rdi)
        "push rsi",         // Save Arg 1 (rsi)
        "push rdx",         // Save Arg 2 (rdx)
        "push r10",         // Save Arg 3 (r10)
        "push r8",          // Save Arg 4 (r8)
        "push r9",          // Save Arg 5 (r9)

        // 2. NOW LOG (Manual loop clobbers rax, rsi, rcx, rdx - but we saved them)
        "mov dx, 0x3f8",
        "lea rsi, [rip + {enter_msg}]",
        "mov ecx, {enter_len}",
        "cld",
        "1:",
        "lodsb",
        "out dx, al",
        "loop 1b",

        "call syscall_stub_after_swapgs",
        "call syscall_stub_after_kstack_switch",

        // 3. Read and Save User PKRU (or synthetic 0 when PKU unsupported)
        "cmp byte ptr [rip + {pku_enabled}], 0",
        "je 90f",
        "xor ecx, ecx",
        "rdpkru",
        "jmp 91f",
        "90:",
        "xor eax, eax",
        "91:",
        "push rax",         // Save PKRU mask to stack

        // 4. Enter God Mode
        "cmp byte ptr [rip + {pku_enabled}], 0",
        "je 92f",
        "xor eax, eax",
        "xor ecx, ecx",
        "xor edx, edx",
        "wrpkru",
        "92:",

        // 5. Save Callee-Saved Registers
        "push rbp", "push rbx", "push r12", "push r13", "push r14", "push r15",
        "mov rbp, rsp",

        // 6. Build SyscallRegs (Volatiles + Args)
        // Stack at this point (top down):
        // [rsp+0..47]: r15-rbp (48)
        // [rsp+48]: PKRU mask (8)
        // [rsp+56]: r9 (8)
        // [rsp+64]: r8 (8)
        // [rsp+72]: r10 (8)
        // [rsp+80]: rdx (8)
        // [rsp+88]: rsi (8)
        // [rsp+96]: rdi (8)
        // [rsp+104]: rax (8)
        // [rsp+112]: rcx (8) - RIP
        // [rsp+120]: r11 (8) - RFLAGS

        "push [rsp + 120]", // r11
        "push [rsp + 120]", // rcx (offset 112 + 8)
        "push [rsp + 72]",  // r9 (offset 56 + 16)
        "push [rsp + 80]",  // r8 (offset 64 + 16)
        "push [rsp + 88]",  // r10 (offset 72 + 16)
        "push [rsp + 96]",  // rdx (offset 80 + 16)
        "push [rsp + 104]", // rsi (offset 88 + 16)
        "push [rsp + 112]", // rdi (offset 96 + 16)
        "push [rsp + 120]", // rax (offset 104 + 16)

        // Call Handler
        "call syscall_stub_before_dispatch",
        "mov rdi, rsp",     // Pointer to SyscallRegs
        "call syscall_handler",

        // 7. RESTORE C-ABI VOLATILES
        "add rsp, 72",      // Discard SyscallRegs

        // 8. Restore Callee-Saved Registers
        "pop r15", "pop r14", "pop r13", "pop r12", "pop rbx", "pop rbp",

        // 9. Restore PKRU Mask
        "pop rdi",          // Pop original PKRU mask into rdi
        "push rax",         // Stash return value
        "cmp byte ptr [rip + {pku_enabled}], 0",
        "je 93f",
        "mov rax, rdi",
        "xor ecx, ecx",
        "xor edx, edx",
        "wrpkru",
        "93:",
        "pop rax",

        // 10. Restore Saved Volatiles
        "pop r9", "pop r8", "pop r10", "pop rdx", "pop rsi", "pop rdi",
        "add rsp, 8",       // Discard rax (return value already in rax)
        "pop rcx",          // RIP
        "pop r11",          // RFLAGS

        "mov rsp, gs:[24]", // Restore User RSP
        "swapgs",
        "sysretq",
        enter_msg = sym SYSCALL_STUB_ENTER_RAW,
        enter_len = const SYSCALL_STUB_ENTER_RAW.len(),
        pku_enabled = sym crate::pku::PKU_ENABLED,
    );
}

#[no_mangle]
pub extern "C" fn syscall_handler(regs: &mut SyscallRegs) -> u64 {
    unsafe { crate::pku::wrpkru(0x00000000); }
    crate::syscalls::dispatch(regs)
}

// ── Assembly Stubs ──────────────────────────────────────────────────────────

#[unsafe(naked)]
pub unsafe extern "C" fn timer_interrupt_stub() {
    core::arch::naked_asm!(
        "push 0", // DUMMY ERROR CODE
        "push r15", "push r14", "push r13", "push r12", "push r11", "push r10", "push r9", "push r8",
        "push rdi", "push rsi", "push rbp", "push rdx", "push rcx", "push rbx", "push rax",
        
        // CPU Frame = 40 bytes. GPRs = 120 bytes. Error Code = 8 bytes.
        // RIP is at 128. CS is at 136.
        "mov rax, [rsp + 136]", 
        "test al, 3", 
        "jz 1f", 
        "swapgs", 
        "1:",

        "cmp byte ptr [rip + {pku_enabled}], 0",
        "je 3f",
        "xor eax, eax",
        "xor ecx, ecx",
        "xor edx, edx",
        "wrpkru",
        "3:",

        "sub rsp, 8", // ALIGN STACK TO 16 BYTES
        "lea rdi, [rsp + 136]", // rdi points to InterruptStackFrame (RIP)
        "call timer_interrupt_handler",
        "add rsp, 8", // UNDO ALIGNMENT

        "mov rax, [rsp + 136]", 
        "test al, 3", 
        "jz 2f", 
        "swapgs", 
        "2:",

        "pop rax", "pop rbx", "pop rcx", "pop rdx", "pop rbp", "pop rsi", "pop rdi",
        "pop r8", "pop r9", "pop r10", "pop r11", "pop r12", "pop r13", "pop r14", "pop r15",
        "add rsp, 8", // DISCARD DUMMY ERROR CODE
        "iretq",
        pku_enabled = sym crate::pku::PKU_ENABLED,
    );
}

#[unsafe(naked)]
pub unsafe extern "C" fn page_fault_stub() {
    core::arch::naked_asm!(
        // CPU already pushed Error Code (8 bytes). CPU frame = 48 bytes.
        "push r15", "push r14", "push r13", "push r12", "push r11", "push r10", "push r9", "push r8",
        "push rdi", "push rsi", "push rbp", "push rdx", "push rcx", "push rbx", "push rax",

        // GPRs = 120 bytes. Error Code is at 120. RIP is at 128. CS is at 136.
        "mov rax, [rsp + 136]", 
        "test al, 3", 
        "jz 1f", 
        "swapgs", 
        "1:",

        "cmp byte ptr [rip + {pku_enabled}], 0",
        "je 3f",
        "xor eax, eax",
        "xor ecx, ecx",
        "xor edx, edx",
        "wrpkru",
        "3:",

        "sub rsp, 8", // CRITICAL: Re-align stack to 16 bytes before C call!
        "lea rdi, [rsp + 136]", // rdi points to InterruptStackFrame (RIP)
        "mov rsi, [rsp + 128]", // rsi contains the Error Code
        "call page_fault_handler",
        "add rsp, 8", // UNDO ALIGNMENT

        "mov rax, [rsp + 136]", 
        "test al, 3", 
        "jz 2f", 
        "swapgs", 
        "2:",

        "pop rax", "pop rbx", "pop rcx", "pop rdx", "pop rbp", "pop rsi", "pop rdi",
        "pop r8", "pop r9", "pop r10", "pop r11", "pop r12", "pop r13", "pop r14", "pop r15",
        "add rsp, 8", // DISCARD ERROR CODE
        "iretq",
        pku_enabled = sym crate::pku::PKU_ENABLED,
    );
}

#[unsafe(naked)]
pub unsafe extern "C" fn general_protection_fault_stub() {
    core::arch::naked_asm!(
        // CPU already pushed Error Code (8 bytes). CPU frame = 48 bytes.
        "push r15", "push r14", "push r13", "push r12", "push r11", "push r10", "push r9", "push r8",
        "push rdi", "push rsi", "push rbp", "push rdx", "push rcx", "push rbx", "push rax",

        // GPRs = 120 bytes. Error Code is at 120. RIP is at 128. CS is at 136.
        "mov rax, [rsp + 136]", 
        "test al, 3", 
        "jz 1f", 
        "swapgs", 
        "1:",

        "cmp byte ptr [rip + {pku_enabled}], 0",
        "je 3f",
        "xor eax, eax",
        "xor ecx, ecx",
        "xor edx, edx",
        "wrpkru",
        "3:",

        "sub rsp, 8", // CRITICAL: Re-align stack to 16 bytes before C call!
        "lea rdi, [rsp + 136]", // rdi points to InterruptStackFrame (RIP)
        "mov rsi, [rsp + 128]", // rsi contains the Error Code
        "call general_protection_fault_handler",
        "add rsp, 8", // UNDO ALIGNMENT

        "mov rax, [rsp + 136]", 
        "test al, 3", 
        "jz 2f", 
        "swapgs", 
        "2:",

        "pop rax", "pop rbx", "pop rcx", "pop rdx", "pop rbp", "pop rsi", "pop rdi",
        "pop r8", "pop r9", "pop r10", "pop r11", "pop r12", "pop r13", "pop r14", "pop r15",
        "add rsp, 8", // DISCARD ERROR CODE
        "iretq",
        pku_enabled = sym crate::pku::PKU_ENABLED,
    );
}

// ── Rust Handlers ───────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn timer_interrupt_handler(stack_frame: &mut InterruptStackFrame) {
    if TIMER_TICK_LOG_BUDGET
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_sub(1))
        .is_ok()
    {
        serial_println!("timer.tick.enter");
    }
    TICKS.fetch_add(1, Ordering::Relaxed);
    let core_id = crate::core_local::CoreLocal::get().core_id;
    let sched = &crate::scheduler::SCHEDULERS[core_id as usize];
    
    if TICKS.load(Ordering::Relaxed) % 100 == 0 {
        serial_println!("[DEBUG] timer_tick: core {} using runqueue at {:p}", core_id, &sched.runqueue);
    }

    let result = sched.tick();
    if result.is_none() {
        unsafe { send_eoi(); }
        return;
    }

    let (old_ctx_ptr, next_ctx_ptr) = result.unwrap();

    // Sanity check: Ensure CoreLocal pointer was set by tick()
    assert!(!crate::core_local::CoreLocal::get().current_pd_ptr.load(core::sync::atomic::Ordering::Acquire).is_null());

    unsafe {
        if !old_ctx_ptr.is_null() {
            let old_ctx = &mut *old_ctx_ptr;
            old_ctx.rip = stack_frame.instruction_pointer.as_u64();
            old_ctx.cs = stack_frame.code_segment.0 as u64;
            old_ctx.rflags = stack_frame.cpu_flags.bits();
            old_ctx.ss = stack_frame.stack_segment.0 as u64;
            
            let base = stack_frame as *const _ as *const u64;
            old_ctx.r15 = *base.offset(-2);
            old_ctx.r14 = *base.offset(-3);
            old_ctx.r13 = *base.offset(-4);
            old_ctx.r12 = *base.offset(-5);
            old_ctx.r11 = *base.offset(-6);
            old_ctx.r10 = *base.offset(-7);
            old_ctx.r9  = *base.offset(-8);
            old_ctx.r8  = *base.offset(-9);
            old_ctx.rdi = *base.offset(-10);
            old_ctx.rsi = *base.offset(-11);
            old_ctx.rbp = *base.offset(-12);
            old_ctx.rdx = *base.offset(-13);
            old_ctx.rcx = *base.offset(-14);
            old_ctx.rbx = *base.offset(-15);
            old_ctx.rax = *base.offset(-16);
            old_ctx.rsp = stack_frame.stack_pointer.as_u64();

            if TIMER_TICK_LOG_BUDGET.load(Ordering::Acquire) > 0 {
                serial_println!("timer.save_context pd_id={} old_rip={:#x} old_rsp={:#x} kstack_top={:#x}",
                    (*old_ctx.pd_ptr).id, old_ctx.rip, old_ctx.rsp, old_ctx.kstack_top);
            }

            use core::sync::atomic::Ordering;
            old_ctx.pkru = (*old_ctx.pd_ptr).current_pkru_mask.load(Ordering::Relaxed) as u64;
        }
        
        let next_rip = (*next_ctx_ptr).rip;
        if next_rip == 0 {
            let next_pd_ptr = crate::core_local::CoreLocal::get().current_pd_ptr.load(core::sync::atomic::Ordering::Acquire);
            panic!("SCHED: null RIP for PD {}", (*next_pd_ptr).id);
        }

        let kstack_top = (*next_ctx_ptr).kstack_top;
        if TIMER_TICK_LOG_BUDGET.load(Ordering::Acquire) > 0 {
            serial_println!("scheduler.restore_context pd_id={} kstack_top={:#x} rip={:#x}",
                (*next_ctx_ptr).pd_id, kstack_top, (*next_ctx_ptr).rip);
        }
        // RSP0 must be empty kernel-stack top, not saved-context base.
        // CPU pushes DOWNWARD from RSP0 on user→kernel interrupt.
        // kstack_top = saved frame base. kstack_top + 160 = kstack_alloc_top.
        crate::gdt::update_tss_rsp0(x86_64::VirtAddr::new(kstack_top + 160));
        serial_println!("rsp0.programmed pd_id={} saved_kstack_top={:#x} rsp0={:#x}",
            (*next_ctx_ptr).pd_id, kstack_top, kstack_top + 160);

        let next_pd_ptr = crate::core_local::CoreLocal::get().current_pd_ptr.load(core::sync::atomic::Ordering::Acquire);
        crate::scheduler::log_first_scheduled_pd((*next_ctx_ptr).pd_id);
        serial_println!("SCHED: Switching to PD {} (RIP={:#x}, RSP={:#x}, KSTACK={:#x})", 
                        (*next_pd_ptr).id, (*next_ctx_ptr).rip, (*next_ctx_ptr).rsp, kstack_top);
        serial_println!(
            "context_switch.before_switch_to rip={:#x} rsp={:#x} rflags={:#x} cs={:#x} ss={:#x} pd_id={}",
            (*next_ctx_ptr).rip,
            (*next_ctx_ptr).rsp,
            (*next_ctx_ptr).rflags,
            (*next_ctx_ptr).cs,
            (*next_ctx_ptr).ss,
            (*next_ctx_ptr).pd_id
        );
        serial_println!(
            "switch.frame rip={:#x} cs={:#x} ss={:#x} rsp={:#x} rflags={:#x}",
            (*next_ctx_ptr).rip,
            (*next_ctx_ptr).cs,
            (*next_ctx_ptr).ss,
            (*next_ctx_ptr).rsp,
            (*next_ctx_ptr).rflags
        );
        log_exec_control_state("before_switch");
        crate::memory::manager::log_page_walk(
            x86_64::VirtAddr::new((*next_ctx_ptr).rip),
            "before_switch.rip",
        );

        serial_println!("context_switch.begin");
        send_eoi();
        crate::scheduler::Scheduler::switch_to(core::ptr::null_mut(), next_ctx_ptr);
        serial_println!("context_switch.end");
    }
}

#[no_mangle]
pub extern "C" fn page_fault_handler(stack_frame: &mut InterruptStackFrame, error_code: u64) {
    use x86_64::registers::control::Cr2;
    let fault_addr = Cr2::read_raw();
    let fault_rip = stack_frame.instruction_pointer.as_u64();
    let fault_rsp = stack_frame.stack_pointer.as_u64();
    let fault_cs = stack_frame.code_segment.0 as u64;
    let fault_ss = stack_frame.stack_segment.0 as u64;
    let fault_rflags = stack_frame.cpu_flags.bits();
    let fault_cs_rpl = fault_cs & 0x3;
    let fault_cs_kind = if fault_cs_rpl == 3 { "user" } else { "kernel" };

    let cur_pd = crate::core_local::CoreLocal::get().current_pd();
    serial_println!(
        "DEBUG: page_fault_handler entered. Addr={:#x}, RIP={:#x}, CS={:#x}({}), RFLAGS={:#x}, RSP={:#x}, SS={:#x}, Err={:#x}, PD={}",
        fault_addr,
        fault_rip,
        fault_cs,
        fault_cs_kind,
        fault_rflags,
        fault_rsp,
        fault_ss,
        error_code,
        cur_pd
    );
    serial_println!(
        "pf.frame.rip={:#x} pf.frame.cs={:#x} pf.frame.cs_kind={} pf.frame.rflags={:#x} pf.frame.rsp={:#x} pf.frame.ss={:#x} pf.cr2={:#x} pf.err={:#x} pf.pd={}",
        fault_rip,
        fault_cs,
        fault_cs_kind,
        fault_rflags,
        fault_rsp,
        fault_ss,
        fault_addr,
        error_code,
        cur_pd
    );
    crate::memory::manager::log_page_walk(stack_frame.instruction_pointer, "pf.rip");
    log_exec_control_state("page_fault");

    if fault_addr == 0 {
        panic!("Userland Null Pointer Jump at RIP: {:#x}", fault_rip);
    }

    // Phase 31: PKU Warden Trigger
    if (error_code & 0x20) != 0 {
        crate::pku::pku_warden(fault_addr, fault_rip, error_code);
        // For Phase 31, we panic on violation to provide clear debug output.
        // In production, this would terminate the faulting domain.
        panic!("PKU SECURITY VIOLATION ({} at {:#x})", 
               if (error_code & 0x10) != 0 { "EXECUTE" } else if (error_code & 0x02) != 0 { "WRITE" } else { "READ" },
               fault_addr);
    }

    serial_println!("EXCEPTION: PAGE FAULT at {:#x} (RIP: {:#x}, RSP: {:#x}, ERR: {:#x})", 
        fault_addr, fault_rip, fault_rsp, error_code);

    let core_id = crate::core_local::CoreLocal::get().core_id;
    let sched = &crate::scheduler::SCHEDULERS[core_id as usize];
    let current_ptr = sched.current_task.load(Ordering::Acquire);
    let mut task_id = 0;
    if !current_ptr.is_null() {
        unsafe { 
            let task = &mut *current_ptr;
            task_id = task.id;
            task.context.rip = fault_rip;
            task.context.rsp = fault_rsp;
            task.context.rflags = fault_rflags;
            task.state.store(crate::scheduler::STATE_BLOCKED, Ordering::Release);
            serial_println!(
                "task.faulted id={} pd_id={} frame_rip={:#x} frame_cs={:#x} frame_rsp={:#x} frame_rflags={:#x} err={:#x}",
                task.id,
                task.context.pd_id,
                task.context.rip,
                fault_cs,
                task.context.rsp,
                task.context.rflags,
                error_code
            );
        }
    }

    if let Err(e) = crate::ipc::pagefault::forward_page_fault(fault_addr, error_code as u32, task_id as u64) {
        serial_println!("EXCEPTION: Failed to forward #PF to sext: {}", e);
        if let Some(target_pd) = crate::ipc::DOMAIN_REGISTRY.get(task_id) {
            let _ = unsafe { &*target_pd.message_ring }.enqueue(MessageType::Signal(11));
        }
    }

    sched.tick();
    unsafe { send_eoi(); }
}

#[no_mangle]
pub extern "C" fn general_protection_fault_handler(stack_frame: &mut InterruptStackFrame, error_code: u64) {
    #[repr(C, packed)]
    struct Gdtr64 {
        limit: u16,
        base: u64,
    }
    let mut gdtr = Gdtr64 { limit: 0, base: 0 };
    let mut tr_sel: u16 = 0;
    unsafe {
        core::arch::asm!("sgdt [{}]", in(reg) &mut gdtr, options(nostack, preserves_flags));
        core::arch::asm!("str {0:x}", out(reg) tr_sel, options(nostack, preserves_flags));
    }
    let gdtr_base = unsafe { core::ptr::addr_of!(gdtr.base).read_unaligned() };
    let gdtr_limit = unsafe { core::ptr::addr_of!(gdtr.limit).read_unaligned() };

    let actual_rsp = unsafe { crate::scheduler::ACTUAL_IRET_RSP };
    let q0 = unsafe { crate::scheduler::ACTUAL_IRET_Q0_RIP };
    let q1 = unsafe { crate::scheduler::ACTUAL_IRET_Q1_CS };
    let q2 = unsafe { crate::scheduler::ACTUAL_IRET_Q2_RFLAGS };
    let q3 = unsafe { crate::scheduler::ACTUAL_IRET_Q3_RSP };
    let q4 = unsafe { crate::scheduler::ACTUAL_IRET_Q4_SS };

    serial_println!(
        "iret.actual rsp={:#x} q0.rip={:#x} q1.cs={:#x} q2.rflags={:#x} q3.rsp={:#x} q4.ss={:#x}",
        actual_rsp, q0, q1, q2, q3, q4
    );
    serial_println!(
        "iret.actual rsp.canonical={} rsp.align16={}",
        ((actual_rsp >> 48) == 0) || ((actual_rsp >> 48) == 0xffff),
        actual_rsp & 0xF
    );
    serial_println!("cpu.tables gdtr.base={:#x} gdtr.limit={:#x} tr={:#x}", gdtr_base, gdtr_limit, tr_sel);

    let gdt_base = gdtr_base as *const u64;
    let cs_idx = ((q1 & !0x7) >> 3) as isize;
    let ss_idx = ((q4 & !0x7) >> 3) as isize;
    let cs_raw = unsafe { *gdt_base.offset(cs_idx) };
    let ss_raw = unsafe { *gdt_base.offset(ss_idx) };
    serial_println!(
        "cpu.gdt.decode cs.sel={:#x} cs.raw={:#018x} ss.sel={:#x} ss.raw={:#018x}",
        q1, cs_raw, q4, ss_raw
    );

    let cur_pd = crate::core_local::CoreLocal::get().current_pd();
    serial_println!(
        "EXCEPTION: GP FAULT ENTER pd={} err={:#x} rip={:#x} rsp={:#x}",
        cur_pd,
        error_code,
        stack_frame.instruction_pointer.as_u64(),
        stack_frame.stack_pointer.as_u64()
    );
    panic!(
        "EXCEPTION: GP FAULT\nError: {:#x}\nRSP (Stack Pointer): {:#x}\nRIP (Instruction Pointer): {:#x}",
        error_code,
        stack_frame.stack_pointer.as_u64(),
        stack_frame.instruction_pointer.as_u64()
    );
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    let cur_pd = crate::core_local::CoreLocal::get().current_pd();
    serial_println!(
        "EXCEPTION: UD ENTER pd={} rip={:#x} rsp={:#x}",
        cur_pd,
        stack_frame.instruction_pointer.as_u64(),
        stack_frame.stack_pointer.as_u64()
    );
    panic!("EXCEPTION: INVALID OPCODE");
}

pub unsafe fn send_eoi() {
    let lapic_vaddr = crate::apic::LAPIC_ADDR.load(Ordering::Acquire);
    if lapic_vaddr != 0 {
        (lapic_vaddr as *mut u32).offset(0x0B0 / 4).write_volatile(0);
    } else {
        core::arch::asm!("mov al, 0x20", "out 0x20, al", "out 0xa0, al");
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    serial_println!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
    panic!("DOUBLE FAULT");
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let scancode = crate::keyboard::read_scancode();
    // Push into lock-free global queue; discard if full (to prevent blocking Ring-0)
    let _ = INPUT_RING.enqueue(scancode);
    
    unsafe { send_eoi(); }
}
extern "x86-interrupt" fn generic_irq_handler(_stack_frame: InterruptStackFrame) { route_interrupt(0x22); }
extern "x86-interrupt" fn revoke_key_handler(_stack_frame: InterruptStackFrame) {
    crate::hal::tlb_flush_local();
    unsafe { send_eoi(); }
}

fn route_interrupt(vector: u8) {
    let pd_id = VECTOR_OWNERS[vector as usize].load(Ordering::Acquire);
    if pd_id == 0 { unsafe { send_eoi(); } return; }
    if let Some(target_pd) = crate::ipc::DOMAIN_REGISTRY.get(pd_id) {
        let msg = MessageType::HardwareInterrupt { vector, data: 0 };
        let _ = unsafe { &*target_pd.message_ring }.enqueue(msg);
        let main_task = target_pd.main_task.load(Ordering::Acquire);
        if !main_task.is_null() { crate::scheduler::unpark_thread(main_task); }
    }
    unsafe { send_eoi(); }
}
