use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use alloc::vec::Vec;

pub struct Transactions {
    pub transaction_id: AtomicU64,
    pub pending: Mutex<Vec<u64>>, // List of active TX IDs
}

pub static TRANSACTIONS: Transactions = Transactions {
    transaction_id: AtomicU64::new(1),
    pending: Mutex::new(Vec::new()),
};

impl Transactions {
    pub fn begin(&self) -> u64 {
        let id = self.transaction_id.fetch_add(1, Ordering::SeqCst);
        self.pending.lock().push(id);
        // PDX Slot 1: VfsOpen("/etc/sexshop/wal/<id>")
        id
    }

    pub fn commit(&self) -> i32 {
        let mut pending = self.pending.lock();
        if let Some(_id) = pending.pop() {
            // PDX Slot 1: VfsRename("/etc/sexshop/wal/<id>", "/etc/sexshop/commit/<id>")
            // PDX Slot 1: VfsSync
            0
        } else {
            -1
        }
    }

    pub fn abort(&self) -> i32 {
        let mut pending = self.pending.lock();
        if let Some(_id) = pending.pop() {
            // PDX Slot 1: VfsUnlink("/etc/sexshop/wal/<id>")
            0
        } else {
            -1
        }
    }
}
