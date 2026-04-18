#![no_std]

pub mod ring;
pub use ring::{AtomicRing, PdxReply};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PageHandover {
    pub pfn: u64,
    pub pku_key: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayProtocol {
    // Phase 16: Basic Display
    DisplayBufferAlloc { width: u32, height: u32, format: u32 },
    DisplayBufferCommit { page: PageHandover },
    Stats,

    // Phase 21: GPU Acceleration
    DmaBufferSubmit { page: PageHandover, offset: u32, len: u32 },
    FenceWait { fence_id: u64 },
    GetGpuCaps,

    // Orbital Port: Window Management
    CreateWindow { x: i32, y: i32, w: u32, h: u32, flags: u32, title: [u8; 64] },
    DestroyWindow { window_id: u32 },
    RequestBuffer { window_id: u32 },
    CommitDamage { window_id: u32, damage: Rect },
    PollEvents { window_id: u32 },
    SetTitle { window_id: u32, title: [u8; 64] },
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum OrbitalEvent {
    Mouse { x: i32, y: i32 },
    Button { left: bool, middle: bool, right: bool },
    Key { code: u32, pressed: bool },
    Resize { w: u32, h: u32 },
    Quit,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreProtocol {
    // Legacy support
    FetchPackage { name: [u8; 256] },
    CacheBinary { name: [u8; 256], image: PageHandover },
    Stats,

    // Phase 20: Sexshop Advanced Protocol
    TransactionBegin,
    TransactionCommit,
    TransactionAbort,

    KVGet { key: [u8; 64] },
    KVSet { key: [u8; 64], value_paddr: u64, value_len: u64 },
    KVDelete { key: [u8; 64] },

    ObjectPut { hash: [u8; 32], data_paddr: u64, data_len: u64 },
    ObjectGet { hash: [u8; 32] },
    ObjectExists { hash: [u8; 32] },
    ObjectMove { hash: [u8; 32], target_node: u32 },

    SyncFilesystem,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LdProtocol {
    ResolveObject { name: [u8; 256] },
    MapLibrary { hash: [u8; 32], base_addr: u64 },
    GetEntry { hash: [u8; 32] },
    Stats,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeProtocol {
    LoadDriver { image: PageHandover },
    // Phase 21: Cluster Fabric
    ClusterObjectFetch { node_id: u32, hash: [u8; 32] },
    ClusterObjectPush { node_id: u32, hash: [u8; 32], page: PageHandover },
    Heartbeat { node_id: u32, load_avg: u32, best_core: u32 },
    // Phase 22: Distributed Capabilities
    CapabilityResolve { name: [u8; 64] },
    NodeRegister { node_id: u32, addr: [u8; 16] }, // IPv6 addr
    ClusterObjectMigrate { node_id: u32, hash: [u8; 32], page: PageHandover },
    ClusterSignalForward { target_node: u32, target_pd: u32, signal: u8 },
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    RawCall(u64),
    Signal(u8),          
    SignalDeliveryAck,
    PageFault { fault_addr: u64, error_code: u32, pd_id: u64, lent_cap: u32 },
    SpawnPD { path_ptr: u64 },
    DmaCall { command: u32, offset: u64, size: u64, buffer_cap: u32, device_cap: u32 },
    DmaReply { status: i64, size: u64 },
    NetCall { command: u32, socket_cap: u32, offset: u64, size: u64, buffer_cap: u32, remote_node: u32 },
    NetReply { status: i64, size: u64, socket_cap: u32 },
    TranslatorCall { command: u32, path_ptr: u64, code_cap: u32 },
    TranslatorReply { status: i64, translated_entry: u64 },
    DriverLoadCall { command: u32, driver_name_ptr: u64 },
    DriverLoadReply { status: i64, driver_pd_id: u32 },
    HardwareInterrupt { vector: u8, data: u64 },
    VfsCall { command: u32, offset: u64, size: u64, buffer_cap: u32 },
    VfsReply { status: i64, size: u64 },

    Store(StoreProtocol),
    StoreCall { command: u32, package_name_ptr: u64, buffer_cap: u32 },
    StoreReply { status: i64, val: u64, size: u64 },
    Ld(LdProtocol),
    LdReply { status: i64, entry: u64 },
    Node(NodeProtocol),

    // Phase 18: Advanced Zero-Copy VFS Protocol
    VfsOpen { path: [u8; 512], flags: u32, mode: u32 },
    VfsRead { fd: u64, len: u64, offset: u64 },
    VfsWrite { fd: u64, len: u64, offset: u64 },
    VfsClose { fd: u64 },
    VfsStat { path: [u8; 512] },
    VfsReaddir { dir_fd: u64, cookie: u64 },
    VfsZeroCopyHandover { page_count: u16, pfn_list: [u64; 64] },

    // Display Server Protocol (Phase 16: PDX Display)
    Display(DisplayProtocol),
    DisplayModeset { width: u32, height: u32, refresh: u32 },
    DisplayCursor { x: i32, y: i32, visible: bool, buffer_id: u32 },
    DisplayBufferCommit { buffer_id: u32, damage_x: u32, damage_y: u32, damage_w: u32, damage_h: u32 },
    DisplayGeminiRepairDisplay,
    RevokeKey { key: u8 },

    // Phase 26: SMP Control
    SetAffinity { core_id: u32 },
    
    // Legacy / Phase 11 compatibility
    ProcCall { command: u32, path_ptr: u64, arg_ptr: u64 },
    ProcReply { status: i64, pd_id: u32 },
    PipeCall { command: u32, pipe_fds_ptr: u64, pipe_cap: u32, buffer_cap: u32, size: u64 },
    PipeReply { status: i64, pipe_cap: u32 },
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PdxMessage {
    pub msg_type: MessageType,
    pub payload: [u8; 64],
}

pub struct PdxClient {
    pub slot: u32,
}

impl PdxClient {
    pub fn new(slot: u32) -> Self {
        Self { slot }
    }

    pub fn ring(&self) -> Option<&'static AtomicRing<MessageType>> {
        // Mock: In real impl would use shared memory address from capability
        None
    }

    pub fn wait(&self) {
        // Futex-style park
    }

    pub fn set_affinity(&self, core_id: u32) -> u64 {
        safe_pdx_call(self.slot, MessageType::SetAffinity { core_id })
    }
}

pub struct Message(u64);

impl Message {
    pub fn new() -> Self { Message(0) }
    pub fn from_u64(val: u64) -> Self { Message(val) }
    pub fn as_u64(&self) -> u64 { self.0 }
    
    pub fn msg_type(&self) -> MessageType {
        MessageType::RawCall(self.0)
    }

    pub fn status(&self) -> i64 { 0 }
    pub fn caller_pd(&self) -> u32 { 0 }
    
    pub fn reply(_pd: u32, val: u64) -> Self { Message(val) }
    
    pub fn dma_call(_opcode: u32, _offset: u64, _size: u64, _buffer_cap: u32) -> Self { Message(0) }
    pub fn dma_read(_offset: u64, _size: u64, _buffer_cap: u32, _device_cap: u32) -> Self { Message(0) }
    pub fn dma_write(_offset: u64, _size: u64, _buffer_cap: u32, _device_cap: u32) -> Self { Message(0) }
    pub fn dma_reply(_status: i64, _size: u64) -> Self { Message(0) }
    
    pub fn vfs_read(_fd: u32, _buf: *mut u8, _len: usize) -> Self { Message(0) }
    pub fn vfs_write(_fd: u32, _buf: *const u8, _len: usize) -> Self { Message(0) }
    
    pub fn proc_reply(_status: i64, _pd_id: u32) -> Self { Message(0) }
    pub fn pipe_reply(_status: i64, _size: u64, _cap: u32) -> Self { Message(0) }

    #[cfg(feature = "serde")]
    pub fn serialize<T: Serialize>(_obj: &T) -> Self { Message(0) }
    
    #[cfg(feature = "serde")]
    pub fn deserialize<'a, T: Deserialize<'a>>(&'a self) -> Option<T> { None }
}

pub fn safe_pdx_call(pd: u32, msg: MessageType) -> u64 {
    let mut pdx_msg = PdxMessage {
        msg_type: msg,
        payload: [0; 64],
    };
    pdx_call(pd, 0, &mut pdx_msg as *mut _ as u64, 0)
}

pub fn pdx_call(pd: u32, num: u64, arg0: u64, arg1: u64) -> u64 {
    let res: u64;
    #[cfg(target_arch = "x86_64")]
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
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (pd, num, arg0, arg1);
        res = 0;
    }
    res
}

pub fn pdx_listen(flags: u32) -> PdxRequest {
    let mut req = PdxRequest::default();
    #[cfg(target_arch = "x86_64")]
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
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = flags;
    }
    req
}

pub fn pdx_reply(pd: u32, val: u64) -> u64 {
    let res: u64;
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("syscall",
            in("rax") 29, // pdx_reply syscall
            in("rdi") pd,
            in("rsi") val,
            lateout("rax") res,
        );
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (pd, val);
        res = 0;
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
    pub fn bind_irq<F: Fn() + Send + Sync + 'static>(_irq: u8, _handler: F) {
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
