use uart_16550::SerialPort;
use spinning_top::Spinlock;
use lazy_static::lazy_static;
use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};

/// True if COM1 at 0x3F8 probed successfully.
static SERIAL_PRESENT: AtomicBool = AtomicBool::new(false);

/// Probe COM1 (0x3F8) for existence before full init.
/// Writes to the scratch register (offset 7) and reads back.
/// If the port doesn't exist or is non-responsive, returns false.
fn probe_com1() -> bool {
    use x86_64::instructions::port::Port;
    let mut scratch: Port<u8> = unsafe { Port::new(0x3F8 + 7) };
    let probe_val: u8 = 0x5A;
    unsafe { scratch.write(probe_val); }
    let readback = unsafe { scratch.read() };
    readback == probe_val
}

lazy_static! {
    pub static ref SERIAL1: Spinlock<Option<SerialPort>> = {
        if probe_com1() {
            let mut serial_port = unsafe { SerialPort::new(0x3F8) };
            serial_port.init();
            SERIAL_PRESENT.store(true, Ordering::Release);
            Spinlock::new(Some(serial_port))
        } else {
            Spinlock::new(None)
        }
    };
}

/// Check if serial port was successfully probed and initialized.
pub fn is_serial_present() -> bool {
    SERIAL_PRESENT.load(Ordering::Acquire)
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        let mut guard = SERIAL1.lock();
        if let Some(ref mut port) = *guard {
            let _ = port.write_fmt(args);
        }
        // If no serial port, output is silently dropped (no panic on headless machines).
    });
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}
