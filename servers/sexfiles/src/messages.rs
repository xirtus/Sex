use sex_pdx::MessageType;

// Re-export the core protocol from our shared crate
pub use sex_pdx::MessageType as VfsMessageType;

/// Phase 18: Advanced Zero-Copy VFS Protocol
/// This allows direct PKU-based memory handoff between the VFS server and applications.
pub enum VfsProtocol {
    Open = 1,
    Read = 2,
    Write = 3,
    Close = 4,
    Stat = 5,
    Readdir = 6,
    Handover = 7,
}
