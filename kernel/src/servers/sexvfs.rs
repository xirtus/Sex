use crate::serial_println;
use crate::capability::{NodeCapData, CapabilityData};
use crate::ipc::DOMAIN_REGISTRY;
use alloc::collections::BTreeMap;
use alloc::string::String;
use spin::RwLock;
use lazy_static::lazy_static;

/// Represents a Virtual Inode (Vnode) in the SexVFS.
#[derive(Debug, Clone, Copy)]
pub struct Vnode {
    pub inode_id: u64,
    pub sexdrive_pd_id: u32,
    pub kind: VnodeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VnodeKind {
    File,
    Directory,
    Translator(u32), // Attached PD ID
}
use core::sync::atomic::AtomicPtr;
use core::ptr;

/// A lock-free Radix Tree node for the Directory Cache (Dcache).
/// Uses RCU-style atomic pointer swaps for high-performance concurrent lookups.
pub struct AtomicRadixNode {
    pub vnode: AtomicPtr<Vnode>,
    /// Pointer to an immutable BTreeMap of children.
    /// In a production SASOS, this would be a dedicated lock-free trie.
    pub children: AtomicPtr<BTreeMap<String, Arc<AtomicRadixNode>>>,
}

impl AtomicRadixNode {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            vnode: AtomicPtr::new(ptr::null_mut()),
            children: AtomicPtr::new(Box::into_raw(Box::new(BTreeMap::new()))),
        })
    }

    /// Looks up a path part in the radix tree (Lock-Free).
    pub fn lookup(&self, part: &str) -> Option<Arc<AtomicRadixNode>> {
        let map_ptr = self.children.load(Ordering::Acquire);
        let map = unsafe { &*map_ptr };
        map.get(part).cloned()
    }

    /// Inserts a child node using RCU (Atomic Swap).
    pub fn insert_child(&self, part: String, child: Arc<AtomicRadixNode>) {
        loop {
            let old_map_ptr = self.children.load(Ordering::Acquire);
            let old_map = unsafe { &*old_map_ptr };

            let mut new_map = old_map.clone();
            new_map.insert(part.clone(), child.clone());

            let new_map_ptr = Box::into_raw(Box::new(new_map));

            if self.children.compare_exchange(old_map_ptr, new_map_ptr, Ordering::SeqCst, Ordering::Relaxed).is_ok() {
                // Success! In a real system, we'd defer the deletion of old_map_ptr via Epochs.
                break;
            } else {
                // Retry if someone else swapped the map
                unsafe { let _ = Box::from_raw(new_map_ptr); }
            }
        }
    }
}

lazy_static! {
    /// The high-performance Atomic Radix-based Directory Cache.
    pub static ref DCACHE_RADIX: Arc<AtomicRadixNode> = AtomicRadixNode::new();
}

/// Resolves a path using the lock-free Radix cache.
pub fn resolve_path(path: &str) -> Result<Vnode, &'static str> {
    let mut current = DCACHE_RADIX.clone();

    for part in path.split('/').filter(|s| !s.is_empty()) {
        if let Some(next) = current.lookup(part) {
            current = next;
        } else {
            return Err("VFS: Path component not found");
        }
    }

    let vnode_ptr = current.vnode.load(Ordering::Acquire);
    if vnode_ptr.is_null() {
        Err("VFS: Path does not resolve to a Vnode")
    } else {
        unsafe { Ok(*vnode_ptr) }
    }
}

    /// The Mount Table.
    static ref MOUNTS: RwLock<BTreeMap<String, u32>> = RwLock::new(BTreeMap::new());

    /// Hurd-style Translator Registry.
    static ref TRANSLATORS: RwLock<BTreeMap<String, u32>> = RwLock::new(BTreeMap::new());
}

pub fn init_mounts() {
    serial_println!("sexvfs: Initializing standard mount points...");
    
    // 1. Root RAM Disk (Internal logic)
    let mut mounts = MOUNTS.write();
    mounts.insert(String::from("/"), 1);

    // 2. Register Advanced Translators (Vampired from Linux)
    let mut translators = TRANSLATORS.write();
    translators.insert(String::from("/ext4"), 400); 
    translators.insert(String::from("/btrfs"), 500); 
    
    serial_println!("sexvfs: Ext4 and Btrfs translators registered.");
}

/// srv_vfs: Virtual File System Federation Layer.
/// Provides the system-wide capability endpoint for file operations.
pub struct VfsFederation {
    pub local_node_id: u32,
}

impl VfsFederation {
    pub fn new(node_id: u32) -> Self {
        Self { local_node_id: node_id }
    }

    /// Resolves a path that might span physical nodes.
    pub fn resolve_path_distributed(&self, path: &str) -> Result<u32, &'static str> {
        if path.starts_with("/remote/") {
            // 1. Extract Node ID from path: /remote/node2/disk0/...
            let node_id = 2; // Simplified parsing
            
            // 2. Perform a PDX call to sexnode to initiate an Export/Import
            // This satisfies Zero-Mediation: VFS doesn't talk to the network,
            // it only requests a capability proxy from sexnode.
            let proxy_id = crate::servers::sexnode::import_remote_proxy(node_id, path)?;
            
            return Ok(proxy_id);
        }
        
