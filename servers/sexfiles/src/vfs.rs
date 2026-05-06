extern crate alloc;
use crate::backends::ramfs::RamFs;
use crate::backends::FsBackend;
use crate::messages;
use core::sync::atomic::{AtomicU64, Ordering};

/// VFS operation counters (diagnostic only).
pub static IPC_OPS_TOTAL: AtomicU64 = AtomicU64::new(0);

/// The single RamFS instance backing all VFS operations.
pub static RAMFS: RamFs = RamFs::new();

/// Route a PDX message to the appropriate backend handler.
/// Called from the trampoline message loop.
/// `caller_pd` is the PD of the requesting process (from PDX message).
pub fn handle_vfs_message(type_id: u64, arg0: u64, arg1: u64, arg2: u64, caller_pd: u32) -> u64 {
    IPC_OPS_TOTAL.fetch_add(1, Ordering::Relaxed);

    // All operations currently route to RamFS.
    let backend: &dyn FsBackend = &RAMFS;

    match type_id {
        // ── OP_RAMFS_OPEN ──
        // arg0 = name[0..7], arg1 = name[8..15], arg2 = name[16..23] | (flags << 24)
        messages::OP_RAMFS_OPEN => {
            // Mask flag byte (bits 24-31) from name portion per protocol spec:
            //   arg2 = name[16..23] | (flags << 24)
            let name_bytes = unpack_name(arg0, arg1, arg2 & !(0xFFu64 << 24));
            let flags = (arg2 >> 24) as u32;
            match backend.open(&name_bytes, flags, 0, caller_pd) {
                Ok(handle) => handle,
                Err(e) => e as u64,
            }
        }

        // ── OP_RAMFS_READ ──
        // arg0 = handle, arg1 = offset, arg2 = max_len
        messages::OP_RAMFS_READ => {
            let handle = arg0;
            let offset = arg1;
            let max_len = (arg2 as usize).min(messages::RAMFS_MAX_FILE_SIZE);
            let mut buf = [0u8; 8]; // Return up to 8 bytes in the reply
            let to_read = max_len.min(buf.len());
            match backend.read(handle, offset, &mut buf[..to_read], caller_pd) {
                Ok(n) => {
                    // Pack read data into reply u64
                    let mut reply = 0u64;
                    for i in 0..n.min(8) as usize {
                        reply |= (buf[i] as u64) << (i * 8);
                    }
                    reply
                }
                Err(e) => e as u64,
            }
        }

        // ── OP_RAMFS_WRITE ──
        // arg0 = handle, arg1 = offset, arg2 = packed data (8 bytes)
        messages::OP_RAMFS_WRITE => {
            let handle = arg0;
            let offset = arg1;
            let data = arg2.to_le_bytes(); // 8 bytes of data
            match backend.write(handle, offset, &data, caller_pd) {
                Ok(n) => n,
                Err(e) => e as u64,
            }
        }

        // ── OP_RAMFS_CLOSE ──
        // arg0 = handle
        messages::OP_RAMFS_CLOSE => {
            let handle = arg0;
            match backend.close(handle, caller_pd) {
                Ok(_) => 0,
                Err(e) => e as u64,
            }
        }

        // ── OP_RAMFS_LIST ──
        // arg0 = index
        // Returns: packed { handle: u64, name_len: u32 } in upper/lower bits,
        // or 0 if no more entries. Only returns entries owned by caller_pd.
        messages::OP_RAMFS_LIST => {
            let index = arg0 as usize;
            match backend.list_at(index, caller_pd) {
                Some((handle, name_len)) => {
                    (handle << 32) | (name_len as u64)
                }
                None => 0,
            }
        }

        // ── OP_RAMFS_CREATE_OWNER ──
        // arg0 = name bytes 0..7
        // arg1 = name bytes 8..15
        // arg2 = name bytes 16..23 (lower 24 bits) | (owner_pd << 32)
        messages::OP_RAMFS_CREATE_OWNER => {
            let name_bytes =
                unpack_name(arg0, arg1, arg2 & !(0xFFFF_FFFFu64 << 32));
            let owner_pd = (arg2 >> 32) as u32;
            match backend.create_with_owner(&name_bytes, owner_pd, caller_pd) {
                Ok(handle) => handle,
                Err(e) => e as u64,
            }
        }

        // ── OP_RAMFS_OBJECT_ID ──
        // arg0 = handle
        // Returns: RamFS-assigned object_id (≥1) for the open handle.
        // Caller must own the file (or caller_pd=0 for server-internal).
        // Use to resolve OQ5: obtain SexFiles-assigned global ID from a handle.
        messages::OP_RAMFS_OBJECT_ID => {
            let handle = arg0;
            match RAMFS.object_id_for_handle(handle, caller_pd) {
                Ok(id) => id,
                Err(e) => e as u64,
            }
        }

        // ── OP_RAMFS_STAT ──
        // arg0 = handle
        messages::OP_RAMFS_STAT => {
            let handle = arg0;
            match backend.stat(handle, caller_pd) {
                Ok((size, name_len)) => {
                    (size << 32) | name_len as u64
                }
                Err(e) => e as u64,
            }
        }

        _ => messages::ERR_NOT_FOUND as u64,
    }
}

/// Unpack a name from three u64 args.
/// Name is stored little-endian in arg0..arg2, up to 24 bytes,
/// zero-padded. Returns the actual name slice (strips trailing zeros).
fn unpack_name(arg0: u64, arg1: u64, arg2: u64) -> alloc::vec::Vec<u8> {
    let mut name = alloc::vec::Vec::with_capacity(messages::RAMFS_MAX_NAME);
    let bytes0 = arg0.to_le_bytes();
    let bytes1 = arg1.to_le_bytes();
    let bytes2 = arg2.to_le_bytes();
    name.extend_from_slice(&bytes0);
    name.extend_from_slice(&bytes1);
    // First 24 bytes of arg2 are name; rest may be flags
    name.extend_from_slice(&bytes2[..8]);
    // Strip trailing zeros (but preserve embedded zeros as valid name bytes)
    while name.last() == Some(&0) {
        name.pop();
    }
    name
}
