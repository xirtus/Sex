#![no_std]

/// Userland serial writer — calls kernel raw_print (syscall 0, opcode 69).
pub struct SerialWriter;

impl core::fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        pdx_call(0, 69, s.as_ptr() as u64, s.len() as u64, 0);
        Ok(())
    }
}

#[macro_export]
macro_rules! serial_println {
    ($($arg:tt)*) => ({
        use core::fmt::Write as _;
        let mut _w = $crate::SerialWriter;
        let _ = core::write!(_w, $($arg)*);
        let _ = core::write!(_w, "\n");
    });
}

/// PDX message returned by pdx_listen. type_id == 0 means EMPTY (never a valid payload).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PdxMessage {
    pub type_id:   u64,
    pub arg0:      u64,
    pub arg1:      u64,
    pub arg2:      u64,
    pub caller_pd: u32,
    _pad:          u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C, u64)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MessageType {
    Ping = 0,
    Yield = 1,
    DisplayPrimaryFramebuffer { 
        virt_addr: u64, 
        width: u32, 
        height: u32 
    } = 0x103,
    CompositorCommit = 0x100,
    CompositorCreateWin = 0x101,
    WindowCreate = 0x104,
    HIDEvent {
        code: u64,
        value: u64,
    } = 0x202,
}

pub struct PdxEvent; // Stub

/// Spin-receive: returns next message with type_id != 0.
/// type_id == 0x00 is reserved as EMPTY and never returned.
pub fn pdx_listen() -> PdxMessage {
    loop {
        let type_id: u64;
        let caller_pd: u64;
        let arg0: u64;
        let arg1: u64;
        let arg2: u64;
        unsafe {
            core::arch::asm!(
                "syscall",
                in("rax") 28u64,
                lateout("rax") type_id,    // 0 = EMPTY, non-zero = valid
                lateout("rsi") caller_pd,
                lateout("rdx") arg0,
                lateout("r10") arg1,
                lateout("r8")  arg2,
                out("rcx") _,
                out("r11") _,
            );
        }
        if type_id == 0 {
            core::hint::spin_loop();
            continue;
        }
        return PdxMessage {
            type_id,
            arg0,
            arg1,
            arg2,
            caller_pd: caller_pd as u32,
            _pad: 0,
        };
    }
}

#[inline(always)]
pub fn pdx_reply(target_pd: u64) {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 1,
            in("rdi") target_pd,
        );
    }
}

pub fn sys_yield() {
    // Stub
}

pub fn sched_yield() {
    sys_yield();
}

pub fn pdx_call(slot: u64, opcode: u64, arg0: u64, arg1: u64, arg2: u64) -> u64 {
    let res: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 0, 
            in("rdi") slot,
            in("rsi") opcode,
            in("rdx") arg0,
            in("r10") arg1,
            in("r8")  arg2,
            lateout("rax") res,
        );
    }
    res
}

pub const SYSCALL_PDX_CALL: u64 = 0;
pub const SYSCALL_PDX_LISTEN: u64 = 2;
pub const SYSCALL_YIELD: u64 = 1;

// Capability slots — single source of truth for all PDs
pub const SLOT_STORAGE: u64 = 1;
pub const SLOT_NETWORK: u64 = 2;
pub const SLOT_INPUT:   u64 = 3;
pub const SLOT_AUDIO:   u64 = 4;
pub const SLOT_DISPLAY: u64 = 5;
pub const SLOT_SHELL:   u64 = 6;

// sexdisplay (SLOT_DISPLAY) opcodes
pub const OP_WINDOW_CREATE:  u64 = 0xDE;
pub const OP_WINDOW_PAINT:   u64 = 0xDF;
pub const OP_WINDOW_DESTROY: u64 = 0xE0;
pub const OP_MOVE_WINDOW:    u64 = 0xEE;
pub const OP_RESIZE_WINDOW:  u64 = 0xEF;

// silk-shell (SLOT_SHELL) opcodes
pub const OP_SET_BG:      u64 = 0x100;
pub const OP_RENDER_BAR:  u64 = 0x101;
