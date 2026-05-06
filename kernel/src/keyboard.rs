use x86_64::instructions::port::Port;

pub fn init() {
    unsafe {
        let mut cmd_port: Port<u8> = Port::new(0x64);
        let mut data_port: Port<u8> = Port::new(0x60);

        // Read configuration byte
        cmd_port.write(0x20);
        let mut config = data_port.read();
        
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
}

/// Reads a single byte from the PS/2 keyboard data port
pub fn read_scancode() -> u8 {
    let mut port = Port::new(0x60);
    unsafe { port.read() }
}
