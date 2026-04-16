use sex_pdx::{MessageType, PageHandover};

// Re-export the core protocol from our shared crate
pub use sex_pdx::MessageType as VfsMessageType;

/// Phase 19: Advanced Zero-Copy VFS Protocol
pub enum VfsProtocol {
    Open { path: [u8; 256], flags: u32, mode: u32 },
    Read { fd: u64, len: u64, offset: u64 },
    Write { fd: u64, len: u64, offset: u64 },
    Close { fd: u64 },
    Stat { path: [u8; 256] },
    Readdir { dir_fd: u64, cookie: u64 },
    HandoverRead { page: PageHandover, offset: u64, len: u32 },
    HandoverWrite { page: PageHandover, offset: u64, len: u32 },
    Stats,
    PreWarmKeys { fd: u64, advice: u32 },
}
