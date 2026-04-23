//! sex-pdx: Single source of truth for PDX capability slots, opcodes, syscalls, and ABI types
//! for the Sex microkernel SASOS, protected by physical Intel MPK (PKU/PKEY)
//! on all 10th gen and up hardware locks for PDX memory isolation.

#![no_std]

use core::arch::asm;

// === Syscall numbers (rax) — Phase 25 alignment ===
pub const SYSCALL_PDX_CALL: u64  = 0;
pub const SYSCALL_PDX_LISTEN: u64 = 28;
pub const SYSCALL_YIELD: u64      = 32;
pub const SYSCALL_PDX_REPLY: u64 = 29;

// === Standardized Capability Slots ===
pub const SLOT_KERNEL: u64 = 0;
pub const SLOT_STORAGE: u64 = 1;
pub const SLOT_NETWORK: u64 = 2;
pub const SLOT_INPUT: u64   = 3;
pub const SLOT_AUDIO: u64   = 4;
pub const SLOT_DISPLAY: u64 = 5;
pub const SLOT_SHELL: u64   = 6;
pub const SLOT_APP_START: u64 = 7;

// === Legacy Opcode Aliases (for transition) ===
pub const OP_WINDOW_CREATE: u64 = 0xDE;
pub const OP_WINDOW_DESTROY: u64 = 0xDF;
pub const OP_WINDOW_PAINT: u64  = 0xE0;
pub const OP_RENDER_BAR: u64    = 0x101;
pub const OP_SET_BG: u64        = 0x100;

pub const PDX_MAGIC: u32 = 0x53454C4B; // 'SELK' matching silkclient

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DisplayInfo {
    pub virt_addr: u64,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
}

#[macro_export]
macro_rules! serial_println {
    ($($arg:tt)*) => {};
}

/// The unified message type for all IPC in SexOS.
/// Uses #[repr(u64)] to bridge the gap between Rust variants and raw hardware PDX rings.
#[repr(u64)]
#[derive(Debug, Copy, Clone)]
pub enum MessageType {
    // --- 1. PDX Message Headers (Discriminants 0-9) ---
    Request = 0,
    Response = 1,
    Signal = 2,
    Interrupt = 3,

    // --- 2. System Event Variants (Data-carrying, starting at 0x10) ---
    RawCall { opcode: u64, arg0: u64, arg1: u64, arg2: u64 } = 0x10,
    DisplayPrimaryFramebuffer { virt_addr: u64, width: u32, height: u32, pitch: u32 } = 0x11,
    HIDEvent { ev_type: u16, code: u16, value: u16 } = 0x12,

    // --- 3. Display Protocol Opcodes (Discriminants 0xDD+) ---
    CompositorCommit = 0xDD,
    WindowCreate = 0xDE,
    SetWindowRoundness = 0xDF,
    SetWindowBlur = 0xE0,
    SetWindowAnimation = 0xE1,
    SetWindowDecorations = 0xE2,
    GetDisplayInfo = 0xE3,
    FocusWindow = 0xE4,
    MoveWindow = 0xEE,
    ResizeWindow = 0xEF,
}

impl MessageType {
    pub fn arg0(&self) -> u64 {
        match self {
            MessageType::RawCall { arg0, .. } => *arg0,
            MessageType::DisplayPrimaryFramebuffer { virt_addr, .. } => *virt_addr,
            MessageType::HIDEvent { ev_type, .. } => *ev_type as u64,
            _ => 0,
        }
    }

    pub fn arg1(&self) -> u64 {
        match self {
            MessageType::RawCall { arg1, .. } => *arg1,
            MessageType::DisplayPrimaryFramebuffer { width, height, .. } => (*width as u64) | ((*height as u64) << 32),
            MessageType::HIDEvent { code, .. } => *code as u64,
            _ => 0,
        }
    }

    pub fn arg2(&self) -> u64 {
        match self {
            MessageType::RawCall { arg2, .. } => *arg2,
            MessageType::DisplayPrimaryFramebuffer { pitch, .. } => *pitch as u64,
            MessageType::HIDEvent { value, .. } => *value as u64,
            _ => 0,
        }
    }
}

// === PdxEvent wrapper for safe ring-3 consumption ===
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct PdxEvent {
    pub msg_type: MessageType,
    pub caller_pd: u32,
}

