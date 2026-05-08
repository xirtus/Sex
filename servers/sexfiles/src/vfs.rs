extern crate alloc;
use crate::backends::ramfs::RamFs;
use crate::backends::FsBackend;
use crate::backends::diskfs::{DiskFs, DISKFS_MANIFEST_OBJECT_PATH, DISKFS_MANIFEST_FLAG_READ, DISKFS_MANIFEST_FLAG_WRITE};
use crate::messages;
use core::sync::atomic::{AtomicU64, Ordering};

/// VFS operation counters (diagnostic only).
pub static IPC_OPS_TOTAL: AtomicU64 = AtomicU64::new(0);

/// The single RamFS instance backing all VFS operations.
pub static RAMFS: RamFs = RamFs::new();

// ── DiskFS bridge buffer state ──────────────────────────────────────────────
/// One-time granted MemLend buffer VA for DiskFS bridge ops.
/// Granted via sys_grant_mem_lend(SLOT_BLOCK, 4096, SLOT_BUF_LEND) on first use.
/// SexFiles owns this buffer for its lifetime; Linen never sees it.
static DISKFS_BRIDGE_BUF_VA: AtomicU64 = AtomicU64::new(0);

/// Whether the manifest at LBA 2046 has been validated (or bootstrapped).
/// Set to true after the first successful diskfs_ensure_manifest() call.
/// Avoids redundant NVMe manifest reads on every bridge WRITE/READ.
static DISKFS_MANIFEST_READY: AtomicU64 = AtomicU64::new(0);

/// Currently selected DiskFS object path_id for bridge operations.
/// Default 0 = /disk/sexfiles-proof-v1. Single-client proof-only.
/// Set via OP_DISKFS_SELECT. Read by bridge WRITE/READ/STAT/HASH handlers.
static DISKFS_SELECTED_PATH_ID: AtomicU64 = AtomicU64::new(0);

/// Whether a SELECT has been issued at least once since boot.
static DISKFS_SELECT_USED: AtomicU64 = AtomicU64::new(0);

fn diskfs_bridge_get_buf_va() -> u64 {
    let va = DISKFS_BRIDGE_BUF_VA.load(Ordering::Relaxed);
    if va != 0 && va != u64::MAX {
        return va;
    }
    let new_va = sex_pdx::sys_grant_mem_lend(
        sex_pdx::SLOT_BLOCK, 4096, sex_pdx::SLOT_BUF_LEND,
    );
    if new_va != 0 && new_va != u64::MAX {
        DISKFS_BRIDGE_BUF_VA.store(new_va, Ordering::Relaxed);
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.buf.ready] buf_va={:#x}",
            new_va
        );
    }
    new_va
}

// ── DiskFS bridge inline handlers ───────────────────────────────────────────

fn handle_diskfs_select(path_id: u64) -> u64 {
    crate::pdx::serial_println!(
        "[sexfiles.bridge.diskfs.recv] op=0x3E select path_id={}",
        path_id
    );

    if path_id >= 3 {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.select.err] reason=bad_path_id path_id={}",
            path_id
        );
        return messages::ERR_BAD_CMD as u64;
    }

    let buf_va = diskfs_bridge_get_buf_va();
    if buf_va == 0 || buf_va == u64::MAX {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.select.err] reason=grant_failed"
        );
        return messages::ERR_NOT_FOUND as u64;
    }

    // Ensure V2 manifest on first SELECT.
    if DISKFS_MANIFEST_READY.load(Ordering::Relaxed) == 0 {
        if let Err(e) = DiskFs::diskfs_ensure_manifest_v2(buf_va) {
            crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.select.err] reason=manifest_ensure_v2_failed code={}",
                e
            );
            return e;
        }
        DISKFS_MANIFEST_READY.store(1, Ordering::Relaxed);
    }

    // Validate the selected path_id resolves.
    match DiskFs::diskfs_lookup_by_path_id(path_id, buf_va) {
        Ok(_entry) => {
            DISKFS_SELECTED_PATH_ID.store(path_id, Ordering::Relaxed);
            if DISKFS_SELECT_USED.load(Ordering::Relaxed) == 0 {
                DISKFS_SELECT_USED.store(1, Ordering::Relaxed);
                crate::pdx::serial_println!(
                    "[sexfiles.bridge.diskfs.select.v1_single_client]"
                );
            }
            crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.select.ok] path_id={}", path_id
            );
            0
        }
        Err(e) => {
            crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.select.err] path_id={} code={}",
                path_id, e
            );
            e
        }
    }
}

