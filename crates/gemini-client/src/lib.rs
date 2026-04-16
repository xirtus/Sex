#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;

pub struct GeminiClient;

impl GeminiClient {
    pub fn new() -> Self { GeminiClient }
    
    pub fn analyze_fault(&self, _addr: u64) -> Option<RepairStrategy> {
        None
    }
}

pub struct RepairStrategy {
    pub actions: Vec<RepairAction>,
}

pub enum RepairAction {
    Fetch(String),
    Compile(String),
    Swap(u32),
}
