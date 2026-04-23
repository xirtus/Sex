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
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
            
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

use x86_64::registers::model_specific::{LStar, Star, SFMask, Efer, EferFlags};
use x86_64::VirtAddr;

pub fn init_idt() {
    serial_println!("   → Loading IDTR...");
    IDT.load();
    serial_println!("   → IDTR loaded successfully");

    unsafe {
        serial_println!("   → Setting up LSTAR (Syscall Entry)...");
        LStar::write(VirtAddr::new(syscall_entry as *const () as u64));
        let selectors = crate::gdt::get_selectors();
        serial_println!("   → Setting up STAR...");
        Star::write(
            selectors.user_code_selector, 
            selectors.user_data_selector,
            selectors.code_selector,
            selectors.kernel_data_selector,
        ).unwrap();
        SFMask::write(x86_64::registers::rflags::RFlags::INTERRUPT_FLAG);
        Efer::write(Efer::read() | EferFlags::SYSTEM_CALL_EXTENSIONS);
        serial_println!("   → Syscall setup COMPLETE");
    }
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
        "mov gs:[16], rsp", 
        "mov rsp, gs:[8]",  
        
        "push r11", "push rcx", // 1st Save: Return RFLAGS and RIP (needed for sysretq)
        
        "xor ecx, ecx", "rdpkru", "push rax", // save PKRU (needs eax, edx, ecx)
        "xor eax, eax", "xor edx, edx", "xor ecx, ecx", "wrpkru", // God Mode (needs eax, edx, ecx)

        "push rbp", "push rbx", "push r12", "push r13", "push r14", "push r15",
        "mov rbp, rsp",
        
        // Push SyscallRegs layout (rax at top, r11 at bottom)
        "push r11", "push rcx", "push r9", "push r8", "push r10", "push rdx", "push rsi", "push rdi", "push rax",
        "mov rdi, rsp", 
        "call syscall_handler",

        // Restore SyscallRegs (allows handler to modify rdi, rsi, rdx, etc.)
        "pop rax", "pop rdi", "pop rsi", "pop rdx", "pop r10", "pop r8", "pop r9", "pop rcx", "pop r11",
        
        // Restore callee-saved registers
        "pop r15", "pop r14", "pop r13", "pop r12", "pop rbx", "pop rbp",
        
        // CRITICAL FIX: Pure stack-based PKRU swap (Zero Register Clobbering)
        "mov rcx, [rsp]",  // Peek at the PKRU mask on the stack
        "push rax",        // Safely stash the syscall result onto the stack
        "mov rax, rcx",    // Move PKRU mask to rax (wrpkru reads eax)
        "xor ecx, ecx",    // wrpkru requires ecx = 0
        "xor edx, edx",    // wrpkru requires edx = 0
        "wrpkru",          // Restore God Mode / PKU isolation
        
        "pop rax",         // Retrieve the syscall result back into rax for userland
        "add rsp, 8",      // Discard the old PKRU mask from the stack
        
        // Restore the original return state for sysretq
        "pop rcx",         // Restore User RIP
        "pop r11",         // Restore User RFLAGS
        
        "mov rsp, gs:[16]", "swapgs", "sysretq"
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
        "push r15", "push r14", "push r13", "push r12", "push r11", "push r10", "push r9", "push r8",
        "push rdi", "push rsi", "push rbp", "push rdx", "push rcx", "push rbx", "push rax",
        
        "mov rax, [rsp + 128]", "test al, 3", "jz 1f", "swapgs", "1:",
        "xor eax, eax", "xor edx, edx", "xor ecx, ecx", "wrpkru",
        
        "sub rsp, 8", // Align stack to 16-bytes (120+8)
        "lea rdi, [rsp + 128]", // stack_frame is now at [rsp+128]
        "call timer_interrupt_handler",
        "add rsp, 8",
        
        "mov rax, [rsp + 128]", "test al, 3", "jz 2f", "swapgs", "2:",
        "pop rax", "pop rbx", "pop rcx", "pop rdx", "pop rbp", "pop rsi", "pop rdi",
        "pop r8", "pop r9", "pop r10", "pop r11", "pop r12", "pop r13", "pop r14", "pop r15",
        "iretq"
    );
}

