use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::serial_println;
use crate::gdt;
use crate::ipc_ring::RingBuffer;
use lazy_static::lazy_static;

use core::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use crate::ipc::messages::MessageType;

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

#[repr(C)]
pub struct SyscallRegs {
    pub rax: u64, pub rdi: u64, pub rsi: u64, pub rdx: u64,
    pub r10: u64, pub r8:  u64, pub r9:  u64, pub rcx: u64, pub r11: u64,
}

#[unsafe(naked)]
pub unsafe extern "C" fn syscall_entry() {
    core::arch::naked_asm!(
        "swapgs",
        "mov gs:[16], rsp", // Save User RSP
        "mov rsp, gs:[8]",  // Load Kernel RSP

        // 1. SAVE RETURN STATE IMMEDIATELY (Before PKRU clobbers rcx!)
        "push r11",         // Save User RFLAGS
        "push rcx",         // Save User RIP

        // 2. Save original syscall number
        "push rax",

        // 3. Read and Save User PKRU
        "xor ecx, ecx",
        "rdpkru",
        "push rax",         // Save PKRU mask to stack

        // 4. Enter God Mode (Clobbers eax, ecx, edx safely now)
        "xor eax, eax",
        "xor ecx, ecx",
        "xor edx, edx",
        "wrpkru",

        // 5. Save Callee-Saved Registers
        "push rbp", "push rbx", "push r12", "push r13", "push r14", "push r15",
        "mov rbp, rsp",

        // 6. Build SyscallRegs (Volatiles + Args)
        // Fetch original rax (syscall num) from stack for the struct.
        // Offset is 56 bytes (6 callee-saved pushes * 8 = 48, plus PKRU mask = 56)
        "mov rax, [rsp + 56]",
        "push r11", "push rcx", "push r9", "push r8", "push r10", "push rdx", "push rsi", "push rdi", "push rax",

        // Call Handler
        "mov rdi, rsp",     // Pointer to SyscallRegs
        "call syscall_handler",

        // 7. RESTORE C-ABI VOLATILES (Fixes the r9 Kernel State Leak)
        "pop rax",          // Retrieves the syscall return value
        "pop rdi", 
        "pop rsi", 
        "pop rdx", 
        "pop r10", 
        "pop r8", 
        "pop r9",           // r9 safely restored to user state!
        "pop rcx", 
        "pop r11",

        // 8. Restore Callee-Saved Registers
        "pop r15", "pop r14", "pop r13", "pop r12", "pop rbx", "pop rbp",

        // 9. Restore PKRU Mask
        "pop rdi",          // Pop original PKRU mask into rdi
        "push rax",         // Stash the handler's return value safely on the stack
        "mov rax, rdi",     // Move PKRU mask to rax for wrpkru
        "xor ecx, ecx",     // Satisfy wrpkru requirements
        "xor edx, edx",
        "wrpkru",           // User PKRU restored
        "pop rax",          // Restore handler's return value back into rax

        // 10. Clean up and Return
        "add rsp, 8",       // Discard the saved original syscall number
        "pop rcx",          // Uncorrupted User RIP restored!
        "pop r11",          // Uncorrupted User RFLAGS restored!

        "mov rsp, gs:[16]", // Restore User RSP
        "swapgs",
        "sysretq"
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

        "xor eax, eax",
        "xor ecx, ecx",
        "xor edx, edx",
        "wrpkru",

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
        "iretq"
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

        "xor eax, eax",
        "xor ecx, ecx",
        "xor edx, edx",
        "wrpkru",

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
        "iretq"
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

        "xor eax, eax",
        "xor ecx, ecx",
        "xor edx, edx",
        "wrpkru",

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
        "iretq"
    );
}

// ── Rust Handlers ───────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn timer_interrupt_handler(stack_frame: &mut InterruptStackFrame) {
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

    let pd_id = unsafe { (*next_ctx_ptr).pd_id };
    crate::core_local::CoreLocal::get().set_pd(pd_id as u32);
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
            
            use core::sync::atomic::Ordering;
            old_ctx.pkru = (*old_ctx.pd_ptr).current_pkru_mask.load(Ordering::Relaxed) as u64;
        }
        
        let next_rip = (*next_ctx_ptr).rip;
        if next_rip == 0 {
            panic!("SCHED: null RIP for PD {}", (*next_ctx_ptr).pd_id);
        }

        let kstack_top = (*next_ctx_ptr).kstack_top;
        crate::gdt::update_tss_rsp0(x86_64::VirtAddr::new(kstack_top));

        serial_println!("SCHED: Switching to PD {} (RIP={:#x}, RSP={:#x}, KSTACK={:#x})", 
                        (*next_ctx_ptr).pd_id, (*next_ctx_ptr).rip, (*next_ctx_ptr).rsp, kstack_top);

        send_eoi();
        crate::scheduler::Scheduler::switch_to(core::ptr::null_mut(), next_ctx_ptr);
    }
}

#[no_mangle]
pub extern "C" fn page_fault_handler(stack_frame: &mut InterruptStackFrame, error_code: u64) {
    use x86_64::registers::control::Cr2;
    let fault_addr = Cr2::read_raw();

    serial_println!("DEBUG: page_fault_handler entered. Addr={:#x}, RIP={:#x}, Err={:#x}", 
                    fault_addr, stack_frame.instruction_pointer.as_u64(), error_code);

    if fault_addr == 0 {
        panic!("Userland Null Pointer Jump at RIP: {:#x}", stack_frame.instruction_pointer.as_u64());
    }

    // Phase 31: PKU Warden Trigger
    if (error_code & 0x20) != 0 {
        crate::pku::pku_warden(fault_addr, stack_frame.instruction_pointer.as_u64(), error_code);
        // For Phase 31, we panic on violation to provide clear debug output.
        // In production, this would terminate the faulting domain.
        panic!("PKU SECURITY VIOLATION ({} at {:#x})", 
               if (error_code & 0x10) != 0 { "EXECUTE" } else if (error_code & 0x02) != 0 { "WRITE" } else { "READ" },
               fault_addr);
    }

    serial_println!("EXCEPTION: PAGE FAULT at {:#x} (RIP: {:#x}, RSP: {:#x}, ERR: {:#x})", 
        fault_addr, stack_frame.instruction_pointer.as_u64(), stack_frame.stack_pointer.as_u64(), error_code);

    let core_id = crate::core_local::CoreLocal::get().core_id;
    let sched = &crate::scheduler::SCHEDULERS[core_id as usize];
    let current_ptr = sched.current_task.load(Ordering::Acquire);
    let mut task_id = 0;
    if !current_ptr.is_null() {
        unsafe { 
            let task = &mut *current_ptr;
            task_id = task.id;
            task.context.rip = stack_frame.instruction_pointer.as_u64();
            task.context.rsp = stack_frame.stack_pointer.as_u64();
            task.context.rflags = stack_frame.cpu_flags.bits();
            task.state.store(crate::scheduler::STATE_BLOCKED, Ordering::Release);
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
    panic!(
        "EXCEPTION: GP FAULT\nError: {:#x}\nRSP (Stack Pointer): {:#x}\nRIP (Instruction Pointer): {:#x}",
        error_code,
        stack_frame.stack_pointer.as_u64(),
        stack_frame.instruction_pointer.as_u64()
    );
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
