//! sex-pdx — Runtime Capability Authority Layer (ARCHITECTURE.md §1.1)
//!
//! Implements the authority layer of the SexOS memory model (ARCHITECTURE.md §0).
//! Single runtime enforcement point. No inter-domain interaction bypasses this crate.
//! Capability slots are the only sanctioned access paths.

#![no_std]

/// Opaque protection domain identifier. All inter-domain references use this type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DomainId(pub u32);

/// SexOS authority layer tags. Canonical definition: ARCHITECTURE.md §0.
///
/// These layers are ORTHOGONAL authority domains, NOT a linear ordering.
/// No comparison or ordering between variants is implied or valid.
/// Use as categorical tags for enforcement routing decisions only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AuthLayer {
    /// Capability genesis domain. Static, boot-time only. Immutable after lock.
    BootDag = 0,
    /// Runtime authority domain. All runtime access decisions. Never bypassed.
    SexPdx  = 1,
    /// Enforcement acceleration domain. Hardware accel only. Non-authoritative.
    Pku     = 2,
}

/// Userland serial writer — calls kernel raw_print (syscall 0, opcode 69).
pub struct SerialWriter;

impl core::fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let _ = pdx_call(0, 69, s.as_ptr() as u64, s.len() as u64, 0);
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
pub struct Window {
    pub id: u32,
    pub pid: u32,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub layer: u8,
    pub buffer_cap: u64, // Slot
}

#[repr(C, u64)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WindowOp {
    Move(u32, i32, i32),
    Resize(u32, u32, u32),
    Focus(u32),
    Destroy(u32),
    Create(u32, u32, i32, i32, u32, u32, u64),
}

// Legacy window operation constants (used by sexdisplay)
pub const OP_WINDOW_CREATE: u64 = 0xE4;
pub const OP_WINDOW_SUBMIT: u64 = 0xE5;
pub const OP_WINDOW_VBLANK: u64 = 0xE6;
pub const OP_WINDOW_MAP:    u64 = 0xE7;
pub const OP_WINDOW_WRITE:  u64 = 0xE8;

// SilkBar protocol opcodes (reserved; server attaches in v7+)
pub const OP_SILKBAR_PING:     u64 = 0xF0;
pub const OP_SILKBAR_GET_ABI:  u64 = 0xF1;
pub const OP_SILKBAR_UPDATE:           u64 = 0xF2;
pub const OP_SILKBAR_WORKSPACE_ACTIVE: u64 = 0xF3;
pub const SILKBAR_ABI_VERSION: u64 = 1;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ShellEvent {
    pub op: WindowOp,
}

/// Normalized input event. Single source of truth for all input in silk-shell FSM.
///
/// IPC encoding (type_id=0x202 from sexinput):
///   arg2=1 (EV_KEY):  arg0=scancode, arg1=1(dn)/0(up)
///   arg2=3 (EV_ABS):  arg0=abs_x,   arg1=abs_y  (mouse position, absolute)
///   arg2=4 (EV_BTN):  arg0=button,  arg1=1(dn)/0(up)
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InputEvent {
    KeyDown(u8),
    KeyUp(u8),
    MouseMove(i32, i32),
    MouseDown(u8),
    MouseUp(u8),
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Layer {
    pub win_id: u32,
    pub rect: [i32; 4], // [x, y, w, h]
    pub buf_ptr: u64,
    pub stride: usize,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FrameContext {
    pub tick: u64,
    pub snapshot_version: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SceneSnapshot {
    pub layers_ptr: u64,
    pub layers_len: u32,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub is_incremental: u32,
    pub damage_rects_ptr: u64,
    pub damage_rects_len: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct WindowDescriptor {
    pub window_id: u64,
    pub buffer_handle: u64, // Opaque capability handle
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub z_index: u32,
    pub focus_state: u32,
}

#[repr(C, u64)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MessageType {
    Ping = 0,
    Yield = 1,
    HIDEvent {
        code: u64,
        value: u64,
    } = 0x202,
}

pub struct PdxEvent; // Stub

/// Spin-receive from a specific capability slot.
pub fn pdx_listen_raw(slot: u64) -> PdxMessage {
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
                in("rdi") slot,
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
            sys_yield();
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

/// Non-blocking listen — calls syscall 28 once, returns None if empty.
pub fn pdx_try_listen_raw(slot: u64) -> Option<PdxMessage> {
    let type_id: u64;
    let caller_pd: u64;
    let arg0: u64;
    let arg1: u64;
    let arg2: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 28u64,
            in("rdi") slot,
            lateout("rax") type_id,
            lateout("rsi") caller_pd,
            lateout("rdx") arg0,
            lateout("r10") arg1,
            lateout("r8")  arg2,
            out("rcx") _,
            out("r11") _,
        );
    }
    if type_id == 0 {
        None
    } else {
        Some(PdxMessage {
            type_id,
            arg0,
            arg1,
            arg2,
            caller_pd: caller_pd as u32,
            _pad: 0,
        })
    }
}

/// Spin-receive from default message ring (Slot 0).
pub fn pdx_listen() -> PdxMessage {
    pdx_listen_raw(0)
}

pub fn pdx_try_listen() -> Option<PdxMessage> {
    pdx_try_listen_raw(0)
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
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 32u64,
            out("rcx") _,
            out("r11") _,
        );
    }
}

