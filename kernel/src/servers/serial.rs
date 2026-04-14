use crate::serial_println;
use x86_64::instructions::interrupts;

/// The Serial Server's entry point.
/// In a full implementation, this would be its own ELF binary.
/// Here, it's a function that will be executed in a dedicated Protection Domain.
pub extern "C" fn serial_server_entry(arg: u64) -> u64 {
    // This code runs with the Serial PD's PKU key.
    // It should be the ONLY place in the system allowed to access the UART hardware.
    
    // Convert the u64 argument to a char or string descriptor in a real system.
    // For this demonstration, we'll just treat the argument as a single byte to print.
    let byte = (arg & 0xFF) as u8;
    
    // Use the low-level serial driver directly
    // This assumes the kernel has granted the Serial PD the right to use I/O ports.
    // (In x86_64, this is controlled by the I/O Permission Bitmap in the TSS).
    
    // We can't use the kernel's serial_println macro directly if it uses a lock 
    // we don't have access to, but here it's part of the same binary for the demo.
    
    // Print the character via the serial port
    interrupts::without_interrupts(|| {
        let mut serial_port = unsafe { uart_16550::SerialPort::new(0x3F8) };
        serial_port.send(byte);
    });

    0 // Success
}
