use sex_pdx::{StoreProtocol, PdxReply, PageHandover, MessageType};
use core::sync::atomic::{AtomicU64, Ordering};
use crate::storage::STORAGE;
use crate::cache::CACHE;
use crate::transactions::TRANSACTIONS;

pub static IPC_OPS_TOTAL: AtomicU64 = AtomicU64::new(0);
pub static ZERO_COPY_HANDOVERS: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
pub fn handle_store_message(msg: &StoreProtocol, reply: &mut PdxReply) {
    IPC_OPS_TOTAL.fetch_add(1, Ordering::Relaxed);
    match msg {
        StoreProtocol::FetchPackage { name } => {
            let name_str = core::str::from_utf8(name).unwrap_or("").trim_matches('\0');
            match STORAGE.fetch_package(name_str) {
                Ok(handover) => {
                    reply.status = 0;
                    reply.size = handover.pfn; // Return PFN for SASOS map
                },
                Err(e) => reply.status = e as i64,
            }
        },
        StoreProtocol::CacheBinary { name, image } => {
            let name_str = core::str::from_utf8(name).unwrap_or("").trim_matches('\0');
            CACHE.insert(name_str, *image);
            reply.status = 0;
        },
        StoreProtocol::TransactionBegin => {
            reply.status = TRANSACTIONS.begin() as i64;
        },
        StoreProtocol::TransactionCommit => {
            reply.status = TRANSACTIONS.commit() as i64;
        },
        StoreProtocol::TransactionAbort => {
            reply.status = TRANSACTIONS.abort() as i64;
        },
        StoreProtocol::KVGet { key } => {
            match STORAGE.kv_get(key) {
                Ok(val) => {
                    reply.status = 0;
                    reply.size = val;
                },
                Err(e) => reply.status = e as i64,
            }
        },
        StoreProtocol::KVSet { key, value_paddr, value_len } => {
            reply.status = STORAGE.kv_set(key, *value_paddr, *value_len) as i64;
        },
        StoreProtocol::KVDelete { key } => {
            // Implementation: STORAGE.kv_delete(key)
            reply.status = 0;
        },
        StoreProtocol::ObjectPut { hash, data_paddr, data_len } => {
            reply.status = STORAGE.object_put(hash, *data_paddr, *data_len) as i64;
        },
        StoreProtocol::ObjectGet { hash } => {
            match STORAGE.object_get(hash) {
                Ok(handover) => {
                    reply.status = 0;
                    reply.size = handover.pfn;
                },
                Err(e) => reply.status = e as i64,
            }
        },
        StoreProtocol::ObjectExists { hash } => {
            reply.status = if STORAGE.exists(hash) { 1 } else { 0 };
        },
        StoreProtocol::ObjectMove { hash, target_node } => {
            reply.status = STORAGE.object_move(hash, *target_node) as i64;
        },
        StoreProtocol::SyncFilesystem => {
            // implementation: pdx_call(1, VfsSync)
            reply.status = 0;
        },
        StoreProtocol::Stats => {
            reply.status = IPC_OPS_TOTAL.load(Ordering::Relaxed) as i64;
            reply.size = ZERO_COPY_HANDOVERS.load(Ordering::Relaxed);
        },
        _ => {
            reply.status = -1;
        }
    }
}
