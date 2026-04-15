use crate::serial_println;
use crate::servers::dde;
use crate::ipc_ring::SpscRing;

/// sexinput: libinput lifting for the Sex Microkernel.
/// Processes HID events (Mouse/Keyboard) for Wayland compositors.

/// Standard Input Event structure (Inspired by Linux evdev).
#[repr(C)]
pub struct InputEvent {
    pub ev_type: u16,
    pub code: u16,
    pub value: i32,
}

pub struct sexinput {
    pub name: &'static str,
    /// Event ring buffer for the compositor (Zero-Copy).
    pub event_queue: SpscRing<InputEvent>, 
}

impl sexinput {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            event_queue: SpscRing::new(),
        }
    }

    /// Real PS/2 Controller initialization and IRQ handling.
    pub fn init(&mut self) -> Result<(), &'static str> {
        serial_println!("sexinput: Initializing PS/2 Controller for {}...", self.name);
        
        // 1. Register with the kernel's IRQ Routing Table for Keyboard (Vector 0x21)
        let ring = Arc::new(self.event_queue.clone());
        crate::interrupts::register_irq_route(0x21, 0, ring); // PD 0 for kernel-bootstrap input

        Ok(())
    }

    /// Process events from the IRQ ring buffer.
    pub fn run_loop(&self) {
        loop {
            if let Some(event) = self.event_queue.dequeue() {
                // In this prototype, the event.irq is the scancode for simplicity
                // or we read directly from the port if needed.
                unsafe {
                    let scancode: u8 = x86_64::instructions::port::Port::new(0x60).read();
                    crate::servers::tty::push_input(scancode);
                }
            }
            x86_64::instructions::hlt();
        }
    }
}

pub extern "C" fn sexinput_entry(arg: u64) -> u64 {
    serial_println!("sexinput PDX: Received input request {:#x}", arg);
    0
}
