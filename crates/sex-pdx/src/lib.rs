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
                in("r9")  0u64,            // no result struct — kernel skips PdxListenResult write
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

/// Non-blocking receive. Kernel writes PdxListenResult via r9 pointer.
/// Sentinel logic stays in kernel — userspace checks has_message, never type_id.
#[repr(C)]
pub struct PdxListenResult {
    pub has_message: u64,
    pub type_id:     u64,
    pub caller_pd:   u64,
    pub arg0:        u64,
    pub arg1:        u64,
    pub arg2:        u64,
}

pub fn pdx_try_listen() -> Option<PdxMessage> {
    let mut result = PdxListenResult {
        has_message: 0, type_id: 0, caller_pd: 0,
        arg0: 0, arg1: 0, arg2: 0,
    };
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 28u64,
            in("r9")  &mut result as *mut PdxListenResult as u64,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
    if result.has_message == 1 {
        Some(PdxMessage {
            type_id:   result.type_id,
            arg0:      result.arg0,
            arg1:      result.arg1,
            arg2:      result.arg2,
            caller_pd: result.caller_pd as u32,
            _pad:      0,
        })
    } else {
        None
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

pub unsafe fn sys_set_state(state: u64) {
    core::arch::asm!(
        "syscall",
        in("rax") SYS_SET_STATE,
        in("rdx") state,
        out("rcx") _,
        out("r11") _,
        options(nostack),
    );
}

// Capability slots — single source of truth for all protection domains.
// These are capability graph edges, NOT service ports.
pub const SLOT_STORAGE: u64 = 1; // sexfiles VFS
pub const SLOT_SEXT:    u64 = 2; // sext demand pager (fault resolver)
pub const SLOT_INPUT:   u64 = 3; // HID input
pub const SLOT_AUDIO:   u64 = 4; // audio server
pub const SLOT_DISPLAY: u64 = 5; // SexDisplay compositor
pub const SLOT_SHELL:   u64 = 6; // silk-shell orchestration entry

// sexdisplay (SLOT_DISPLAY) opcodes
pub const OP_WINDOW_CREATE: u64 = 0xE4;
pub const OP_WINDOW_SUBMIT: u64 = 0xE5;
pub const OP_WINDOW_VBLANK: u64 = 0xE6;
pub const OP_WINDOW_MAP:    u64 = 0xE7;
pub const OP_WINDOW_WRITE:  u64 = 0xE8;

pub const OP_WINDOW_PAINT:   u64 = 0xDF;
pub const OP_WINDOW_DESTROY: u64 = 0xE0;
pub const OP_MOVE_WINDOW:    u64 = 0xEE;
pub const OP_RESIZE_WINDOW:  u64 = 0xEF;

// silk-shell (SLOT_SHELL) opcodes
pub const OP_SET_BG:      u64 = 0x100;
pub const OP_RENDER_BAR:  u64 = 0x101;

// Capability invocation trap numbers (ring-3 → ring-0 transition only).
// These are sex-pdx implementation details, NOT POSIX-style syscall numbers.
pub const SYSCALL_PDX_REPLY: u64 = 1;

// Capability error sentinels — must match kernel/src/ipc.rs exactly.
// Returned when sex-pdx rejects an invocation.
pub const ERR_SERVICE_NOT_READY: u64 = 0xFFFF_FFFF_FFFF_FFFE;
pub const ERR_CAP_INVALID:       u64 = 0xFFFF_FFFF_FFFF_FFFC;

/// Explicit return struct for pdx_call.
/// Kernel writes both fields via the r9 pointer before sysretq.
/// No sentinel-in-rax; status and value are fully separate.
#[repr(C)]
pub struct PdxResponse {
    pub status: u64,  // 0 = Ok, ERR_* = error
    pub value:  u64,  // meaningful when status == 0
}

pub fn pdx_call(slot: u64, opcode: u64, arg0: u64, arg1: u64, arg2: u64) -> PdxResponse {
    let mut resp = PdxResponse { status: 0, value: 0 };
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 0u64,
            in("rdi") slot,
            in("rsi") opcode,
            in("rdx") arg0,
            in("r10") arg1,
            in("r8")  arg2,
            in("r9")  &mut resp as *mut PdxResponse as u64,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
    resp
}

#[inline]
pub fn pdx_call_checked(slot: u64, opcode: u64, arg0: u64, arg1: u64, arg2: u64) -> Result<u64, u64> {
    let resp = pdx_call(slot, opcode, arg0, arg1, arg2);
    if resp.status == 0 {
        Ok(resp.value)
    } else {
        Err(resp.status)
    }
}

/// Spawn a new protection domain from an ELF image.
///
/// BOOT_DAG-gated: only domains with an explicit spawn capability granted by
/// BOOT_DAG at boot time may call this. Without the capability → ERR_CAP_INVALID.
/// Calling before BOOT_DAG lock is undefined behavior in the capability model.
pub fn pdx_spawn_pd(elf_addr: u64, elf_len: u64) -> Result<DomainId, u64> {
    let resp = pdx_call(0, 0x10, elf_addr, elf_len, 0);
    if resp.status == 0 {
        Ok(DomainId(resp.value as u32))
    } else {
        Err(resp.status)
    }
}
