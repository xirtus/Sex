use crate::capability::NodeCapData;

/// IPCtax-compliant Message structures for Driver/VFS interaction.
/// 100% Zero-Copy via Lent Capabilities.

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FsArgs {
    pub command: u32,
    pub offset: u64,
    pub size: u64,
    pub buffer: u64, // Virtual address of lent-memory buffer
}

pub const FS_READ: u32 = 1;
pub const FS_WRITE: u32 = 2;
pub const FS_LOOKUP: u32 = 3;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InputEvent {
    pub ev_type: u16,
    pub code: u16,
    pub value: i32,
}

pub const EV_KEY: u16 = 1;
pub const EV_ABS: u16 = 3;

/// Ring buffer message for Storage Completion.
#[repr(C)]
pub struct StorageCompletion {
    pub id: u64,
    pub status: i32,
}

/// Message types for XIPC Communication.
#[derive(Debug, Clone, Copy)]
pub enum MessageType {
    Empty,
    Signal(u8),
    IpcCall { func_id: u32, arg0: u64 },
    IpcReply(u64),
    PageFault { fault_addr: u64, error_code: u32, pd_id: u64, lent_cap: u64 },
}
