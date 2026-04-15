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
        
        // 1. Request Keyboard (1) and Mouse (12) IRQs
        dde::dde_request_irq(1, Self::keyboard_handler)?;
        dde::dde_request_irq(12, Self::mouse_handler)?;

        Ok(())
    }

    pub extern "C" fn keyboard_handler(_arg: u64) -> u64 {
        unsafe {
            let scancode: u8 = x86_64::instructions::port::Port::new(0x60).read();
            // serial_println!("sexinput: Keyboard Scancode: {:#x}", scancode);
            
            // Push to the TTY server's input buffer
            crate::servers::tty::push_input(scancode);
        }
        0
    }

    pub extern "C" fn mouse_handler(_arg: u64) -> u64 {
        unsafe {
            let data: u8 = x86_64::instructions::port::Port::new(0x60).read();
            serial_println!("sexinput: Mouse Data: {:#x}", data);
        }
        0
    }
}

pub extern "C" fn sexinput_entry(arg: u64) -> u64 {
    serial_println!("sexinput PDX: Received input request {:#x}", arg);
    0
}
