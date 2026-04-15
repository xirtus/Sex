use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use crate::serial_println;
use crate::gdt;
use crate::capability::ProtectionDomain;
use crate::ipc_ring::RingBuffer;
use crate::ipc_ring::SpscRing;
use lazy_static::lazy_static;
use spin::Mutex;
use alloc::sync::Arc;
use alloc::vec::Vec;

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

/// Mapping from Hardware Vector to user-space PD Interrupt Ring.
pub struct VectorRouting {
    pub vector: u8,
    pub pd_id: u32,
    pub ring: Arc<SpscRing<InterruptEvent>>,
}

/// The event structure for a system-wide fault (Security or #PF).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SystemFaultEvent {
    pub pd_id: u32,
    pub fault_addr: u64,
    pub fault_type: u8, // 0: #PF, 1: CAP_VIOLATION
}

use core::sync::atomic::{AtomicU64, Ordering};

lazy_static! {
    /// Global Uptime Ticks (incremented by LAPIC timer).
    pub static ref TICKS: AtomicU64 = AtomicU64::new(0);

    /// The global asynchronous queue for Page Faults (sext pager).
    pub static ref SEXT_QUEUE: RingBuffer<PageFaultEvent, 256> = RingBuffer::new();
    
    /// The global fault interception ring for sexit (PID 1).
    pub static ref FAULT_RING: RingBuffer<SystemFaultEvent, 128> = RingBuffer::new();
    
    /// The global Dynamic IRQ Routing Table.
    pub static ref IRQ_ROUTING_TABLE: Mutex<Vec<VectorRouting>> = Mutex::new(Vec::new());
}

pub fn register_irq_route(vector: u8, pd_id: u32, ring: Arc<SpscRing<InterruptEvent>>) {
    let mut table = IRQ_ROUTING_TABLE.lock();
    table.push(VectorRouting { vector, pd_id, ring });
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
        // In a real system, we'd use individual stubs to capture the vector.
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
    unsafe {
        if let Some(ref mut sched) = crate::scheduler::SCHEDULERS[0] {
            if let Some(ref current_mutex) = sched.current_task {
                let mut task = current_mutex.lock();
                task_id = task.id;
                
                task.context.rip = stack_frame.instruction_pointer.as_u64();
                task.context.rsp = stack_frame.stack_pointer.as_u64();
                task.context.rflags = stack_frame.cpu_flags.as_u64();

                task.state = crate::scheduler::TaskState::Blocked;
            }
            sched.block_current();
        }
    }

    // 4. Notify sext PD via the asynchronous ring buffer
    let event = PageFaultEvent {
        addr: fault_addr.as_u64(),
        error_code: error_code.bits(),
        task_id,
    };

    if SEXT_QUEUE.enqueue(event).is_err() {
        serial_println!("EXCEPTION: Page Fault Queue FULL. Dropping fault for Task {}.", task_id);
    }

    // 5. Trigger Scheduler to pick next task (since current is blocked)
    unsafe {
        if let Some(ref mut sched) = crate::scheduler::SCHEDULERS[0] {
            sched.tick();
        }
        send_eoi();
    }
}

pub unsafe fn send_eoi() {
    if let Some(lapic_virt) = crate::apic::LAPIC_ADDR.lock().as_ref() {
        let lapic_ptr = lapic_virt.as_u64() as *mut u32;
        let eoi_reg = lapic_ptr.offset(0x0B0 / 4);
        eoi_reg.write_volatile(0);
    } else {
        core::arch::asm!("mov al, 0x20", "out 0x20, al", "out 0xa0, al");
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(stack_frame: InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);
    unsafe {
        if let Some(ref mut sched) = crate::scheduler::SCHEDULERS[0] {
            if let Some((old_ctx_ptr, next_ctx_ptr)) = sched.tick() {
                // 1. Save state from the interrupt stack frame into the old context
                let old_ctx = &mut *old_ctx_ptr;
                old_ctx.rip = stack_frame.instruction_pointer.as_u64();
                old_ctx.rsp = stack_frame.stack_pointer.as_u64();
                old_ctx.rflags = stack_frame.cpu_flags.as_u64();
                
                // 2. Perform EOI before the switch, as iretq will re-enable interrupts
                send_eoi();

                // 3. Jump to the next task (Naked switch with iretq)
                crate::scheduler::Scheduler::switch_to(old_ctx_ptr, next_ctx_ptr);
            }
        }
        send_eoi();
    }
}

extern "x86-interrupt" fn generic_irq_handler(_stack_frame: InterruptStackFrame) {
    // Prototype: Routing for Vector 0x22 (e.g. NVMe/NIC)
    route_interrupt(0x22);
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Routing for Vector 0x21
    route_interrupt(0x21);
}

fn route_interrupt(vector: u8) {
    let event = InterruptEvent {
        irq: vector - 0x20,
        vector,
    };

    let table = IRQ_ROUTING_TABLE.lock();
    for route in table.iter().filter(|r| r.vector == vector) {
        let _ = route.ring.enqueue(event);
    }

    unsafe {
        send_eoi();
    }
}
