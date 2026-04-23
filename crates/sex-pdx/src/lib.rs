#![no_std]

pub use core::alloc::{GlobalAlloc, Layout};

pub type Pdx = u64;

// --- Standard Macro Shims ---
#[macro_export]
macro_rules! serial_println {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        struct SerialWriter;
        impl Write for SerialWriter {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                unsafe { $crate::serial_print(s); }
                Ok(())
            }
        }
        let _ = writeln!(SerialWriter, $($arg)*);
    }};
}

pub unsafe fn serial_print(s: &str) {
    core::arch::asm!(
        "syscall",
        in("rax") 69u64,
        in("rdi") s.as_ptr() as u64,
        in("rsi") s.len() as u64,
        lateout("rax") _,
        lateout("rcx") _,
        lateout("r11") _,
    );
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {};
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {};
}

// Standardized Capability Slots
pub const SLOT_STORAGE: u32 = 1;
pub const SLOT_NETWORK: u32 = 2;
pub const SLOT_INPUT: u32   = 3;
pub const SLOT_AUDIO: u32   = 4;
pub const SLOT_DISPLAY: u32 = 5;
pub const SLOT_SHELL: u32   = 6;

// Compositor Opcodes (Slot 5)
pub const OP_WINDOW_CREATE: u64 = 0xDE;
pub const OP_WINDOW_COMMIT: u64 = 0xDD;
pub const OP_WINDOW_PAINT: u64  = 0xDF;

// Shell Opcodes (Slot 6)
pub const OP_SET_BG: u64        = 0x100;
pub const OP_RENDER_BAR: u64    = 0x101;
pub const OP_LAUNCH_APP: u64    = 0x102;
pub const PDX_FOCUS_WINDOW: u64          = 0xE1;
pub const PDX_MINIMIZE_WINDOW: u64       = 0xE2;
pub const PDX_MAXIMIZE_WINDOW: u64       = 0xE3;
pub const PDX_CLOSE_WINDOW: u64          = 0xE4;
pub const PDX_MOVE_WINDOW: u64           = 0xE5;
pub const PDX_RESIZE_WINDOW: u64         = 0xE6;
pub const PDX_SET_WINDOW_TAGS: u64       = 0xE7;
pub const PDX_GET_WINDOW_TAGS: u64       = 0xE8;
pub const PDX_SET_VIEW_TAGS: u64         = 0xE9;
pub const PDX_GET_VIEW_TAGS: u64         = 0xEA;
pub const PDX_SET_WINDOW_ROUNDNESS: u64  = 0xEC;
pub const PDX_SET_WINDOW_BLUR: u64       = 0xED;
pub const PDX_SET_WINDOW_ANIMATION: u64  = 0xEE;

// --- Memory ---
pub const PDX_ALLOCATE_MEMORY: u64       = 0x01;
pub const PDX_MAP_MEMORY: u64            = 0x02;
pub const PDX_GET_DISPLAY_INFO: u64      = 0x03;
pub const PDX_GET_FRAMEBUFFER_INFO: u64  = 0x04;

// --- Messaging / time ---
pub const PDX_SEND_MESSAGE: u64          = 0x0E;
pub const PDX_GET_TIME: u64              = 0x10;

// --- Silkbar applet ops ---
pub const PDX_SILKBAR_REGISTER: u64      = 0x20;
pub const PDX_SILKBAR_NOTIFY: u64        = 0x21;

// --- File manager ops ---
pub const LINEN_READDIR: u64             = 0x30;
pub const LINEN_COPY: u64                = 0x31;
pub const LINEN_MOVE: u64                = 0x32;
pub const LINEN_DELETE: u64              = 0x33;
pub const LINEN_MKDIR: u64               = 0x34;

// ── Dynamic PD discovery (MPK-safe, kernel fallback) ─────────────────────────────
pub const SEXSHOP_NAME: &str = "sexshop";
pub const SEXFILES_NAME: &str = "sexfiles";
pub const PDX_DISCOVER_SERVICE: u32 = 0x100;   // kernel/sexshop lookup ID

