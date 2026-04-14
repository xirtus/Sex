use crate::serial_println;
use crate::capability::{NodeCapData, CapabilityData};
use crate::ipc::DOMAIN_REGISTRY;
use alloc::collections::BTreeMap;
use alloc::string::String;
use spin::RwLock;
use lazy_static::lazy_static;

lazy_static! {
    /// Global registry of mount points.
    /// Maps a path (e.g., "/disk0") to a Driver's Protection Domain ID.
    static ref MOUNT_POINTS: RwLock<BTreeMap<String, u32>> = RwLock::new(BTreeMap::new());
}

/// The VFS Server's entry point for PDX calls.
pub extern "C" fn vfs_entry(arg: u64) -> u64 {
    // For simplicity, we'll use a request structure passed by pointer or 
    // split the 64-bit arg into (RequestType, Data).
    
    let req_type = (arg >> 32) as u32;
    let req_data = (arg & 0xFFFF_FFFF) as u32;

    match req_type {
        1 => { // OPEN
            serial_println!("VFS: Open request for node ID: {}", req_data);
            0
        },
        2 => { // MOUNT
            serial_println!("VFS: Mount request for driver PD: {}", req_data);
            0
        },
        3 => { // MOUNT BTRFS
            serial_println!("VFS: Mounting Btrfs volume on driver PD: {}", req_data);
            0
        },
        4 => { // MOUNT NTFS
            serial_println!("VFS: Mounting NTFS volume on driver PD: {}", req_data);
            0
        },
        _ => {
            serial_println!("VFS: Unknown request type: {}", req_type);
            u64::MAX
        }
    }
}

pub fn mount(path: &str, driver_pd_id: u32, fs_type: &str) {
    let mut mounts = MOUNT_POINTS.write();
    mounts.insert(String::from(path), driver_pd_id);
    serial_println!("VFS: Mounted {} ({}) to driver PD {}", path, fs_type, driver_pd_id);
}

/// Simulates opening a file.
/// In a real implementation, this would involve path resolution and 
/// interaction with the storage driver to get a node ID.
pub fn open(caller_pd_id: u32, _path: &str) -> Result<u32, &'static str> {
    // 1. Resolve path to driver PD
    // For this demo, let's assume everything is on driver 10.
    let driver_pd_id = 10;
    let inode_id = 42; // Simulated inode

    // 2. Create a Node Capability
    let node_cap = CapabilityData::Node(NodeCapData {
        node_id: 1, // Assume local node is 1
        driver_pd_id,
        inode_id,
        permissions: 0x7, // R/W/X
    });

    // 3. Grant the capability to the caller
    let registry = DOMAIN_REGISTRY.read();
    let caller_pd = registry.get(&caller_pd_id)
        .ok_or("VFS: Caller PD not found")?;

    Ok(caller_pd.grant(node_cap))
}