/// Resolve the currently selected path_id to a path slice.
/// Returns the path bytes, or an error if path_id is invalid.
fn diskfs_selected_path() -> Result<&'static [u8], u64> {
    let path_id = DISKFS_SELECTED_PATH_ID.load(Ordering::Relaxed);
    DiskFs::diskfs_path_for_id(path_id)
}

fn handle_diskfs_write(byte_offset: u64, data_lo: u64, data_hi: u64) -> u64 {
    crate::pdx::serial_println!(
        "[sexfiles.bridge.diskfs.recv] op=0x38 offset={}",
        byte_offset
    );

    if byte_offset >= messages::DISKFS_OBJECT_SIZE {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.write.err] reason=offset_past_end offset={}",
            byte_offset
        );
        return messages::ERR_OVERFLOW as u64;
    }

    let max_write = (messages::DISKFS_OBJECT_SIZE - byte_offset) as usize;
    if max_write < messages::DISKFS_MAX_WRITE {
        // Boundary: reject writes that would cross the 4096-byte boundary.
        // Preferred for V1 simplicity (option A from plan).
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.write.err] reason=boundary_write offset={} max={}",
            byte_offset, max_write
        );
        return messages::ERR_OVERFLOW as u64;
    }

    let buf_va = diskfs_bridge_get_buf_va();
    if buf_va == 0 || buf_va == u64::MAX {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.write.err] reason=grant_failed buf_va={:#x}",
            buf_va
        );
        return messages::ERR_NOT_FOUND as u64;
    }

    // Ensure V2 manifest exists on NVMe before attempting write.
    // Cached: only performs NVMe read on first bridge operation.
    if DISKFS_MANIFEST_READY.load(Ordering::Relaxed) == 0 {
        if let Err(e) = DiskFs::diskfs_ensure_manifest_v2(buf_va) {
            crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.write.err] reason=manifest_ensure_v2_failed code={}",
                e
            );
            return e;
        }
        DISKFS_MANIFEST_READY.store(1, Ordering::Relaxed);
    }

    // Resolve selected path.
    let path = match diskfs_selected_path() {
        Ok(p) => p,
        Err(e) => return e,
    };

    // Pack 16 bytes inline from data_lo + data_hi.
    let mut inline_data = [0u8; 16];
    {
        let lo = data_lo.to_le_bytes();
        let hi = data_hi.to_le_bytes();
        let mut i = 0;
        while i < 8 {
            inline_data[i] = lo[i];
            inline_data[i + 8] = hi[i];
            i += 1;
        }
    }

    match DiskFs::diskfs_write_object(
        path,
        byte_offset,
        &inline_data,
        buf_va,
    ) {
        Ok(n) => {
            crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.write.ok] offset={} written={}",
                byte_offset, n
            );
            n
        }
        Err(e) => {
            crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.write.err] offset={} code={}",
                byte_offset, e
            );
            e
        }
    }
}

