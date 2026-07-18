//! Thin wrappers over sex-pdx for sexfiles server.
//! Uses standard pdx_listen_raw(0) / pdx_reply pattern.

pub use sex_pdx::{
    pdx_listen_raw, pdx_reply, serial_println,
    SLOT_BLOCK,
    BLOCK_ERR_BAD_CMD, BLOCK_ERR_BAD_LEN, BLOCK_ERR_NO_DEVICE,
};

/// SEXFILES_DEFER_V1: client requests that arrive while a nested
/// reply-wait loop (diskfs_block_call) is listening on slot 0. The old
/// loop DISCARDED them as "stale startup messages" — every NVMe block
/// roundtrip made on behalf of client A silently ate any request client
/// B sent during the wait (root cause of the vanished-request hangs).
/// Stash them here; the trampoline main loop drains before listening.
/// Single-threaded PD — plain statics, no atomics needed.
const DEFER_CAP: usize = 8;
static mut DEFER_RING: [(u64, u32, u64, u64, u64); DEFER_CAP] = [(0, 0, 0, 0, 0); DEFER_CAP];
static mut DEFER_LEN: usize = 0;

pub fn defer_stash(type_id: u64, caller_pd: u32, a0: u64, a1: u64, a2: u64) -> bool {
    unsafe {
        if DEFER_LEN >= DEFER_CAP {
            serial_println!(
                "[sexfiles.defer.drop] type={:#x} caller={} reason=stash_full",
                type_id, caller_pd
            );
            return false;
        }
        DEFER_RING[DEFER_LEN] = (type_id, caller_pd, a0, a1, a2);
        DEFER_LEN += 1;
        serial_println!(
            "[sexfiles.defer.stash] type={:#x} caller={} depth={}",
            type_id, caller_pd, DEFER_LEN
        );
        true
    }
}

/// Message shape shared by the live-listen and defer-replay paths in the
/// trampoline loop (mirrors the PdxMessage fields the loop uses).
pub struct ReplayMsg {
    pub type_id: u64,
    pub caller_pd: u32,
    pub arg0: u64,
    pub arg1: u64,
    pub arg2: u64,
}

pub fn defer_pop() -> Option<(u64, u32, u64, u64, u64)> {
    unsafe {
        if DEFER_LEN == 0 { return None; }
        let msg = DEFER_RING[0];
        for i in 1..DEFER_LEN {
            DEFER_RING[i - 1] = DEFER_RING[i];
        }
        DEFER_LEN -= 1;
        serial_println!(
            "[sexfiles.defer.replay] type={:#x} caller={} depth={}",
            msg.0, msg.1, DEFER_LEN
        );
        Some(msg)
    }
}
