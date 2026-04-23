#![no_std]
use alloc::vec::Vec;
use sex_pdx::MessageType;

pub struct BuildCache {
    pub workspace_builds: Vec<(u64, &'static str)>, // (workspace_mask, status)
}

impl BuildCache {
    pub fn new() -> Self { BuildCache { workspace_builds: Vec::new() } }
    pub fn record_build(&mut self, workspace: u64, status: &'static str) {
        self.workspace_builds.push((workspace, status));
    }
}
