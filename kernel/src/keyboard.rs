use x86_64::instructions::port::Port;
use core::sync::atomic::{AtomicBool, Ordering};

/// True if i8042 PS/2 controller probed successfully and keyboard was initialized.
static KEYBOARD_READY: AtomicBool = AtomicBool::new(false);

/// Probe i8042 controller at port 0x64 before attempting init.
/// Reads status port; 0xFF on absent hardware indicates no controller.
fn probe_i8042() -> bool {
    let mut status: Port<u8> = unsafe { Port::new(0x64) };
    let s = unsafe { status.read() };
    if s == 0xFF {
        return false;
    }
    true
}

pub fn init() {
    if !probe_i8042() {
        return;
    }
    unsafe {
        let mut cmd_port: Port<u8> = Port::new(0x64);
        let mut data_port: Port<u8> = Port::new(0x60);

        // Read configuration byte
        cmd_port.write(0x20);
        let mut config = data_port.read();

        // If config reads as 0xFF, controller is absent
        if config == 0xFF {
            return;
        }

        // Enable IRQ1 (bit 0) and translate (bit 6)
        config |= 0x41;
        // Clear disable keyboard (bit 4)
        config &= !0x10;

        // Write configuration byte
        cmd_port.write(0x60);
        data_port.write(config);

        // Enable keyboard port
        cmd_port.write(0xAE);

        // Enable scanning
        data_port.write(0xF4);
        let _ack = data_port.read(); // Read ACK
    }
    KEYBOARD_READY.store(true, Ordering::Release);
}

/// Returns true if the PS/2 keyboard was successfully probed and initialized.
pub fn is_keyboard_ready() -> bool {
    KEYBOARD_READY.load(Ordering::Acquire)
}

/// Reads a single byte from the PS/2 keyboard data port.
/// Returns None if keyboard was not initialized.
pub fn read_scancode() -> Option<u8> {
    if !KEYBOARD_READY.load(Ordering::Acquire) {
        return None;
    }
    let mut port: Port<u8> = Port::new(0x60);
    Some(unsafe { port.read() })
}

/// Legacy raw read (for callers that don't check readiness — returns 0 if absent).
pub fn read_scancode_raw() -> u8 {
    let mut port: Port<u8> = Port::new(0x60);
    unsafe { port.read() }
}
