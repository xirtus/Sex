pub use sex_pdx::{pdx_call, pdx_listen, pdx_reply, MessageType, PdxRequest};

/// Simplified wrapper for VFS PDX calls
pub fn vfs_pdx_reply(caller: u32, msg: &MessageType) {
    pdx_reply(caller, msg as *const _ as u64);
}
