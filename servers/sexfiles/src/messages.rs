use sex_pdx::MessageType;

// Re-export the core protocol from our shared crate
pub use sex_pdx::MessageType as VfsMessageType;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PageHandover {
    pub pfn: u64,
    pub pku_key: u8,
}

/// Phase 19: Advanced Zero-Copy VFS Protocol
pub enum VfsProtocol {
    Open = 1,
    Read = 2,
    Write = 3,
    Close = 4,
    Stat = 5,
    Readdir = 6,
    HandoverRead { page: PageHandover, offset: u64, len: u32 },
    HandoverWrite { page: PageHandover, offset: u64, len: u32 },
    Stats,
}