fn handle_diskfs_read(byte_offset: u64, max_len: u64, caller_pd: u32) -> u64 {
    crate::pdx::serial_println!(
        "[sexfiles.bridge.diskfs.read.recv] caller={} off={} len={}",
        caller_pd, byte_offset, max_len
    );

    if max_len == 0 || max_len > messages::DISKFS_MAX_READ as u64 {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.read.err] reason=bad_max_len max_len={}",
            max_len
        );
        return messages::ERR_OVERFLOW as u64;
    }

    if byte_offset >= messages::DISKFS_OBJECT_SIZE {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.read.err] reason=offset_past_end offset={}",
            byte_offset
        );
        return messages::ERR_OVERFLOW as u64;
    }

    if byte_offset + max_len > messages::DISKFS_OBJECT_SIZE {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.read.err] reason=read_past_end offset={} max_len={}",
            byte_offset, max_len
        );
        return messages::ERR_OVERFLOW as u64;
    }

    let buf_va = diskfs_bridge_get_buf_va();
    if buf_va == 0 || buf_va == u64::MAX {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.read.err] reason=grant_failed buf_va={:#x}",
            buf_va
        );
        return messages::ERR_NOT_FOUND as u64;
    }

    // Ensure V2 manifest exists on NVMe before attempting read.
    // Cached: only performs NVMe read on first bridge operation.
    if DISKFS_MANIFEST_READY.load(Ordering::Relaxed) == 0 {
        if let Err(e) = DiskFs::diskfs_ensure_manifest_v2(buf_va) {
            crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.read.err] reason=manifest_ensure_v2_failed code={}",
                e
            );
            return e;
        }
        DISKFS_MANIFEST_READY.store(1, Ordering::Relaxed);
    }

    // Resolve selected path.
    let path = match diskfs_selected_path() {
        Ok(p) => p,
        Err(e) => return e,
    };

    let rlen = max_len as usize;
    let mut rbuf = [0u8; 8];
    match DiskFs::diskfs_read_object(
        path,
        byte_offset,
        &mut rbuf[..rlen],
        buf_va,
    ) {
        Ok(n) => {
            // Pack up to 8 bytes into reply u64 (LE).
            let mut reply: u64 = 0;
            let mut i = 0;
            while i < n as usize && i < 8 {
                reply |= (rbuf[i] as u64) << (i * 8);
                i += 1;
            }
            crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.read.reply] caller={} value={:#x} off={} read={}",
                caller_pd, reply, byte_offset, n
            );
            reply
        }
        Err(e) => {
            crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.read.err] caller={} err={} off={}",
                caller_pd, e, byte_offset
            );
            e
        }
    }
}

fn handle_diskfs_flush() -> u64 {
    crate::pdx::serial_println!(
        "[sexfiles.bridge.diskfs.recv] op=0x3A flush"
    );
    let status = DiskFs::diskfs_fsync();
    if status == 0 {
        crate::pdx::serial_println!("[sexfiles.bridge.diskfs.flush.ok]");
    } else {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.flush.err] status={} honest=flush_not_emulated_by_qemu_nvme",
            status
        );
    }
    status
}

fn handle_diskfs_stat() -> u64 {
    crate::pdx::serial_println!(
        "[sexfiles.bridge.diskfs.recv] op=0x3B stat"
    );
    let path_id = DISKFS_SELECTED_PATH_ID.load(Ordering::Relaxed);
    let path = match DiskFs::diskfs_path_for_id(path_id) {
        Ok(p) => p,
        Err(e) => return e,
    };
    // For now, all 3 objects have identical metadata (4096 bytes, READ|WRITE).
    // In future, per-object sizes can be read from the manifest entry.
    let flags: u64 = (DISKFS_MANIFEST_FLAG_READ | DISKFS_MANIFEST_FLAG_WRITE) as u64;
    let size: u64 = messages::DISKFS_OBJECT_SIZE;
    let packed = (flags << 32) | (size & 0xFFFF_FFFF);
    crate::pdx::serial_println!(
        "[sexfiles.bridge.diskfs.stat.ok] path_id={} path={} size={} flags={:#x}",
        path_id,
        core::str::from_utf8(path).unwrap_or("?"),
        size, flags
    );
    packed
}

fn handle_diskfs_manifest_hash() -> u64 {
    crate::pdx::serial_println!(
        "[sexfiles.bridge.diskfs.recv] op=0x3C manifest_hash"
    );
    let path_id = DISKFS_SELECTED_PATH_ID.load(Ordering::Relaxed);
    let path = match DiskFs::diskfs_path_for_id(path_id) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let hash = DiskFs::proof_manifest_name_hash(path);
    crate::pdx::serial_println!(
        "[sexfiles.bridge.diskfs.manifest_hash.ok] path_id={} hash={:#x}",
        path_id, hash
    );
    hash
}