impl PdxEvent {
    pub fn opcode(&self) -> u64 {
        match self.msg_type {
            MessageType::RawCall { opcode, .. } => opcode,
            // For fieldless variants, the discriminant is the opcode
            m => {
                let ptr = &m as *const MessageType as *const u64;
                unsafe { *ptr }
            }
        }
    }
}

/// Generic PDX syscall wrapper
pub unsafe fn pdx_call(slot: u32, opcode: u64, arg0: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
    let ret: u64;
    asm!(
        "syscall",
        in("rax") SYSCALL_PDX_CALL,
        in("rdi") slot,
        in("rsi") opcode,
        in("rdx") arg0,
        in("r10") arg1,
        in("r8") arg2,
        in("r9") arg3,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
    );
    ret
}

/// Reconstruct a MessageType from raw register values (Phase 25 ABI)
pub fn reconstruct_msg(opcode: u64, a0: u64, a1: u64, a2: u64) -> MessageType {
    match opcode {
        0x10 => MessageType::RawCall { opcode: a0, arg0: a1, arg1: a2, arg2: 0 },
        0x11 => MessageType::DisplayPrimaryFramebuffer {
            virt_addr: a0,
            width: (a1 & 0xFFFF_FFFF) as u32,
            height: (a1 >> 32) as u32,
            pitch: (a2 & 0xFFFF_FFFF) as u32,
        },
        0x12 => MessageType::HIDEvent {
            ev_type: (a0 & 0xFFFF) as u16,
            code: (a1 & 0xFFFF) as u16,
            value: (a2 & 0xFFFF) as u16,
        },
        // Map common display opcodes
        0xDD => MessageType::CompositorCommit,
        0xDE => MessageType::WindowCreate,
        0xDF => MessageType::SetWindowRoundness,
        0xE0 => MessageType::SetWindowBlur,
        0xE1 => MessageType::SetWindowAnimation,
        0xE2 => MessageType::SetWindowDecorations,
        0xE3 => MessageType::GetDisplayInfo,
        0xE4 => MessageType::FocusWindow,
        0xEE => MessageType::MoveWindow,
        0xEF => MessageType::ResizeWindow,
        _ => MessageType::RawCall { opcode, arg0: a0, arg1: a1, arg2: a2 },
    }
}

/// Listen for incoming PDX messages on a specific slot (Raw)
#[deprecated(note = "Use zero-argument pdx_listen() for standard SASOS messaging")]
pub unsafe fn pdx_listen_raw(slot: u32) -> PdxEvent {
    let sender: u64;
    let opcode: u64;
    let a0: u64;
    let a1: u64;
    let a2: u64;
    asm!(
        "syscall",
        in("rax") SYSCALL_PDX_LISTEN,
        in("rdi") slot,
        lateout("rax") sender,
        lateout("rsi") opcode,
        lateout("rdx") a0,
        lateout("r10") a1,
        lateout("r8") a2,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
    );
    
    PdxEvent {
        msg_type: reconstruct_msg(opcode, a0, a1, a2),
        caller_pd: sender as u32,
    }
}

/// Phase 25 Standard Listener
/// Polls the default ring and returns the sender ID + Message
pub fn pdx_listen() -> (u32, MessageType) {
    loop {
        let sender: u64;
        let opcode: u64;
        let a0: u64;
        let a1: u64;
        let a2: u64;

        unsafe {
            asm!(
                "syscall",
                in("rax") SYSCALL_PDX_LISTEN,
                in("rdi") 0u64,             // Slot 0 by default
                lateout("rax") sender,     // Sender PD ID
                lateout("rsi") opcode,     // Message Discriminant
                lateout("rdx") a0,
                lateout("r10") a1,
                lateout("r8")  a2,
                lateout("rcx") _,          // Hazard 4 Mitigation
                lateout("r11") _,          // Hazard 4 Mitigation
                options(nostack)
            );
        }

        if sender != 0 {
            return (sender as u32, reconstruct_msg(opcode, a0, a1, a2));
        }
        sys_yield(); 
    }
}

/// Send an anonymous PDX reply to a target Protection Domain (Syscall 29)
pub fn pdx_reply(target_pd: u32, val: u64) {
    unsafe {
        asm!(
            "syscall",
            in("rax") SYSCALL_PDX_REPLY,
            in("rdi") target_pd,
            in("rsi") val,
            lateout("rax") _,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
    }
}

pub fn sched_yield() {
    unsafe {
        asm!(
            "syscall",
            in("rax") SYSCALL_YIELD,
            lateout("rax") _,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
    }
}

pub fn sys_yield() {
    sched_yield();
}
