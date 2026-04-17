use sex_pdx::PageHandover;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::string::String;

pub struct Cache {
    pub hits: AtomicU64,
    entries: Mutex<BTreeMap<String, PageHandover>>,
}

pub static CACHE: Cache = Cache {
    hits: AtomicU64::new(0),
    entries: Mutex::new(BTreeMap::new()),
};

impl Cache {
    pub fn insert(&self, name: &str, image: PageHandover) {
        let mut entries = self.entries.lock();
        if entries.len() > 1024 {
            entries.clear();
        }
        entries.insert(String::from(name), image);
    }

    pub fn lookup(&self, name: &str) -> Option<PageHandover> {
        self.hits.fetch_add(1, Ordering::Relaxed);
        self.entries.lock().get(name).copied()
    }

    pub fn invalidate(&self, name: &str) {
        self.entries.lock().remove(name);
    }
}