/// Route a PDX message to the appropriate backend handler.
/// Called from the trampoline message loop.
/// `caller_pd` is the PD of the requesting process (from PDX message).
pub fn handle_vfs_message(type_id: u64, arg0: u64, arg1: u64, arg2: u64, caller_pd: u32) -> u64 {
    crate::pdx::serial_println!(
        "[sexfiles.vfs.enter] type={:#x} caller={} a0={:#x} a1={:#x}",
        type_id, caller_pd, arg0, arg1
    );
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

        // ── OP_RAMFS_READNAME ──
        // arg0 = handle, arg1 = byte_offset, arg2 = max_len (clamped to 8)
        // Returns: up to 8 filename bytes LE. 0 = EOF. negative = error.
        messages::OP_RAMFS_READNAME => {
            let handle = arg0;
            let byte_offset = arg1;
            let max_len = (arg2 as usize).min(8);
            let mut buf = [0u8; 8];
            match RAMFS.read(handle, byte_offset, &mut buf[..max_len], caller_pd) {
                Ok(n) => {
                    let mut packed = 0u64;
                    for i in 0..(n as usize).min(8) {
                        packed |= (buf[i] as u64) << (i * 8);
                    }
                    crate::pdx::serial_println!(
                        "[sexfiles.ramfs.readname.ok] handle={} off={} len={}",
                        handle, byte_offset, max_len
                    );
                    packed
                }
                Err(e) => {
                    crate::pdx::serial_println!(
                        "[sexfiles.ramfs.readname.deny] handle={} err={}",
                        handle, e
                    );
                    e as u64
                }
            }
        }

        // ── DiskFS bridge opcodes (0x38-0x3C) ──
        // Route: Linen → SLOT_STORAGE → SexFiles → DiskFS → SLOT_BLOCK → SexDrive → NVMe
        // Fixed-object bridge: /disk/sexfiles-proof-v1
        messages::OP_DISKFS_WRITE => {
            // arg0 = byte_offset, arg1 = data_lo, arg2 = data_hi
            crate::pdx::serial_println!("[sexfiles.route.dispatch] op=0x38 name=write caller={}", caller_pd);
            let reply = handle_diskfs_write(arg0, arg1, arg2);
            crate::pdx::serial_println!("[sexfiles.route.reply] op=0x38 caller={} value={:#x}", caller_pd, reply);
            reply
        }
        messages::OP_DISKFS_READ => {
            // arg0 = byte_offset, arg1 = max_len, arg2 = 0 (reserved)
            crate::pdx::serial_println!("[sexfiles.route.dispatch] op=0x39 name=read caller={}", caller_pd);
            let reply = handle_diskfs_read(arg0, arg1, caller_pd);
            crate::pdx::serial_println!("[sexfiles.route.reply] op=0x39 caller={} value={:#x}", caller_pd, reply);
            reply
        }
        messages::OP_DISKFS_FLUSH => {
            crate::pdx::serial_println!("[sexfiles.route.dispatch] op=0x3A name=flush caller={}", caller_pd);
            let reply = handle_diskfs_flush();
            crate::pdx::serial_println!("[sexfiles.route.reply] op=0x3A caller={} value={:#x}", caller_pd, reply);
            reply
        }
        messages::OP_DISKFS_STAT => {
            crate::pdx::serial_println!("[sexfiles.route.dispatch] op=0x3B name=stat caller={}", caller_pd);
            let reply = handle_diskfs_stat();
            crate::pdx::serial_println!("[sexfiles.route.reply] op=0x3B caller={} value={:#x}", caller_pd, reply);
            reply
        }
        messages::OP_DISKFS_MANIFEST_HASH => {
            crate::pdx::serial_println!("[sexfiles.route.dispatch] op=0x3C name=hash caller={}", caller_pd);
            let reply = handle_diskfs_manifest_hash();
            crate::pdx::serial_println!("[sexfiles.route.reply] op=0x3C caller={} value={:#x}", caller_pd, reply);
            reply
        }
        messages::OP_DISKFS_SELECT => {
            // arg0 = path_id, arg1/arg2 = 0 (reserved)
            crate::pdx::serial_println!("[sexfiles.route.dispatch] op=0x3E name=select caller={}", caller_pd);
            let reply = handle_diskfs_select(arg0);
            crate::pdx::serial_println!("[sexfiles.route.reply] op=0x3E caller={} value={:#x}", caller_pd, reply);
            reply
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