        // Fallback to local Radix-Tree resolution
        match resolve_path(path) {
            Ok(vnode) => {
                // Return a local Node capability
                let node_cap = CapabilityData::Node(NodeCapData {
                    node_id: 1,
                    sexdrive_pd_id: vnode.sexdrive_pd_id,
                    inode_id: vnode.inode_id,
                    permissions: 0x7,
                });
                
                let registry = crate::ipc::DOMAIN_REGISTRY.read();
                let current_pd = registry.get(&crate::core_local::CoreLocal::get().current_pd())
                    .ok_or("sexvfs: Identity lost")?;
                Ok(current_pd.grant(node_cap))
            },
            Err(e) => Err(e),
        }
    }
}

/// The Unified Filesystem Server Interface.
/// Every filesystem driver (FAT, Ext4, Btrfs) must implement this PDX interface.
pub enum FsCommand {
    Lookup { dir_inode: u64, name: String },
    Read { inode: u64, offset: u64, size: u64, buffer: u64 },
    GetAttr { inode: u64 },
    ReadDir { inode: u64, cookie: u64 },
}

#[repr(C)]
pub struct FsLookupArgs {
    pub dir_inode: u64,
    pub name: [u8; 128],
}

/// Resolves a full path to a Vnode by walking the directory tree.
/// This function now supports multiple filesystems by dispatching to PDX servers.
pub fn resolve_path_multi_fs(path: &str) -> Result<Vnode, &'static str> {
    serial_println!("sexvfs: Multi-FS Resolve: {}", path);

    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Ok(Vnode { inode_id: 2, sexdrive_pd_id: 1, kind: VnodeKind::Directory });
    }

    // 1. Identify the filesystem (Mount point)
    let mount_pd_id = if path.starts_with("/ext4") { 400 }
    else if path.starts_with("/btrfs") { 500 }
    else if path.starts_with("/disk0") { 800 }
    else { 1 };

    // 2. Perform component-by-component walk via PDX calls to the FS server
    let mut current_inode = 2; // Root inode
    
    // We need the VFS PD to have an IPC capability to the target FS PD.
    // For this prototype, we'll assume a "System-Internal" PDX mechanism
    // that uses PD IDs directly for trusted service coordination.
    let registry = crate::ipc::DOMAIN_REGISTRY.read();
    let fs_pd = registry.get(&mount_pd_id).ok_or("sexvfs: FS Driver PD not found")?;

    for part in parts {
        serial_println!("sexvfs: Requesting lookup for '{}' from PD {}", part, mount_pd_id);
        
        // Prepare arguments in SAS (Single Address Space)
        let mut args = FsLookupArgs {
            dir_inode: current_inode,
            name: [0; 128],
        };
        let bytes = part.as_bytes();
        let len = bytes.len().min(127);
        args.name[..len].copy_from_slice(&bytes[..len]);

        // Perform the call
        // In a real system, the VFS would use a previously granted IPC cap.
        // For the prototype, we simulate the FS driver's response.
        let result = if mount_pd_id == 1 {
            // Mock local FS
            match part {
                "bin" => 100,
                "usr" => 200,
                "init" => 0x1000,
                _ => current_inode + 1,
            }
        } else {
            // Simulate safe_pdx_call to the external driver
            // result = crate::ipc::safe_pdx_call(vfs_pd, fs_cap_id, &args as *const _ as u64)?;
            current_inode + 100 // Simulated remote inode offset
        };
        
        current_inode = result;
    }

    Ok(Vnode {
        inode_id: current_inode,
        sexdrive_pd_id: mount_pd_id,
        kind: VnodeKind::File,
    })
}

/// The sexvfs Server's entry point for PDX calls.
pub extern "C" fn sexvfs_entry(arg: u64) -> u64 {
...

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
        5 => { // SET TRANSLATOR
            serial_println!("sexvfs: Request to set translator for PD: {}", req_data);
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

/// Real path resolution and inode-to-capability conversion.
pub fn open(caller_pd_id: u32, path: &str) -> Result<u32, &'static str> {
    // 1. Resolve path to a Vnode (Lock-Free CSPR walk)
    let vnode = resolve_path(path)?;

    // 2. Handle Translators (Hurd-style)
    if let VnodeKind::Translator(pd_id) = vnode.kind {
        serial_println!("sexvfs: Path {} matches translator PD {}. Handing off...", path, pd_id);
        return Ok(0x_TR_A_NS); 
    }

    // 3. Coordinate with the Storage Server for real LBA resolution
    // For the prototype, we assume the first Storage PD (ID 800) handles all requests.
    let lba = match path {
        "/disk0/init" => 0x1000, // Hardcoded LBA for the init process for now
        "/disk0/config.json" => 0x2000,
        _ => vnode.inode_id, // Fallback to simulated inode
    };

    serial_println!("sexvfs: Path {} resolved to LBA {} on Storage PD {}.", 
        path, lba, vnode.sexdrive_pd_id);

    // 4. Create a Node Capability for the resolved Vnode
    let node_cap = crate::capability::CapabilityData::Node(crate::capability::NodeCapData {
        node_id: 1, // Local node
        sexdrive_pd_id: vnode.sexdrive_pd_id,
        inode_id: lba, // Store the real LBA in the inode field
        permissions: 0x7, // R/W/X
    });

    // 5. Grant the capability to the caller
    let registry = DOMAIN_REGISTRY.read();
    let caller_pd = registry.get(&caller_pd_id)
        .ok_or("sexvfs: Caller PD not found")?;

    Ok(caller_pd.grant(node_cap))
}
