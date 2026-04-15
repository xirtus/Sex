use crate::capability::{ProtectionDomain, CapabilityData, NodeCapData};
use crate::serial_println;

pub struct CapEngine;

impl CapEngine {
    /// Mints and grants the initial set of capabilities required for a new PD.
    /// IPCtax: Uses RCU-based CapabilityTable (Lock-Free).
    pub fn grant_initial_rights(pd: &ProtectionDomain) {
        serial_println!("cap: Granting root rights to PD {}...", pd.id);

        // 1. Root VFS capability
        pd.grant(CapabilityData::Node(NodeCapData {
            node_id: 1,
            sexdrive_pd_id: 100, // sexvfs
            inode_id: 2,
            permissions: 0x7, // R/W/X
        }));

        // 2. Control Ring Capability (Self-management)
        pd.grant(CapabilityData::Domain(pd.id));
    }

    pub fn verify_signal_rights(pd: &ProtectionDomain, cap_id: u64) -> bool {
        // RCU Lookup: Wait-Free
        pd.cap_table.find(cap_id as u32).is_some()
    }
}
