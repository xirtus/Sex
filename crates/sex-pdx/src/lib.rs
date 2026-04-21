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

/// Structure for window move parameters passed across PDX.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SexWindowMoveParams {
    pub window_id: u64,
    pub new_x: u32,
    pub new_y: u32,
}

/// Structure for window resize parameters passed across PDX.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SexWindowResizeParams {
    pub window_id: u64,
    pub new_width: u32,
    pub new_height: u32,
}

/// Structure for setting a window's tags across PDX.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SexSetWindowTagsParams {
    pub window_id: u64,
    pub tag_mask: u64,
}

/// Structure for setting a window's corner radius across PDX.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SexSetWindowRoundnessParams {
    pub window_id: u64,
    pub radius: u32,
}

/// Structure for setting a window's blur strength across PDX.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SexSetWindowBlurParams {
    pub window_id: u64,
    pub strength: u32,
}

/// Structure for setting a window's animation state across PDX.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SexSetWindowAnimationParams {
    pub window_id: u64,
    pub is_animating: bool,
}

/// Structure for switching the active view.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SexSwitchViewParams {
    pub view_index: u32,
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
    AuthenticateWindow { window_id: u32, auth_token: [u8; 32] },
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
    HIDEvent { ev_type: u16, code: u16, value: i32 },
    VfsCall { command: u32, offset: u64, size: u64, buffer_cap: u32 },
    VfsReply { status: i64, size: u64 },
    MapMemory { pfn: u64, size: u64 },
    AllocateMemory { size: u64 },

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
    DisplayPrimaryFramebuffer { virt_addr: u64, width: u32, height: u32, pitch: u32 },
    DisplayModeset { width: u32, height: u32, refresh: u32 },
    DisplayCursor { x: i32, y: i32, visible: bool, buffer_id: u32 },
    DisplayBufferCommit { buffer_id: u32, damage_x: u32, damage_y: u32, damage_w: u32, damage_h: u32 },
    DisplayGeminiRepairDisplay,
    DisplayGetInfo,
    SetWindowDecorations { window_id: u64, border_color: u32, border_thickness: u32 },
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

/// Syscall IDs for SexCompositor operations (consistent with sexdisplay)
pub const PDX_SEX_WINDOW_CREATE: u64 = 0xDE;
pub const PDX_SET_WINDOW_DECORATIONS: u64 = 0xE2;
pub const PDX_GET_DISPLAY_INFO: u64 = 0xE3;
pub const PDX_FOCUS_WINDOW: u64 = 0xE4;
pub const PDX_MINIMIZE_WINDOW: u64 = 0xE5;
pub const PDX_MAXIMIZE_WINDOW: u64 = 0xE6;
pub const PDX_CLOSE_WINDOW: u64 = 0xE7;
pub const PDX_ALLOCATE_MEMORY: u64 = 0xEC; // Aligned with sexdisplay
pub const PDX_MAP_MEMORY: u64 = 0xED;     // Aligned with sexdisplay
pub const PDX_MOVE_WINDOW: u64 = 0xEE; // Aligned with sexdisplay
pub const PDX_RESIZE_WINDOW: u64 = 0xEF; // Aligned with sexdisplay

// New syscall for committing individual window frames (aligned with sexdisplay)
pub const PDX_WINDOW_COMMIT_FRAME: u64 = 0xF0;

// New syscalls for Tag management (aligned with sexdisplay)
pub const PDX_SET_WINDOW_TAGS: u64 = 0xE8;
pub const PDX_GET_WINDOW_TAGS: u64 = 0xE9;
pub const PDX_SET_VIEW_TAGS: u64 = 0xEA;
pub const PDX_GET_VIEW_TAGS: u64 = 0xEB;

// New syscalls for UI aesthetics (aligned with sexdisplay)
pub const PDX_SET_WINDOW_ROUNDNESS: u64 = 0xF1;
pub const PDX_SET_WINDOW_BLUR: u64 = 0xF2;
pub const PDX_SET_WINDOW_ANIMATION: u64 = 0xF3;

// View/workspace management
pub const PDX_SWITCH_VIEW: u64 = 0xF4;
pub const PDX_GET_ALL_VIEWS: u64 = 0xF5;

// System telemetry (0xFC-0xFE reserved; 0xF6-0xFB used by sexdisplay for UI syscalls)
pub const PDX_GET_TIME: u64 = 0xFC;       // → packed u64: (seconds << 20) | millis
pub const PDX_GET_CPU_USAGE: u64 = 0xFD;  // arg0=core_id (0=all) → percent * 100
pub const PDX_GET_MEM_USAGE: u64 = 0xFE;  // → packed u64: (used_kb << 32) | total_kb

