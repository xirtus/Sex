use crate::serial_println;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use spin::RwLock;
use lazy_static::lazy_static;

lazy_static! {
    /// Registry of known active nodes in the cluster.
    /// Maps Node ID to its network address (simulated as u32).
    pub static ref CLUSTER_NODES: RwLock<BTreeMap<u32, u32>> = RwLock::new(BTreeMap::new());
}

/// The Cluster Server PD manages node discovery and distributed capabilities.
pub struct ClusterServer {
    // In a real system, this would listen on a UDP multicast socket
    // via the NetStack to discover peers.
}

/// Simulated PDX entry point for the Cluster Server.
pub extern "C" fn cluster_entry(arg: u64) -> u64 {
    serial_println!("CLUSTER: Received command: {:#x}", arg);
    0
}

/// Simulates discovering a new node on the network.
pub fn discover_node(node_id: u32, network_addr: u32) {
    let mut nodes = CLUSTER_NODES.write();
    nodes.insert(node_id, network_addr);
    serial_println!("CLUSTER: Discovered Node {} at Address {:#x}", node_id, network_addr);
}

/// Simulates receiving an exported capability from a remote node.
pub fn import_remote_capability(source_node: u32, target_pd: u32, cap_id: u32) -> u32 {
    serial_println!("CLUSTER: Imported capability {} from Node {} (Target PD: {})",
        cap_id, source_node, target_pd);
    // Return a local handle/ID for this imported capability
    cap_id + 1000
}
