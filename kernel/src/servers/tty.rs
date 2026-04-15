use crate::serial_println;
use spin::Mutex;
use lazy_static::lazy_static;

/// Minimal TTY Server for SexOS.
/// Manages terminal state and line discipline.

#[repr(C)]
pub struct Termios {
    pub iflag: u32,
    pub oflag: u32,
    pub cflag: u32,
    pub lflag: u32,
    pub line: u8,
    pub cc: [u8; 32],
    pub ispeed: u32,
    pub ospeed: u32,
}

#[repr(C)]
pub struct WinSize {
    pub row: u16,
    pub col: u16,
    pub xpixel: u16,
    pub ypixel: u16,
}

use crate::ipc_ring::SpscRing;

pub struct TtyState {
    pub termios: Termios,
    pub winsize: WinSize,
    pub input_buffer: SpscRing<u8>,
}

lazy_static! {
    pub static ref DEFAULT_TTY: Mutex<TtyState> = Mutex::new(TtyState {
        termios: Termios {
            iflag: 0,
            oflag: 0,
            cflag: 0,
            lflag: 0x00000002 | 0x00000010, // ICANON | ECHO
            line: 0,
            cc: [0; 32],
            ispeed: 0,
            ospeed: 0,
        },
        winsize: WinSize {
            row: 24,
            col: 80,
            xpixel: 0,
            ypixel: 0,
        },
        input_buffer: SpscRing::new(),
    });
}

pub fn push_input(c: u8) {
    let tty = DEFAULT_TTY.lock();
    let _ = tty.input_buffer.enqueue(c);
}

pub fn read(buffer: *mut u8, count: usize) -> usize {
    let tty = DEFAULT_TTY.lock();
    let mut read_count = 0;
    
    while read_count < count {
        if let Some(c) = tty.input_buffer.dequeue() {
            unsafe { *buffer.add(read_count) = c; }
            read_count += 1;
        } else {
            break;
        }
    }
    
    read_count
}

pub fn write(buffer: *const u8, count: usize) -> usize {
    let data = unsafe { core::slice::from_raw_parts(buffer, count) };
    
    // In a real system, we'd parse ANSI escapes here and update VESA/GOP.
    // For the prototype, we log the write and forward to serial.
    for &c in data {
        if c == 0x1B { // ESC
            serial_println!("TTY: [ANSI] Escape sequence detected.");
        }
        crate::serial::print_char(c as char);
    }
    
    count
}

pub fn handle_ioctl(fd: u32, request: u64, arg: u64) -> u64 {
    serial_println!("TTY: ioctl(fd: {}, request: {:#x})", fd, request);
    
    match request {
        0x5401 => { // TCGETS
            let tty = DEFAULT_TTY.lock();
            unsafe {
                core::ptr::copy_nonoverlapping(
                    &tty.termios as *const Termios,
                    arg as *mut Termios,
                    1
                );
            }
            0
        },
        0x5402 => { // TCSETS
            let mut tty = DEFAULT_TTY.lock();
            unsafe {
                core::ptr::copy_nonoverlapping(
                    arg as *const Termios,
                    &mut tty.termios as *mut Termios,
                    1
                );
            }
            0
        },
        0x5413 => { // TIOCGWINSZ
            let tty = DEFAULT_TTY.lock();
            unsafe {
                core::ptr::copy_nonoverlapping(
                    &tty.winsize as *const WinSize,
                    arg as *mut WinSize,
                    1
                );
            }
            0
        },
        _ => {
            serial_println!("TTY: Unknown ioctl {:#x}", request);
            u64::MAX
        }
    }
}