/// Resolve service name → PD ID (temporary kernel fallback until sexshop PDX lookup is wired).
/// Zero-copy, single PKEY flip — preserves Intel MPK isolation model.
pub unsafe fn discover_pd(name: &str) -> Option<u32> {
    // Phase 24 stub: hardcoded well-known IDs (will be replaced by real sexshop query)
    match name {
        "sexshop"  => Some(3),
        "sexfiles" => Some(4),
        _          => None,
    }
}

// Re-export existing PDX primitives
// (Removed redundant re-exports that conflict with local definitions)
pub const SEXFILES_PD_FALLBACK: u32 = 4;


// x/y: i32 to allow centred placement math, pfn_base: physical frame for compositor
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SexWindowCreateParams {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub title: [u8; 32],
    pub pfn_base: u64,
}

// x/y: i32 to allow negative offsets and arithmetic without casts
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

// --- IPC types ---
#[derive(Debug, Clone, Copy)]
pub enum MessageType {
    HIDEvent          { ev_type: u16, code: u16, value: i32 },
    CompileRequest    { source_path: &'static [u8], target_triple: &'static [u8] },
    KeyEvent          { code: u16, value: i32, modifiers: u16 },
    Notification      { progress: u8, message: &'static str },
    DmaCall           { command: u32, offset: u64, size: u32, buffer_cap: u64 },
    DmaReply          { status: u32, size: u32 },
    HardwareInterrupt { vector: u8 },
    PageFault         { addr: u64, error_code: u64 },
}

#[repr(C)]
pub struct PdxMessage {
    pub msg_type: MessageType,
    pub payload: [u8; 64],
}

// --- Capability / memory transfer ---
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PageHandover {
    pub pfn: u64,
    pub pku_key: u32,
}

// --- Silkbar applet params ---
#[repr(C)]
pub struct SilkbarRegisterParams {
    pub name: [u8; 32],
    pub applet_pd: u32,
}

#[repr(C)]
pub struct SilkbarNotifyParams {
    pub applet_pd: u32,
    pub text: [u8; 32],
    pub icon_state: u32,
}

// --- File manager dir entry ---
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinenDirEntry {
    pub name: [u8; 64],
    pub name_len: u16,
    pub flags: u16,
    pub size: u64,
}

impl Default for LinenDirEntry {
    fn default() -> Self {
        Self { name: [0u8; 64], name_len: 0, flags: 0, size: 0 }
    }
}

// --- Event received via pdx_listen ---
#[repr(C)]
pub struct PdxEvent {
    pub num: u64,        // opcode from caller (0 = no message)
    pub arg0: u64,       // first argument
    pub arg1: u64,       // second argument
    pub arg2: u64,       // third argument
    pub caller_pd: u32,  // which PD sent this
}

/// Framebuffer info returned by PDX_GET_DISPLAY_INFO (0x03).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DisplayInfo {
    pub virt_addr: u64,
    pub width:     u32,
    pub height:    u32,
    pub pitch:     u32,   // pixels per row (pitch / 4)
}

// --- Core call gate: slot=target PD, syscall=opcode, arg0/arg1/arg2=data ---
// rax=27, rdi=slot, rsi=opcode, rdx=arg0, r10=arg1, r8=arg2
pub unsafe fn pdx_call(slot: u32, opcode: u64, arg0: u64, arg1: u64, arg2: u64) -> u64 {
    let result: u64;
    core::arch::asm!(
        "syscall",
        inout("rax") 27u64 => result,
        in("rdi") slot as u64,
        in("rsi") opcode,
        in("rdx") arg0,
        in("r10") arg1,
        in("r8")  arg2,
        lateout("rcx") _,
        lateout("r11") _,
    );
    result
}

pub fn pdx_spawn_pd(_path: &[u8]) -> u32 { 42 }

