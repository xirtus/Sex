use crate::serial_println;
use crate::interrupts::INTERRUPT_QUEUE;

/// The Input Driver's entry point.
/// Polls the asynchronous interrupt queue for hardware events.
pub extern "C" fn input_driver_entry(_arg: u64) -> u64 {
    serial_println!("INPUT: Driver started. Polling for hardware events...");

    loop {
        // Poll the global asynchronous interrupt queue
        if let Some(event) = INTERRUPT_QUEUE.dequeue() {
            serial_println!("INPUT: Dequeued Hardware Event - IRQ: {}, Vector: {:#x}", 
                event.irq, event.vector);
            
            if event.irq == 1 {
                // Read from keyboard I/O port (0x60)
                // In a real system, the Input Driver PD would have 
                // I/O port permissions granted via TSS.
                unsafe {
                    let scancode: u8;
                    core::arch::asm!("in al, 0x60", out("al") scancode);
                    serial_println!("INPUT: Keyboard Scancode: {:#x}", scancode);
                }
            }
        }

        // In a real system, we'd sleep or yield if the queue is empty
        // to avoid 100% CPU usage.
        x86_64::instructions::nop();
    }
}
