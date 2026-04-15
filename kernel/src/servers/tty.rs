use crate::serial_println;
use spin::Mutex;
use lazy_static::lazy_static;

/// Minimal TTY Server for SexOS.
/// Manages terminal state and line discipline.

#[repr(C)]
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
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
    pub line_buffer: [u8; 256],
    pub line_pos: usize,
}

lazy_static! {
    pub static ref DEFAULT_TTY: Mutex<TtyState> = Mutex::new(TtyState {
        termios: Termios {
            iflag: 0,
            oflag: 0,
            cflag: 0,
            lflag: 0x00000002 | 0x00000008, // ICANON | ECHO
            line: 0,
            cc: [0; 32],
            ispeed: 38400,
            ospeed: 38400,
        },
        winsize: WinSize {
            row: 24,
            col: 80,
            xpixel: 0,
            ypixel: 0,
        },
        input_buffer: SpscRing::new(),
        line_buffer: [0; 256],
        line_pos: 0,
    });
}

/// Scancode Set 1 (US QWERTY) to ASCII mapping.
const SCANCODE_SET1: [u8; 128] = [
    0, 27, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'-', b'=', 8, // 14
    9, b'q', b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p', b'[', b']', 10, // 28
    0, b'a', b's', b'd', b'f', b'g', b'h', b'j', b'k', b'l', b';', b'\'', b'`', 0, // 41
    b'\\', b'z', b'x', b'c', b'v', b'b', b'n', b'm', b',', b'.', b'/', 0, b'*', 0, // 55
    b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 71
    0, 0, b'-', 0, 0, 0, b'+', 0, 0, 0, 0, 0, 0, 0, 0, 0, // 87
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 103
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 119
    0, 0, 0, 0, 0, 0, 0, 0, // 127
];

pub fn push_input(scancode: u8) {
    let mut tty = DEFAULT_TTY.lock();
    
    // Ignore key releases (MSB set)
    if scancode & 0x80 != 0 {
        return;
    }

    let ascii = if (scancode as usize) < SCANCODE_SET1.len() {
        SCANCODE_SET1[scancode as usize]
    } else {
        0
    };

    if ascii == 0 {
        return;
    }

    // ECHO logic
    if tty.termios.lflag & 0x00000008 != 0 {
        crate::serial::print_char(ascii as char);
    }

    // ICANON (Line Discipline) logic
    if tty.termios.lflag & 0x00000002 != 0 {
        if ascii == 10 || ascii == 13 { // LF or CR
            // Commit line to buffer
            for i in 0..tty.line_pos {
                let _ = tty.input_buffer.enqueue(tty.line_buffer[i]);
            }
            let _ = tty.input_buffer.enqueue(b'\n');
            tty.line_pos = 0;
            if tty.termios.lflag & 0x00000008 != 0 {
                crate::serial::print_char('\n');
            }
        } else if ascii == 8 { // Backspace
            if tty.line_pos > 0 {
                tty.line_pos -= 1;
                if tty.termios.lflag & 0x00000008 != 0 {
                    crate::serial::print_char('\x08');
                    crate::serial::print_char(' ');
                    crate::serial::print_char('\x08');
                }
            }
        } else {
            if tty.line_pos < tty.line_buffer.len() {
                tty.line_buffer[tty.line_pos] = ascii;
                tty.line_pos += 1;
            }
        }
    } else {
        // Raw mode: Push immediately
        let _ = tty.input_buffer.enqueue(ascii);
    }
}

pub fn read(buffer: *mut u8, count: usize) -> usize {
    let mut tty = DEFAULT_TTY.lock();
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
        // Forward to serial port (The underlying Interaction Pillar)
        crate::serial::print_char(c as char);
    }
    
    count
}

pub fn handle_ioctl(fd: u32, request: u64, arg: u64) -> u64 {
    // serial_println!("TTY: ioctl(fd: {}, request: {:#x})", fd, request);
    
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

