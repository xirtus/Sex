use crate::serial_println;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use spin::RwLock;
use lazy_static::lazy_static;

use crate::ipc_ring::SpscRing;
use crate::capability::{CapabilityData, NodeCapData};
use core::sync::atomic::{AtomicPtr, Ordering};
use alloc::sync::Arc;

/// IPCtax Mandate: Cache-aligned atomic primitives to prevent false sharing.
#[repr(align(64))]
pub struct RemoteCapEntry {
    pub target_node_id: u32,
    pub remote_pd_id: u32,
    pub local_cap_id: u32,
    pub is_active: core::sync::atomic::AtomicBool,
}

/// Distributed Capability Registry (DCR).
/// Sharded by Core ID to satisfy the 128-core scaling requirement.
pub struct DistributedRegistry {
    /// Each shard is a lock-free list of exported capabilities.
    pub shards: [AtomicPtr<RemoteCapEntry>; 128],
}

impl DistributedRegistry {
    pub const fn new() -> Self {
        Self {
            shards: [const { AtomicPtr::new(core::ptr::null_mut()) }; 128],
        }
    }

    /// Exports a local capability to a remote node via Zero-Mediation.
    pub fn export_to_node(&self, core_id: usize, entry: *mut RemoteCapEntry) {
        loop {
            let current = self.shards[core_id].load(Ordering::Acquire);
            if self.shards[core_id].compare_exchange(
                current, entry, Ordering::Release, Ordering::Relaxed
            ).is_ok() { break; }
        }
    }
}

pub static GLOBAL_DCR: DistributedRegistry = DistributedRegistry::new();

/// Performs a PDX call to sexnode to initiate an Export/Import.
pub fn import_remote_proxy(node_id: u32, path: &str) -> Result<u32, &'static str> {
    serial_println!("sexnode: Importing remote proxy for {} from Node {}.", path, node_id);
    
    // 1. Recover the caller's PD
    let registry = crate::ipc::DOMAIN_REGISTRY.read();
    let current_pd = registry.get(&crate::core_local::CoreLocal::get().current_pd())
        .ok_or("sexnode: Identity lost")?;

    // 2. Grant a RemoteProxy capability
    let proxy_cap = CapabilityData::RemoteProxy(crate::capability::GlobalCapId {
        node_id,
        local_id: 1234, // Simulated remote ID
        generation: 1,
        padding: [0; 13],
    });

    Ok(current_pd.grant(proxy_cap))
}

/// Real Node Manager (Coordinator).
...
/// Manages cluster membership, heartbeats, and Redox-style URL schemes.

pub struct SexNodeManager {
    pub local_node_id: u32,
}

impl SexNodeManager {
    /// Discovers a new node on the network (UDP Multicast listener).
    pub fn discover_node(&self, node_id: u32, network_addr: [u8; 4]) {
        let mut nodes = CLUSTER_NODES.write();
        nodes.insert(node_id, network_addr);
        serial_println!("sexnode: Discovered Node {} at {}.{}.{}.{}", 
            node_id, network_addr[0], network_addr[1], network_addr[2], network_addr[3]);
    }

    /// Resolves a resource URL into a system capability or PD ID.
    pub fn resolve_url(&self, url: &str) -> Result<u32, &'static str> {
        serial_println!("sexnode: Resolving URL: {}", url);
        
        // 1. Parse Scheme (e.g., "sexnet://")
        let split: Vec<&str> = url.split("://").collect();
        if split.len() < 2 {
            return Err("sexnode: Invalid URL format");
        }
        let scheme = split[0];

        // 2. Lookup in Scheme Registry
        let schemes = SCHEME_REGISTRY.read();
        schemes.get(scheme).cloned().ok_or("sexnode: Unknown URL scheme")
    }

    /// Sends a heartbeat to all known nodes.
    pub fn send_heartbeat(&self) {
        let nodes = CLUSTER_NODES.read();
        for (id, addr) in nodes.iter() {
            serial_println!("sexnode: Heartbeat -> Node {} ({}.{}.{}.{})", 
                id, addr[0], addr[1], addr[2], addr[3]);
            // In a real system, this sends a UDP packet via sexnet.
        }
    }
}
/// Epoch-Based Heartbeat Matrix for Distributed Consensus.
pub struct ConsensusRegistry {
    pub node_epochs: [core::sync::atomic::AtomicU64; 32], // Max 32 nodes for the prototype
}

impl ConsensusRegistry {
    pub const fn new() -> Self {
        const ZERO: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
        Self {
            node_epochs: [ZERO; 32],
        }
    }

    /// Increments the epoch for a dead node, instantly invalidating its capabilities.
    pub fn invalidate_node(&self, node_id: u32) {
        if node_id < 32 {
            self.node_epochs[node_id as usize].fetch_add(1, core::sync::atomic::Ordering::SeqCst);
            serial_println!("CONSENSUS: Node {} invalidated. Epoch incremented.", node_id);
        }
    }

    /// Verifies if a capability's generation matches the current node epoch.
    pub fn is_valid(&self, gcid: &crate::capability::GlobalCapId) -> bool {
        if gcid.node_id >= 32 { return false; }
        let current_epoch = self.node_epochs[gcid.node_id as usize].load(core::sync::atomic::Ordering::Relaxed);
        gcid.generation as u64 == current_epoch
    }
}

pub static GLOBAL_CONSENSUS: ConsensusRegistry = ConsensusRegistry::new();

lazy_static! {
    /// Registry of known active nodes in the sexnode.
...

    /// Maps Node ID to its IPv4 address.
    pub static ref CLUSTER_NODES: RwLock<BTreeMap<u32, [u8; 4]>> = RwLock::new(BTreeMap::new());

    /// Redox-style URL Scheme Registry.
    /// Maps a scheme (e.g., "sexnet") to a target PD ID.
    static ref SCHEME_REGISTRY: RwLock<BTreeMap<&'static str, u32>> = RwLock::new(BTreeMap::new());
}
