use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use crate::serial_println;
use crate::gdt;
use crate::capability::ProtectionDomain;
use crate::ipc_ring::RingBuffer;
use crate::ipc_ring::SpscRing;
use lazy_static::lazy_static;
use conquer_once::spin::Mutex;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use crate::ipc::DOMAIN_REGISTRY;
use crate::ipc::messages::MessageType;

/// The event structure for a page fault.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PageFaultEvent {
    pub addr: u64,
    pub error_code: u64,
    pub task_id: u32,
}

/// The event structure for a hardware interrupt.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct InterruptEvent {
    pub irq: u8,
    pub vector: u8,
}

/// The event structure for a system-wide fault (Security or #PF).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SystemFaultEvent {
    pub pd_id: u32,
    pub fault_addr: u64,
    pub fault_type: u8, // 0: #PF, 1: CAP_VIOLATION
}

lazy_static! {
    /// Global Uptime Ticks (incremented by LAPIC timer).
    pub static ref TICKS: AtomicU64 = AtomicU64::new(0);

    /// The global asynchronous queue for Page Faults (sext pager).
    pub static ref SEXT_QUEUE: RingBuffer<PageFaultEvent, 256> = RingBuffer::new();
    
    /// The global fault interception ring for sexit (PID 1).
    pub static ref FAULT_RING: RingBuffer<SystemFaultEvent, 128> = RingBuffer::new();
}

/// Static mapping for Hardware Vector -> Driver PD.
/// IPCtax: Lock-free interrupt delivery.
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
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        
        // Timer at 0x20
        idt[0x20].set_handler_fn(timer_interrupt_handler);
        
        // Map hardware vectors (0x21 to 0x30) to generic_irq_handler
        idt[0x21].set_handler_fn(keyboard_interrupt_handler);
        idt[0x22].set_handler_fn(generic_irq_handler); 
        
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame, _error_code: u64) -> ! 
{
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;
    let fault_addr = Cr2::read();

    // 1. Identify current faulting task
    let mut task_id = 0;
    let core_id = crate::core_local::CoreLocal::get().core_id;
    let sched = &crate::scheduler::SCHEDULERS[core_id as usize];
    let current_ptr = sched.current_task.load(Ordering::Acquire);
    if !current_ptr.is_null() {
        unsafe {
            let task = &mut *current_ptr;
            task_id = task.id;
            task.context.rip = stack_frame.instruction_pointer.as_u64();
            task.context.rsp = stack_frame.stack_pointer.as_u64();
            task.context.rflags = stack_frame.cpu_flags.as_u64();
            task.state.store(crate::scheduler::STATE_BLOCKED, Ordering::Release);
        }
    }

    // 2. Async #PF Forwarding via safe_pdx_call (IPCtax mandate)
    if let Err(e) = crate::ipc::pagefault::forward_page_fault(fault_addr.as_u64(), error_code.bits() as u32, task_id as u64) {
        serial_println!("EXCEPTION: Failed to forward #PF to sext: {}", e);
    }

    // 3. Trigger Scheduler to pick next task
    sched.tick();
    unsafe { send_eoi(); }
}

pub unsafe fn send_eoi() {
    let lapic_vaddr = crate::apic::LAPIC_ADDR.load(core::sync::atomic::Ordering::Acquire);
    if lapic_vaddr != 0 {
        let lapic_ptr = lapic_vaddr as *mut u32;
        let eoi_reg = lapic_ptr.offset(0x0B0 / 4);
        eoi_reg.write_volatile(0);
    } else {
        core::arch::asm!("mov al, 0x20", "out 0x20, al", "out 0xa0, al");
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(stack_frame: InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);
    let core_id = crate::core_local::CoreLocal::get().core_id;
    let sched = &crate::scheduler::SCHEDULERS[core_id as usize];
    if let Some((old_ctx_ptr, next_ctx_ptr)) = sched.tick() {
        unsafe {
            let old_ctx = &mut *old_ctx_ptr;
            old_ctx.rip = stack_frame.instruction_pointer.as_u64();
            old_ctx.rsp = stack_frame.stack_pointer.as_u64();
            old_ctx.rflags = stack_frame.cpu_flags.as_u64();
            send_eoi();
            crate::scheduler::Scheduler::switch_to(old_ctx_ptr, next_ctx_ptr);
        }
    }
    unsafe { send_eoi(); }
}

extern "x86-interrupt" fn generic_irq_handler(_stack_frame: InterruptStackFrame) {
    route_interrupt(0x22);
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    route_interrupt(0x21);
}

fn route_interrupt(vector: u8) {
    let pd_id = VECTOR_OWNERS[vector as usize].load(Ordering::Acquire);
    if pd_id == 0 {
        unsafe { send_eoi(); }
        return;
    }

    if let Some(target_pd) = DOMAIN_REGISTRY.get(pd_id) {
        let msg = MessageType::HardwareInterrupt { vector, data: 0 };
        let _ = target_pd.message_ring.enqueue(msg);
        
        // Wake the driver trampoline/main thread
        let main_task = target_pd.main_task.load(Ordering::Acquire);
        if !main_task.is_null() {
            crate::scheduler::unpark_thread(main_task);
        }
    }

    unsafe { send_eoi(); }
}
