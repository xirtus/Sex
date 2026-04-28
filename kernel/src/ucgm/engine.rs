use core::sync::atomic::{AtomicU64, Ordering};
use alloc::collections::BTreeMap;
use spin::Mutex;

#[derive(Clone, Copy, Debug)]
pub struct Transaction {
    pub actor_id: u32,
    pub version: u64,
    pub timestamp: u64,
}

pub struct UCGMTransactionEngine {
    pub version: AtomicU64,
    pub state_store: Mutex<BTreeMap<u32, u64>>, // Simple state projection
}

pub static ENGINE: UCGMTransactionEngine = UCGMTransactionEngine {
    version: AtomicU64::new(0),
    state_store: Mutex::new(BTreeMap::new()),
};

impl UCGMTransactionEngine {
    pub fn commit(&self, tx: Transaction, delta: BTreeMap<u32, u64>) -> bool {
        let mut store = self.state_store.lock();
        let current_version = self.version.load(Ordering::Acquire);
        
        if tx.version != current_version {
            return false;
        }

        for (k, v) in delta {
            store.insert(k, v);
        }
        
        self.version.fetch_add(1, Ordering::SeqCst);
        true
    }

    pub fn snapshot(&self) -> (u64, BTreeMap<u32, u64>) {
        let store = self.state_store.lock();
        (self.version.load(Ordering::Acquire), store.clone())
    }
}
