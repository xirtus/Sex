#![no_std]

pub mod ring;
pub use ring::{AtomicRing, PdxReply};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MessageType {
    RawCall(u64),
    Signal(u32),
    PageFault { fault_addr: u64, error_code: u32, pd_id: u64 },
    SpawnPD { path_ptr: u64 },
    DmaCall { command: u32, offset: u64, size: u64, buffer_cap: u32, device_cap: u32 },
    DmaReply { status: i64, size: u64 },
    NetCall,
    NetReply { status: i64, size: u64, socket_cap: u32 },
    TranslatorCall,
    TranslatorReply { status: i64, translated_entry: u64 },
    DriverLoadCall,
    DriverLoadReply { status: i64, driver_pd_id: u32 },
    HardwareInterrupt { vector: u8, data: u64 },
    VfsCall,
    VfsReply { status: i64, size: u64 },
    StoreCall,
    StoreReply { status: i64, size: u64 },
    InputCall,
    InputReply { status: i64, size: u64 },

    // Phase 18: Advanced Zero-Copy VFS Protocol
    VfsOpen { path: [u8; 512], flags: u32, mode: u32 },
    VfsRead { fd: u64, len: u64, offset: u64 },
    VfsWrite { fd: u64, len: u64, offset: u64 },
    VfsClose { fd: u64 },
    VfsStat { path: [u8; 512] },
    VfsReaddir { dir_fd: u64, cookie: u64 },
    VfsZeroCopyHandover { page_count: u16, pfn_list: [u64; 64] },

    // Display Server Protocol (Phase 16: PDX Display)
    DisplayBufferAlloc { width: u32, height: u32, format: u32 },
    DisplayBufferCommit { buffer_id: u32, damage_x: u32, damage_y: u32, damage_w: u32, damage_h: u32 },
    DisplayBufferReply { page_count: u32, pfn_list: [u64; 64], pku_key: u8 },
    DisplayModeset { width: u32, height: u32, refresh: u32 },
    DisplayCursor { x: i32, y: i32, visible: bool, buffer_id: u32 },
    DisplayGeminiRepairDisplay,
}

#[derive(Debug, Clone, Copy)]
pub struct Message(u64);

impl Message {
    pub fn new() -> Self { Message(0) }
    pub fn from_u64(val: u64) -> Self { Message(val) }
    pub fn as_u64(&self) -> u64 { self.0 }
    
    pub fn msg_type(&self) -> MessageType {
        // Mock implementation for serialization/deserialization logic
        MessageType::RawCall(self.0)
    }

    pub fn status(&self) -> i64 { 0 }
    pub fn caller_pd(&self) -> u32 { 0 }
    
    pub fn reply(pd: u32, val: u64) -> Self { Message(val) }
    
    pub fn dma_call(opcode: u32, offset: u64, size: u64, buffer_cap: u32) -> Self { Message(0) }
    pub fn dma_read(offset: u64, size: u64, buffer_cap: u32, device_cap: u32) -> Self { Message(0) }
    pub fn dma_write(offset: u64, size: u64, buffer_cap: u32, device_cap: u32) -> Self { Message(0) }
    pub fn dma_reply(status: i64, size: u64) -> Self { Message(0) }
    
    pub fn vfs_read(fd: u32, buf: *mut u8, len: usize) -> Self { Message(0) }
    pub fn vfs_write(fd: u32, buf: *const u8, len: usize) -> Self { Message(0) }
    
    pub fn proc_reply(status: i64, pd_id: u32) -> Self { Message(0) }
    pub fn pipe_reply(status: i64, size: u64, cap: u32) -> Self { Message(0) }

    #[cfg(feature = "serde")]
    pub fn serialize<T: Serialize>(obj: &T) -> Self { Message(0) }
    
    #[cfg(feature = "serde")]
    pub fn deserialize<'a, T: Deserialize<'a>>(&'a self) -> Option<T> { None }
}

pub fn pdx_call(pd: u32, num: u64, arg0: u64, arg1: u64) -> u64 {
    let res: u64;
    unsafe {
        core::arch::asm!("syscall",
            in("rax") 27, // pdx_call syscall
            in("rdi") pd,
            in("rsi") num,
            in("rdx") arg0,
            in("rcx") arg1,
            lateout("rax") res,
        );
    }
    res
}

pub fn pdx_listen(flags: u32) -> PdxRequest {
    let mut req = PdxRequest::default();
    unsafe {
        core::arch::asm!("syscall",
            in("rax") 28, // pdx_listen syscall
            in("rdi") flags,
            lateout("rax") _,
            lateout("rdi") req.caller_pd,
            lateout("rsi") req.num,
            lateout("rdx") req.arg0,
            lateout("rcx") req.arg1,
            lateout("r8") req.arg2,
        );
    }
    req
}

pub fn pdx_reply(pd: u32, val: u64) -> u64 {
    let res: u64;
    unsafe {
        core::arch::asm!("syscall",
            in("rax") 29, // pdx_reply syscall
            in("rdi") pd,
            in("rsi") val,
            lateout("rax") res,
        );
    }
    res
}

#[repr(C)]
#[derive(Default)]
pub struct PdxRequest {
    pub caller_pd: u32,
    pub num: u64,
    pub arg0: u64,
    pub arg1: u64,
    pub arg2: u64,
}

pub mod irq {
    pub fn bind_irq<F: Fn() + Send + Sync + 'static>(irq: u8, handler: F) {
        // IRQ binding logic via PDX
    }
}

pub mod dma {
    pub struct DmaBuffer {
        pub addr: u64,
        pub size: usize,
    }
    impl DmaBuffer {
        pub fn map(phys: u64, size: usize) -> Self {
            Self { addr: phys, size }
        }
    }
}

pub mod mmio {
    pub struct Mmio(u64);
    impl Mmio {
        pub unsafe fn read_u32(&self, offset: u64) -> u32 {
            ((self.0 + offset) as *const u32).read_volatile()
        }
        pub unsafe fn write_u32(&self, offset: u64, val: u32) {
            ((self.0 + offset) as *mut u32).write_volatile(val)
        }
    }
}
