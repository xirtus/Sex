use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use crate::serial_println;
use crate::gdt;
use crate::capability::ProtectionDomain;
use crate::ipc_ring::RingBuffer;
use lazy_static::lazy_static;
use spin::Mutex;
use alloc::sync::Arc;

/// The event structure for a page fault.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PageFaultEvent {
    pub addr: u64,
    pub error_code: u64,
}

/// The event structure for a hardware interrupt.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct InterruptEvent {
    pub irq: u8,
    pub vector: u8,
}

lazy_static! {
    /// The global asynchronous queue for Page Faults.
    pub static ref SEXT_QUEUE: RingBuffer<PageFaultEvent, 256> = RingBuffer::new();
    
    /// The global asynchronous queue for Hardware Interrupts.
    /// This is the "prestep" for user-space sexdrives.
    pub static ref INTERRUPT_QUEUE: RingBuffer<InterruptEvent, 1024> = RingBuffer::new();
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
        
        // Keyboard (or other I/O) at 0x21
        idt[0x21].set_handler_fn(keyboard_interrupt_handler);
        
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

    let event = PageFaultEvent {
        addr: fault_addr.as_u64(),
        error_code: error_code.bits(),
    };

    let _ = SEXT_QUEUE.enqueue(event);
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Timer is handled by the kernel scheduler, but we still send EOI
    unsafe {
        core::arch::asm!("mov al, 0x20", "out 0x20, al", "out 0xa0, al");
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Asynchronous Forwarding to User-Space sexdrive
    let event = InterruptEvent {
        irq: 1, // Keyboard is IRQ 1
        vector: 0x21,
    };

    // Fast enqueue and EOI. No context switch here!
    let _ = INTERRUPT_QUEUE.enqueue(event);

    unsafe {
        // EOI to Local APIC (handled in apic module in a real system)
        core::arch::asm!("mov al, 0x20", "out 0x20, al", "out 0xa0, al");
    }
}
