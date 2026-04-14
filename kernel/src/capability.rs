use alloc::vec::Vec;
use x86_64::VirtAddr;
use spin::Mutex;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, Ordering};

/// Capability Types as defined in ARCHITECTURE.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityKind {
    Memory,
    IPC,      
    Interrupt, 
    Domain,    
    Node,      // Unified VFS Node capability
}

#[derive(Debug, Clone, Copy)]
pub struct IpcCapData {
    pub node_id: u32,
    pub target_pd_id: u32,
    pub entry_point: VirtAddr,
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryCapData {
    pub start: VirtAddr,
    pub size: u64,
    pub pku_key: u8,
    pub permissions: u8, 
}

#[derive(Debug, Clone, Copy)]
pub struct InterruptCapData {
    pub irq: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeCapData {
    pub node_id: u32,
    pub sexdrive_pd_id: u32,
    pub inode_id: u64,
    pub permissions: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum CapabilityData {
    Memory(MemoryCapData),
    IPC(IpcCapData),
    Interrupt(InterruptCapData),
    Domain(u32), 
    Node(NodeCapData),
}

#[derive(Debug, Clone, Copy)]
pub struct Capability {
    pub id: u32,
    pub data: CapabilityData,
}

/// A Protection Domain (PD) represents an isolated execution context.
pub struct ProtectionDomain {
    pub id: u32,
    pub pku_key: u8,
    pub base_pkru_mask: u32,
    /// The current PKRU mask, updated by memory capabilities or "Domain Fusion".
    pub current_pkru_mask: AtomicU32,
    pub cap_table: Arc<CapabilityTable>,
}

/// Per-domain Capability Table.
/// In a 128-core system, this is sharded or core-local to avoid global locks.
pub struct CapabilityTable {
    // For the demo, we use a Mutex-protected Vec, but the API 
    // is designed for a lockless/distributed backend.
    caps: Mutex<Vec<Capability>>,
    next_cap_id: AtomicU32,
}

impl CapabilityTable {
    pub fn new() -> Self {
        Self {
            caps: Mutex::new(Vec::new()),
            next_cap_id: AtomicU32::new(1),
        }
    }

    pub fn insert(&self, data: CapabilityData) -> u32 {
        let id = self.next_cap_id.fetch_add(1, Ordering::SeqCst);
        self.caps.lock().push(Capability { id, data });
        id
    }

    pub fn find(&self, id: u32) -> Option<Capability> {
        self.caps.lock().iter().find(|c| c.id == id).copied()
    }
}

impl ProtectionDomain {
    pub fn new(id: u32, pku_key: u8) -> Self {
        let mut pkru_mask: u32 = 0xFFFF_FFFF;
        let shift = pku_key * 2;
        pkru_mask &= !(0b11 << shift);
        
        Self {
            id,
            pku_key,
            base_pkru_mask: pkru_mask,
            current_pkru_mask: AtomicU32::new(pkru_mask),
            cap_table: Arc::new(CapabilityTable::new()),
        }
    }

    pub fn grant(&self, data: CapabilityData) -> u32 {
        self.cap_table.insert(data)
    }

    /// Implement "Domain Fusion" (Phase 2 Step 2.3):
    /// Temporarily merges the access rights of another domain into this one.
    pub fn fuse_with(&self, other_pku_key: u8) {
        let shift = other_pku_key * 2;
        // Enable R/W for the other domain's key in our mask
        let mut current = self.current_pkru_mask.load(Ordering::SeqCst);
        current &= !(0b11 << shift);
        self.current_pkru_mask.store(current, Ordering::SeqCst);
    }

    /// Revokes fusion or a specific memory capability.
    pub fn revoke_access(&self, pku_key: u8) {
        let shift = pku_key * 2;
        let mut current = self.current_pkru_mask.load(Ordering::SeqCst);
        current |= 0b11 << shift; // Disable access
        self.current_pkru_mask.store(current, Ordering::SeqCst);
    }
}