// sexnet PDX server protocol (well-known PD = 6)
pub const SEXNET_PD: u32 = 6;
pub const SEXNET_GET_STATUS: u64 = 0x200;  // → packed u64: (link_speed_mbps << 16) | flags
pub const SEXNET_SCAN_WIFI: u64 = 0x201;   // arg0=out_buf_ptr, arg1=max_entries → entry count
pub const SEXNET_CONNECT: u64 = 0x202;     // arg0=ssid_ptr, arg1=ssid_len, arg2=pass_ptr → 0=ok
pub const SEXNET_DISCONNECT: u64 = 0x203;  // → 0=ok
pub const SEXNET_VPN_UP: u64 = 0x204;      // arg0=config_ptr, arg1=config_len → 0=ok
pub const SEXNET_VPN_DOWN: u64 = 0x205;    // → 0=ok
pub const SEXNET_GET_IP: u64 = 0x206;      // → packed IPv4 (big-endian u32 in low bits)

/// WiFi AP entry written by sexnet into caller's scan buffer.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SexnetApEntry {
    pub ssid: [u8; 33],    // null-terminated SSID
    pub rssi: i8,          // dBm
    pub channel: u8,
    pub flags: u8,         // bit0=open, bit1=wpa2, bit2=wpa3, bit3=connected
}

// sexaudio PDX server protocol (well-known PD = 7)
pub const SEXAUDIO_PD: u32 = 7;
pub const SEXAUDIO_SUBMIT_PCM: u64 = 0x300;  // arg0=frame_ptr → 0=ok
pub const SEXAUDIO_SET_VOLUME: u64 = 0x301;  // arg0=vol_percent (0–100) → 0=ok
pub const SEXAUDIO_GET_STATUS: u64 = 0x302;  // → packed: (sample_rate<<32)|flags

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SexAudioFrame {
    pub pfn: u64,           // physical frame holding interleaved PCM
    pub pku_key: u8,        // PKU key of the decoder sandbox
    pub channels: u8,       // 1=mono, 2=stereo
    pub sample_rate: u32,   // Hz (e.g. 44100, 48000)
    pub sample_count: u32,  // samples per channel in this frame
    pub format: u8,         // 0=f32le, 1=s16le
}

// Linen file manager VFS client protocol (sexfiles server)
pub const LINEN_READDIR:  u64 = 0x500; // arg0=path_ptr, arg1=out_buf_ptr → entry count
pub const LINEN_STAT:     u64 = 0x501; // arg0=path_ptr → packed(flags<<32|size)
pub const LINEN_COPY:     u64 = 0x502; // arg0=src_ptr, arg1=dst_ptr → 0=ok
pub const LINEN_MOVE:     u64 = 0x503; // arg0=src_ptr, arg1=dst_ptr → 0=ok
pub const LINEN_DELETE:   u64 = 0x504; // arg0=path_ptr → 0=ok
pub const LINEN_MKDIR:    u64 = 0x505; // arg0=path_ptr → 0=ok
pub const SEXFILES_PD:    u32 = 4;     // well-known PD for sexfiles server

/// Directory entry written by sexfiles into linen's output buffer.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinenDirEntry {
    pub name: [u8; 64],
    pub name_len: u32,
    pub flags: u32,   // bit 0 = is_dir, bit 1 = is_link, bit 2 = is_exec
    pub size: u64,
}

impl Default for LinenDirEntry {
    fn default() -> Self {
        LinenDirEntry { name: [0u8; 64], name_len: 0, flags: 0, size: 0 }
    }
}

// Silkbar applet protocol
pub const PDX_SILKBAR_REGISTER: u64 = 0x100;
pub const PDX_SILKBAR_UNREGISTER: u64 = 0x101;
pub const PDX_SILKBAR_NOTIFY: u64 = 0x102;
pub const PDX_SILKBAR_WINDOW_OPEN: u64 = 0x103;
pub const PDX_SILKBAR_WINDOW_CLOSE: u64 = 0x104;
pub const PDX_SILKBAR_WINDOW_FOCUS: u64 = 0x105;

/// Window creation parameters passed to PDX_SEX_WINDOW_CREATE.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SexWindowCreateParams {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub pfn_base: u64,
}

/// Applet registration payload for PDX_SILKBAR_REGISTER.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SilkbarRegisterParams {
    pub name: [u8; 32],
    pub applet_pd: u32,
}

/// Tray notification payload for PDX_SILKBAR_NOTIFY.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SilkbarNotifyParams {
    pub applet_pd: u32,
    pub text: [u8; 32],
    pub icon_state: u8,  // 0=idle, 1=active, 2=alert
}

