use crate::serial_println;
use crate::capability::{NodeCapData, CapabilityData};
use crate::ipc::DOMAIN_REGISTRY;
use alloc::collections::BTreeMap;
use alloc::string::String;
use spin::RwLock;
use lazy_static::lazy_static;

lazy_static! {
    /// Global registry of mount points.
    /// Maps a path (e.g., "/disk0") to a sexdrive's Protection Domain ID.
    static ref MOUNT_POINTS: RwLock<BTreeMap<String, u32>> = RwLock::new(BTreeMap::new());
}

/// The sexvfs Server's entry point for PDX calls.
pub extern "C" fn sexvfs_entry(arg: u64) -> u64 {
    // For simplicity, we'll use a request structure passed by pointer or 
    // split the 64-bit arg into (RequestType, Data).
    
    let req_type = (arg >> 32) as u32;
    let req_data = (arg & 0xFFFF_FFFF) as u32;

    match req_type {
        1 => { // OPEN
            serial_println!("sexvfs: Open request for node ID: {}", req_data);
            0
        },
        2 => { // MOUNT
            serial_println!("sexvfs: Mount request for sexdrive PD: {}", req_data);
            0
        },
        3 => { // MOUNT BTRFS
            serial_println!("sexvfs: Mounting Btrfs volume on sexdrive PD: {}", req_data);
            0
        },
        4 => { // MOUNT NTFS
            serial_println!("sexvfs: Mounting NTFS volume on sexdrive PD: {}", req_data);
            0
        },
        _ => {
            serial_println!("sexvfs: Unknown request type: {}", req_type);
            u64::MAX
        }
    }
}

pub fn mount(path: &str, sexdrive_pd_id: u32, fs_type: &str) {
    let mut mounts = MOUNT_POINTS.write();
    mounts.insert(String::from(path), sexdrive_pd_id);
    serial_println!("sexvfs: Mounted {} ({}) to sexdrive PD {}", path, fs_type, sexdrive_pd_id);
}

/// Simulates opening a file.
/// In a real implementation, this would involve path resolution and 
/// interaction with the storage sexdrive to get a node ID.
pub fn open(caller_pd_id: u32, _path: &str) -> Result<u32, &'static str> {
    // 1. Resolve path to sexdrive PD
    // For this demo, let's assume everything is on sexdrive 10.
    let sexdrive_pd_id = 10;
    let inode_id = 42; // Simulated inode

    // 2. Create a Node Capability
    let node_cap = CapabilityData::Node(NodeCapData {
        node_id: 1, // Assume local node is 1
        sexdrive_pd_id,
        inode_id,
        permissions: 0x7, // R/W/X
    });

    // 3. Grant the capability to the caller
    let registry = DOMAIN_REGISTRY.read();
    let caller_pd = registry.get(&caller_pd_id)
        .ok_or("sexvfs: Caller PD not found")?;

    Ok(caller_pd.grant(node_cap))
}
