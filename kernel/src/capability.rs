use alloc::vec::Vec;
use x86_64::VirtAddr;
use spin::Mutex;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, Ordering};
use crate::cheri::SexCapability;

/// Capability Types as defined in ARCHITECTURE.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityKind {
    Memory,
    DMA,       // New: Contiguous DMA capability
    IPC,      
    Interrupt, 
    Domain,    
    Node,      // Unified sexvfs Node capability
    Spawn,     // PD Spawning capability
    Pci,       // PCI Hardware access
    Network,   // Network Interface capability
    Socket,    // Network Socket capability
    MemLend,   // Zero-copy memory lending capability
    RemoteProxy, // Distributed capability proxy
}

/// IPCtax Mandate: 64-byte alignment to prevent false sharing in 128-core interconnects.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalCapId {
    pub node_id: u32,
    pub local_id: u32,
    pub generation: u32,
    pub padding: [u32; 13], // Fill to 64 bytes
}

#[derive(Debug, Clone, Copy)]
pub struct MemLendCapData {
    pub base: u64,
    pub length: u64,
    pub pku_key: u8,
    pub permissions: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct IpcCapData {
    pub node_id: u32,
    pub target_pd_id: u32,
    pub entry_point: VirtAddr,
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryCapData {
    pub cheri_cap: SexCapability,
    pub pku_key: u8,
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
pub struct SpawnCapData {
    pub max_pds: u32,
    pub allowed_pku_keys: u32, // Bitmask
}

#[derive(Debug, Clone, Copy)]
pub struct PciCapData {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
    pub vendor_id: u16,
    pub device_id: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkCapData {
    pub interface_id: u32,
    pub mac_address: [u8; 6],
}

#[derive(Debug, Clone, Copy)]
pub struct SocketCapData {
    pub protocol: u8, // 6 = TCP, 17 = UDP
    pub local_port: u16,
    pub remote_addr: [u8; 4],
    pub remote_port: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct DmaCapData {
    pub phys_addr: u64,
    pub length: u64,
    pub pku_key: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum CapabilityData {
    Memory(MemoryCapData),
    DMA(DmaCapData), // Added DMA data
    IPC(IpcCapData),
    Interrupt(InterruptCapData),
    Domain(u32), 
    Node(NodeCapData),
    Spawn(SpawnCapData),
    Pci(PciCapData),
    Network(NetworkCapData),
    Socket(SocketCapData),
    MemLend(MemLendCapData),
    RemoteProxy(GlobalCapId),
}

#[derive(Debug, Clone, Copy)]
pub struct Capability {
    pub id: u32,
    pub data: CapabilityData,
}

use alloc::collections::BTreeMap;

/// A Protection Domain (PD) represents an isolated execution context.
pub struct ProtectionDomain {
    pub id: u32,
    pub pku_key: u8,
    pub base_pkru_mask: u32,
    /// The current PKRU mask, updated by memory capabilities or "Domain Fusion".
    pub current_pkru_mask: AtomicU32,
    pub cap_table: Arc<CapabilityTable>,
    /// POSIX Signal Handlers (Signum -> Handler Entry Point)
    pub signal_handlers: Mutex<BTreeMap<i32, u64>>,
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

    /// Finds a capability that covers a specific virtual address.
    pub fn find_by_addr(&self, addr: u64) -> Option<Capability> {
        self.caps.lock().iter().find(|c| {
            match c.data {
                CapabilityData::Memory(data) => {
                    addr >= data.cheri_cap.base && addr < data.cheri_cap.base + data.cheri_cap.length
                },
                CapabilityData::MemLend(data) => {
                    addr >= data.base && addr < data.base + data.length
                },
                _ => false,
            }
        }).copied()
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
            signal_handlers: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn grant(&self, data: CapabilityData) -> u32 {
        self.cap_table.insert(data)
    }

    /// Activates a memory capability by validating bounds and updating PKRU.
    pub fn activate_memory_cap(&self, cap_id: u32) -> Result<(), &'static str> {
        let cap = self.cap_table.find(cap_id).ok_or("Cap: Not found")?;
        match cap.data {
            CapabilityData::Memory(data) => {
                // 1. CHERI-style Software Bounds Check
                if !data.cheri_cap.is_valid() {
                    return Err("CHERI: Capability bounds violation.");
                }

                // 2. Hardware PKU Activation
                let shift = data.pku_key * 2;
                let mut current = self.current_pkru_mask.load(Ordering::SeqCst);
                
                // Enable Access (Read)
                current &= !(0b01 << shift);
                
                // Enable Write if permission bit 1 is set
                if (data.cheri_cap.permissions & 2) != 0 {
                    current &= !(0b10 << shift);
                }

                self.current_pkru_mask.store(current, Ordering::SeqCst);
                serial_println!("CHERI: Activated Memory Capability {} (Base: {:#x}, Len: {})", 
                    cap_id, data.cheri_cap.base, data.cheri_cap.length);
                Ok(())
            },
            _ => Err("Cap: Not a memory capability"),
        }
    }

    /// Activates a memory lending capability.
    pub fn activate_mem_lend(&self, cap_id: u32) -> Result<(), &'static str> {
        let cap = self.cap_table.find(cap_id).ok_or("Cap: Not found")?;
        match cap.data {
            CapabilityData::MemLend(data) => {
                let shift = data.pku_key * 2;
                let mut current = self.current_pkru_mask.load(Ordering::SeqCst);
                
                // Enable Read
                current &= !(0b01 << shift);
                // Enable Write if permission bit 1 is set
                if (data.permissions & 2) != 0 {
                    current &= !(0b10 << shift);
                }

                self.current_pkru_mask.store(current, Ordering::SeqCst);
                serial_println!("SHM: Activated Memory Lending {} (Base: {:#x})", cap_id, data.base);
                Ok(())
            },
            _ => Err("Cap: Not a MemLend capability"),
        }
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

    /// Activates a DMA capability.
    pub fn activate_dma_cap(&self, cap_id: u32) -> Result<(), &'static str> {
        let cap = self.cap_table.find(cap_id).ok_or("Cap: Not found")?;
        match cap.data {
            CapabilityData::DMA(data) => {
                let shift = data.pku_key * 2;
                let mut current = self.current_pkru_mask.load(Ordering::SeqCst);
                
                // DMA usually requires R/W access for the hardware and PD
                current &= !(0b11 << shift); 

                self.current_pkru_mask.store(current, Ordering::SeqCst);
                serial_println!("DMA: Activated Contiguous DMA Capability {} (Phys: {:#x})", 
                    cap_id, data.phys_addr);
                Ok(())
            },
            _ => Err("Cap: Not a DMA capability"),
        }
    }
}
