use x86_64::instructions::port::Port;

/// Reads a single byte from the PS/2 keyboard data port
pub fn read_scancode() -> u8 {
    let mut port = Port::new(0x60);
    unsafe { port.read() }
}