pub fn sched_yield() {
    sys_yield();
}

pub const SYSCALL_PDX_CALL: u64   = 0;
pub const SYSCALL_PDX_LISTEN: u64 = 2;
pub const SYSCALL_YIELD: u64      = 32;
pub const SYS_SET_STATE: u64      = 42;

pub const SVC_STATE_LISTENING: u64 = 1;

pub unsafe fn sys_set_state(state: u64) -> u64 {
    let ret: u64;
    core::arch::asm!(
        "syscall",
        inlateout("rax") SYS_SET_STATE => ret,
        in("rdi") state,
        out("rcx") _,
        out("r11") _,
        options(nostack),
    );
    ret
}

// Capability slots — single source of truth for all protection domains.
// These are capability graph edges, NOT service ports.
pub const SLOT_STORAGE: u64 = 1; // sexfiles VFS
pub const SLOT_SEXT:    u64 = 2; // sext demand pager (fault resolver)
pub const SLOT_INPUT:   u64 = 3; // HID input
pub const SLOT_AUDIO:   u64 = 4; // audio server
pub const SLOT_DISPLAY: u64 = 5; // SexDisplay compositor
pub const SLOT_SHELL:   u64 = 6; // silk-shell orchestration entry
pub const SLOT_SILKBAR: u64 = 7; // SilkBar model authority

// Capability invocation trap numbers (ring-3 → ring-0 transition only).
// These are sex-pdx implementation details, NOT POSIX-style syscall numbers.
pub const SYSCALL_PDX_REPLY: u64 = 1;

// Capability error sentinels — must match kernel/src/ipc.rs exactly.
// Returned when sex-pdx rejects an invocation.
pub const ERR_SERVICE_NOT_READY: u64 = 0xFFFF_FFFF_FFFF_FFFE;
pub const ERR_CAP_INVALID:       u64 = 0xFFFF_FFFF_FFFF_FFFC;

pub fn pdx_call(slot: u64, opcode: u64, arg0: u64, arg1: u64, arg2: u64) -> (u64, u64) {
    let status: u64;
    let value: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 0u64,
            in("rdi") slot,
            in("rsi") opcode,
            in("rdx") arg0,
            in("r10") arg1,
            in("r8")  arg2,
            lateout("rax") status,
            lateout("rsi") value,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
    (status, value)
}

#[inline]
pub fn pdx_call_checked(slot: u64, opcode: u64, arg0: u64, arg1: u64, arg2: u64) -> Result<u64, u64> {
    let (status, value) = pdx_call(slot, opcode, arg0, arg1, arg2);
    if status == 0 {
        Ok(value)
    } else {
        Err(status)
    }
}

/// Spawn a new protection domain from an ELF image.
///
/// BOOT_DAG-gated: only domains with an explicit spawn capability granted by
/// BOOT_DAG at boot time may call this. Without the capability → ERR_CAP_INVALID.
/// Calling before BOOT_DAG lock is undefined behavior in the capability model.
pub fn pdx_spawn_pd(elf_addr: u64, elf_len: u64) -> Result<DomainId, u64> {
    let (status, value) = pdx_call(0, 0x10, elf_addr, elf_len, 0);
    if status == 0 {
        Ok(DomainId(value as u32))
    } else {
        Err(status)
    }
}
