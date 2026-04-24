#![no_std]

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

#[inline(always)]
pub fn pdx_listen() -> (u64, MessageType) {
    (0, MessageType::Ping) 
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
pub const OP_WINDOW_CREATE: u64 = 0x104;
pub const OP_WINDOW_PAINT: u64 = 0x105;
pub const OP_SET_BG: u64 = 0x201;
pub const OP_RENDER_BAR: u64 = 0x200;

// Capability Slots
pub const SLOT_STORAGE: u64 = 1;
pub const SLOT_NETWORK: u64 = 2;
pub const SLOT_INPUT:   u64 = 3;
pub const SLOT_AUDIO:   u64 = 4;
pub const SLOT_DISPLAY: u64 = 5;
pub const SLOT_SHELL:   u64 = 6;
