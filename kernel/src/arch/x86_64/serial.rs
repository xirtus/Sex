use core::fmt;
use x86::io::{outb, inb};

pub struct SerialPort { port: u16 }
impl SerialPort {
    pub const fn new(port: u16) -> Self { Self { port } }
    pub fn init(&self) {
        unsafe {
            outb(self.port + 1, 0x00);
            outb(self.port + 3, 0x80);
            outb(self.port + 0, 0x03);
            outb(self.port + 1, 0x00);
            outb(self.port + 3, 0x03);
            outb(self.port + 2, 0xC7);
            outb(self.port + 4, 0x0B);
        }
    }
    pub fn send(&self, data: u8) {
        unsafe {
            while (inb(self.port + 5) & 0x20) == 0 {}
            outb(self.port, data);
        }
    }
}
impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() { self.send(byte); }
        Ok(())
    }
}
pub static mut COM1: SerialPort = SerialPort::new(0x3F8);

#[macro_export]
macro_rules! serial_println {
    ($($arg:tt)*) => {
        unsafe {
            use core::fmt::Write;
            let _ = writeln!($crate::arch::x86_64::serial::COM1, $($arg)*);
        }
    };
}