pub fn pdx_map_memory(map_syscall_num: u64, pfn: u64, size: u64) -> Result<u64, ()> {
    let res = pdx_call(0, map_syscall_num, pfn, size); // PD 0 for kernel calls

    if res == u64::MAX { // Assuming u64::MAX indicates an error from kernel
        Err(())
    } else {
        Ok(res)
    }
}

pub fn pdx_allocate_memory(alloc_syscall_num: u64, size: u64) -> Result<u64, ()> {
    let res = pdx_call(0, alloc_syscall_num, size, 0); // PD 0 for kernel calls, 0 for unused arg

    if res == u64::MAX { // Assuming u64::MAX indicates an error from kernel
        Err(())
    } else {
        Ok(res)
    }
}

/// Requests sexdisplay to move a window.
pub fn pdx_move_window(window_id: u64, new_x: u32, new_y: u32) -> Result<(), ()> {
    let params = SexWindowMoveParams {
        window_id,
        new_x,
        new_y,
    };
    let res = unsafe {
        pdx_call(
            1, // Target PD ID for sexdisplay (assuming PID 1)
            PDX_MOVE_WINDOW,
            &params as *const _ as u64, // Pass pointer to params as arg0
            0,
        )
    };
    if res == 0 {
        Ok(())
    } else {
        Err(())
    }
}

/// Requests sexdisplay to resize a window.
pub fn pdx_resize_window(window_id: u64, new_width: u32, new_height: u32) -> Result<(), ()> {
    let params = SexWindowResizeParams {
        window_id,
        new_width,
        new_height,
    };
    let res = unsafe {
        pdx_call(
            1, // Target PD ID for sexdisplay (assuming PID 1)
            PDX_RESIZE_WINDOW,
            &params as *const _ as u64, // Pass pointer to params as arg0
            0,
        )
    };
    if res == 0 {
        Ok(())
    } else {
        Err(())
    }
}

/// Requests the kernel to spawn a new Protection Domain (process).
/// Returns the new PD's ID on success.
pub fn pdx_spawn_pd(path: &[u8]) -> Result<u32, ()> {
    let mut message = PdxMessage {
        msg_type: MessageType::SpawnPD { path_ptr: path.as_ptr() as u64 },
        payload: [0; 64], // Payload not used for SpawnPD message itself
    };

    let res = unsafe {
        pdx_call(
            0, // Target PD ID 0 for kernel calls
            0, // num field is 0 for direct message passing (MessageType based)
            &mut message as *mut _ as u64,
            0,
        )
    };

    if res == u64::MAX { // Assuming u64::MAX indicates an error
        Err(())
    } else {
        Ok(res as u32) // Return the new PD ID
    }
}

/// Requests sexdisplay to set a window's tags.
pub fn pdx_set_window_tags(window_id: u64, tag_mask: u64) -> Result<(), ()> {
    let params = SexSetWindowTagsParams {
        window_id,
        tag_mask,
    };
    let res = unsafe {
        pdx_call(
            1, // Target PD ID for sexdisplay (assuming PID 1)
            PDX_SET_WINDOW_TAGS,
            &params as *const _ as u64, // Pass pointer to params as arg0
            0,
        )
    };
    if res == 0 {
        Ok(())
    } else {
        Err(())
    }
}

/// Requests sexdisplay to get a window's tags.
pub fn pdx_get_window_tags(window_id: u64) -> Result<u64, ()> {
    let res = unsafe {
        pdx_call(
            1, // Target PD ID for sexdisplay (assuming PID 1)
            PDX_GET_WINDOW_TAGS,
            window_id, // arg0 is the window ID
            0,
        )
    };
    if res == 0 { // 0 indicates error or window not found
        Err(())
    } else {
        Ok(res) // Return the tag mask
    }
}

/// Requests sexdisplay to set the current view's tags.
pub fn pdx_set_view_tags(tag_mask: u64) -> Result<(), ()> {
    let res = unsafe {
        pdx_call(
            1, // Target PD ID for sexdisplay (assuming PID 1)
            PDX_SET_VIEW_TAGS,
            tag_mask, // arg0 is the tag mask
            0,
        )
    };
    if res == 0 {
        Ok(())
    } else {
        Err(())
    }
}

/// Requests sexdisplay to get the current view's tags.
pub fn pdx_get_view_tags() -> Result<u64, ()> {
    let res = unsafe {
        pdx_call(
            1, // Target PD ID for sexdisplay (assuming PID 1)
            PDX_GET_VIEW_TAGS,
            0, // No specific arguments
            0,
        )
    };
    if res == 0 { // 0 could indicate error or no tags set, but sexdisplay returns current mask
        // Need to clarify sexdisplay's error handling for get_view_tags
        Err(()) // For now, assume 0 is an error
    } else {
        Ok(res) // Return the tag mask
    }
}