/// Non-blocking poll on current PD's IPC ring (syscall 28).
/// Returns PdxEvent with num=0 when ring is empty.
/// num != 0 → valid event; num = opcode sent by caller.
pub unsafe fn pdx_listen(_slot: u32) -> PdxEvent {
    let caller: u64;
    let num: u64;
    let arg0_ret: u64;
    let arg1_ret: u64;
    let arg2_ret: u64;
    core::arch::asm!(
        "syscall",
        inout("rax") 28u64 => _,
        lateout("rdi") caller,
        lateout("rsi") num,
        lateout("rdx") arg0_ret,
        lateout("r10") arg1_ret,
        lateout("r8")  arg2_ret,
        lateout("rcx") _,
        lateout("r11") _,
    );
    PdxEvent { num: num, arg0: arg0_ret, arg1: arg1_ret, arg2: arg2_ret, caller_pd: caller as u32 }
}

/// Reply to a caller via syscall 29.
pub unsafe fn pdx_reply(target_pd: u32, result: u64) {
    core::arch::asm!(
        "syscall",
        inout("rax") 29u64 => _,
        in("rdi") target_pd as u64,
        in("rsi") result,
        lateout("rcx") _,
        lateout("r11") _,
    );
}

/// Cooperative yield: briefly re-enables interrupts so APIC timer can preempt (syscall 32).
pub unsafe fn sys_yield() {
    core::arch::asm!(
        "syscall",
        inout("rax") 32u64 => _,
        lateout("rcx") _,
        lateout("r11") _,
    );
}

/// Cooperative yield: invokes Syscall 24 (sys_yield).
pub fn sched_yield() {
    unsafe {
        core::arch::asm!(
            "syscall",
            inout("rax") 24u64 => _,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
}
pub fn safe_pdx_register(_name: &[u8]) -> u32 { 0 }

// --- Memory stubs ---
pub fn pdx_allocate_memory(_op: u64, _size: u64) -> Result<u64, ()> { Ok(0x1000_0000) }
pub fn pdx_map_memory(_op: u64, _pfn: u64, _size: u64) -> Result<u64, ()> { Ok(0x2000_0000) }
pub fn pdx_get_framebuffer_info() -> Result<u64, ()> { Ok(0) }

// --- Window management stubs ---
pub fn pdx_move_window(_id: u64, _x: u32, _y: u32) -> Result<(), ()> { Ok(()) }
pub fn pdx_resize_window(_id: u64, _w: u32, _h: u32) -> Result<(), ()> { Ok(()) }
pub fn pdx_set_window_tags(_id: u64, _mask: u64) -> Result<(), ()> { Ok(()) }
pub fn pdx_get_window_tags(_id: u64) -> Result<u64, ()> { Ok(0) }
pub fn pdx_set_view_tags(_mask: u64) -> Result<(), ()> { Ok(()) }
pub fn pdx_get_view_tags() -> Result<u64, ()> { Ok(0) }
pub fn pdx_commit_window_frame(_id: u64, _pfns: &[u64]) -> Result<(), ()> { Ok(()) }
pub fn pdx_set_window_roundness(_id: u64, _radius: u32) -> Result<(), ()> { Ok(()) }
pub fn pdx_set_window_blur(_id: u64, _strength: u32) -> Result<(), ()> { Ok(()) }
pub fn pdx_set_window_animation(_id: u64, _anim: bool) -> Result<(), ()> { Ok(()) }

// --- Allocator for binaries to use ---
pub struct DummyAllocator;

unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 { core::ptr::null_mut() }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

/// Invokes Syscall 35 to pop a keystroke from the kernel's lock-free ring.
/// Returns `Some(scancode)` if a key is waiting, or `None` if the buffer is empty.
pub fn get_keystroke() -> Option<u8> {
    let result: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            inout("rax") 35u64 => result,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    
    if result == u64::MAX {
        None
    } else {
        Some(result as u8)
    }
}