#[unsafe(naked)]
pub unsafe extern "C" fn page_fault_stub() {
    core::arch::naked_asm!(
        "push r15", "push r14", "push r13", "push r12", "push r11", "push r10", "push r9", "push r8",
        "push rdi", "push rsi", "push rbp", "push rdx", "push rcx", "push rbx", "push rax",
        
        "mov rax, [rsp + 136]", "test al, 3", "jz 1f", "swapgs", "1:",
        "xor eax, eax", "xor edx, edx", "xor ecx, ecx", "wrpkru",
        
        "lea rdi, [rsp + 128]", "mov rsi, [rsp + 120]", "call page_fault_handler",
        
        "mov rax, [rsp + 136]", "test al, 3", "jz 2f", "swapgs", "2:",
        "pop rax", "pop rbx", "pop rcx", "pop rdx", "pop rbp", "pop rsi", "pop rdi",
        "pop r8", "pop r9", "pop r10", "pop r11", "pop r12", "pop r13", "pop r14", "pop r15",
        "add rsp, 8", "iretq"
    );
}

#[unsafe(naked)]
pub unsafe extern "C" fn general_protection_fault_stub() {
    core::arch::naked_asm!(
        "push r15", "push r14", "push r13", "push r12", "push r11", "push r10", "push r9", "push r8",
        "push rdi", "push rsi", "push rbp", "push rdx", "push rcx", "push rbx", "push rax",
        "mov rax, [rsp + 136]", "test al, 3", "jz 1f", "swapgs", "1:",
        "xor eax, eax", "xor edx, edx", "xor ecx, ecx", "wrpkru",
        "lea rdi, [rsp + 128]", "mov rsi, [rsp + 120]", "call general_protection_fault_handler",
        "mov rax, [rsp + 136]", "test al, 3", "jz 2f", "swapgs", "2:",
        "pop rax", "pop rbx", "pop rcx", "pop rdx", "pop rbp", "pop rsi", "pop rdi",
        "pop r8", "pop r9", "pop r10", "pop r11", "pop r12", "pop r13", "pop r14", "pop r15",
        "add rsp, 8", "iretq"
    );
}

// ── Rust Handlers ───────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn timer_interrupt_handler(stack_frame: &mut InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);
    let core_id = crate::core_local::CoreLocal::get().core_id;
    let sched = &crate::scheduler::SCHEDULERS[core_id as usize];
    
    if let Some((old_ctx_ptr, next_ctx_ptr)) = sched.tick() {
        let pd_id = unsafe { (*next_ctx_ptr).pd_id };
        // Bug 1 fix: update CoreLocal so current_pd_ref() returns correct PD.
        crate::core_local::CoreLocal::get().set_pd(pd_id);
        unsafe {
            if !old_ctx_ptr.is_null() {
                let old_ctx = &mut *old_ctx_ptr;
                old_ctx.rip = stack_frame.instruction_pointer.as_u64();
                old_ctx.cs = stack_frame.code_segment.0 as u64;
                old_ctx.rflags = stack_frame.cpu_flags.bits();
                old_ctx.rsp = stack_frame.stack_pointer.as_u64();
                old_ctx.ss = stack_frame.stack_segment.0 as u64;
                // Bug 2 fix: user callee-saved regs are on the kernel stack, pushed by
                // timer_interrupt_stub before the InterruptStackFrame. The stub pushed
                // (high→low): r15,r14,r13,r12,r11,r10,r9,r8,rdi,rsi,rbp,rdx,rcx,rbx,rax
                // stack_frame sits at [rsp+120], so base.offset(-1) = r15 at [rsp+112], etc.
                let base = stack_frame as *const _ as *const u64;
                old_ctx.r15 = *base.offset(-1);
                old_ctx.r14 = *base.offset(-2);
                old_ctx.r13 = *base.offset(-3);
                old_ctx.r12 = *base.offset(-4);
                old_ctx.rbp = *base.offset(-11);
                old_ctx.rbx = *base.offset(-14);
                // switch_to reads PKRU after its own wrpkru(0), so it always saves 0.
                // Read the correct PKRU from the domain struct instead.
                use core::sync::atomic::Ordering;
                old_ctx.pkru = (*old_ctx.pd_ptr).current_pkru_mask.load(Ordering::Relaxed);
            }
            let next_rip = (*next_ctx_ptr).rip;
            if next_rip == 0 {
                panic!("SCHED: null RIP for PD {}", (*next_ctx_ptr).pd_id);
            }
            send_eoi();
            crate::scheduler::Scheduler::switch_to(old_ctx_ptr, next_ctx_ptr);
        }
    }
    unsafe { send_eoi(); }
}

#[no_mangle]
pub extern "C" fn page_fault_handler(stack_frame: &mut InterruptStackFrame, error_code: u64) {
    use x86_64::registers::control::Cr2;
    let fault_addr = Cr2::read_raw();

    if fault_addr == 0 {
        panic!("Userland Null Pointer Jump at RIP: {:#x}", stack_frame.instruction_pointer.as_u64());
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
    serial_println!("EXCEPTION: GP FAULT at {:#x}, Error: {:#x}", stack_frame.instruction_pointer.as_u64(), error_code);
    panic!("GENERAL PROTECTION FAULT");
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