/// Requests sexdisplay to set a window's corner radius.
pub fn pdx_set_window_roundness(window_id: u64, radius: u32) -> Result<(), ()> {
    let params = SexSetWindowRoundnessParams {
        window_id,
        radius,
    };
    let res = unsafe {
        pdx_call(
            1, // Target PD ID for sexdisplay (assuming PID 1)
            PDX_SET_WINDOW_ROUNDNESS,
            &params as *const _ as u64, // Pass pointer to params as arg0
            0,
        )
    };
    if res == 0 {
        Ok(())
    } else {
        Err(())
    }
}

/// Requests sexdisplay to set a window's blur strength.
pub fn pdx_set_window_blur(window_id: u64, strength: u32) -> Result<(), ()> {
    let params = SexSetWindowBlurParams {
        window_id,
        strength,
    };
    let res = unsafe {
        pdx_call(
            1, // Target PD ID for sexdisplay (assuming PID 1)
            PDX_SET_WINDOW_BLUR,
            &params as *const _ as u64, // Pass pointer to params as arg0
            0,
        )
    };
    if res == 0 {
        Ok(())
    } else {
        Err(())
    }
}

/// Requests sexdisplay to set a window's animation state.
pub fn pdx_set_window_animation(window_id: u64, is_animating: bool) -> Result<(), ()> {
    let params = SexSetWindowAnimationParams {
        window_id,
        is_animating,
    };
    let res = unsafe {
        pdx_call(
            1, // Target PD ID for sexdisplay (assuming PID 1)
            PDX_SET_WINDOW_ANIMATION,
            &params as *const _ as u64, // Pass pointer to params as arg0
            0,
        )
    };
    if res == 0 {
        Ok(())
    } else {
        Err(())
    }
}

/// Requests sexdisplay to switch to a specified view.
pub fn pdx_switch_view(view_index: u32) -> Result<(), ()> {
    let params = SexSwitchViewParams {
        view_index,
    };
    let res = unsafe {
        pdx_call(
            1, // Target PD ID for sexdisplay (assuming PID 1)
            PDX_SWITCH_VIEW,
            &params as *const _ as u64, // Pass pointer to params as arg0
            0,
        )
    };
    if res == 0 {
        Ok(())
    } else {
        Err(())
    }
}

/// Requests sexdisplay to get all available view tag masks.
/// Fills the provided mutable slice with view tag masks and returns the number of views copied.
pub fn pdx_get_all_views(views_buffer: &mut [u64]) -> Result<usize, ()> {
    if views_buffer.is_empty() {
        return Err(()); // Invalid buffer
    }

    let res = unsafe {
        pdx_call(
            1, // Target PD ID for sexdisplay (assuming PID 1)
            PDX_GET_ALL_VIEWS,
            views_buffer.as_mut_ptr() as u64, // Pointer to buffer
            views_buffer.len() as u64, // Length of buffer
        )
    };

    if res == u64::MAX { // Assuming u64::MAX indicates an error from sexdisplay
        Err(())
    } else {
        Ok(res as usize) // Return number of views copied
    }
}

/// Requests sexdisplay to commit a frame for a specific window.
/// Commit a window frame. Passes pfn_list pointer packed with length in arg1.
/// Encoding: arg1 = (len << 48) | (ptr & 0x0000_FFFF_FFFF_FFFF)
pub fn pdx_commit_window_frame(window_id: u64, pfn_list: &[u64]) -> Result<(), ()> {
    let ptr = pfn_list.as_ptr() as u64;
    let len = pfn_list.len() as u64;
    let packed = (len << 48) | (ptr & 0x0000_FFFF_FFFF_FFFF);
    let res = pdx_call(1, PDX_WINDOW_COMMIT_FRAME, window_id, packed);
    if res == 0 { Ok(()) } else { Err(()) }
}



/// Queries sexdisplay for the current framebuffer width and height.
/// Returns a Result containing a tuple (width, height) on success, or Err(()) on failure.
pub fn pdx_get_framebuffer_info() -> Result<(u32, u32), ()> {
    let mut message = PdxMessage {
        msg_type: MessageType::DisplayGetInfo,
        payload: [0; 64],
    };

    let res = unsafe {
        pdx_call(
            1, // Target PD ID for sexdisplay (assuming PID 1, or a known constant)
            0, // num field is 0 for direct message passing
            &mut message as *mut _ as u64,
            0,
        )
    };

    if res == u64::MAX { // Assuming u64::MAX indicates an error
        Err(())
    } else {
        let width = (res >> 32) as u32;
        let height = res as u32;
        Ok((width, height))
    }
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
            in("r10") arg1,
            lateout("rax") res,
            lateout("rcx") _, // clobbered
            lateout("r11") _, // clobbered
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
#[repr(C)]
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
