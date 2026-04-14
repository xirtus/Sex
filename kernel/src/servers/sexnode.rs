use crate::serial_println;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use spin::RwLock;
use lazy_static::lazy_static;

lazy_static! {
    /// Registry of known active nodes in the sexnode.
    /// Maps Node ID to its network address (simulated as u32).
    pub static ref CLUSTER_NODES: RwLock<BTreeMap<u32, u32>> = RwLock::new(BTreeMap::new());

    /// Redox-style URL Scheme Registry.
    /// Maps a scheme (e.g., "sexnet") to a target PD ID.
    static ref SCHEME_REGISTRY: RwLock<BTreeMap<&'static str, u32>> = RwLock::new(BTreeMap::new());
}

/// Registers a URL scheme with a specific system server PD.
pub fn register_scheme(scheme: &'static str, pd_id: u32) {
    let mut schemes = SCHEME_REGISTRY.write();
    schemes.insert(scheme, pd_id);
    serial_println!("sexnode: Registered scheme {}:// to PD {}", scheme, pd_id);
}

/// Resolves a resource URL into a system capability or PD ID.
pub fn resolve_url(url: &str) -> Result<u32, &'static str> {
    serial_println!("sexnode: Resolving URL: {}", url);
    
    // In a real system, this would parse the URL (scheme://host/path)
    if url.starts_with("sexnet://") {
        let schemes = SCHEME_REGISTRY.read();
        return schemes.get("sexnet").cloned().ok_or("Scheme not found");
    } else if url.starts_with("sexdrm://") {
        let schemes = SCHEME_REGISTRY.read();
        return schemes.get("sexdrm").cloned().ok_or("Scheme not found");
    }
    
    Err("sexnode: Unknown URL scheme")
}

/// The sexnode PD manages node discovery and distributed capabilities.
pub struct sexnode {
...    // In a real system, this would listen on a UDP multicast socket
    // via the sexnet to discover peers.
}

/// Simulated PDX entry point for the sexnode.
pub extern "C" fn sexnode_entry(arg: u64) -> u64 {
    serial_println!("sexnode: Received command: {:#x}", arg);
    0
}

/// Simulates discovering a new node on the network.
pub fn discover_node(node_id: u32, network_addr: u32) {
    let mut nodes = CLUSTER_NODES.write();
    nodes.insert(node_id, network_addr);
    serial_println!("sexnode: Discovered Node {} at Address {:#x}", node_id, network_addr);
}

/// Simulates receiving an exported capability from a remote node.
pub fn import_remote_capability(source_node: u32, target_pd: u32, cap_id: u32) -> u32 {
    serial_println!("sexnode: Imported capability {} from Node {} (Target PD: {})",
        cap_id, source_node, target_pd);
    // Return a local handle/ID for this imported capability
    cap_id + 1000
}
