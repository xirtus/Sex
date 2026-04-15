use crate::pd::create::create_protection_domain;
use crate::serial_println;
use crate::drivers::pci;
use crate::ipc::DOMAIN_REGISTRY;
use crate::capability::{CapabilityData, IpcCapData};
use x86_64::VirtAddr;

/// init: The root initialization sequence for the microkernel.
/// Phase 13.1: Final hardware polish and capability-based routing bootstrap.
pub fn init() {
    serial_println!("init: Bootstrapping system Protection Domains...");

    // 1. Spawn Core System Servers with Fixed Slots
    let sext = create_protection_domain("/servers/sext/bin/sext\0", Some(2)).expect("sext lost");
    let sexc = create_protection_domain("/servers/sexc/bin/sexc\0", Some(3)).expect("sexc lost");
    let sexvfs = create_protection_domain("/servers/sexvfs/bin/sexvfs\0", Some(100)).expect("sexvfs lost");
    let sexdrives = create_protection_domain("/servers/sexdrives/bin/sexdrives\0", Some(200)).expect("sexdrives lost");
    let sexinput = create_protection_domain("/servers/sexinput/bin/sexinput\0", Some(300)).expect("sexinput lost");
    let sexnet = create_protection_domain("/servers/sexnet/bin/sexnet\0", Some(400)).expect("sexnet lost");
    let sexdisplay = create_protection_domain("/servers/sexdisplay/bin/sexdisplay\0", Some(500)).expect("sexdisplay lost");
    let sexnode = create_protection_domain("/servers/sexnode/bin/sexnode\0", Some(600)).expect("sexnode lost");
    let sexstore = create_protection_domain("/servers/sexstore/bin/sexstore\0", Some(700)).expect("sexstore lost");

    // 2. Cross-grant IPC capabilities (Standardize slots)
    // Grant sexvfs to sexc (Slot 1)
    let sexc_pd = DOMAIN_REGISTRY.get(3).unwrap();
    sexc_pd.grant(CapabilityData::IPC(IpcCapData { 
        node_id: 1, target_pd_id: 100, entry_point: VirtAddr::new(0x_4000_0000) 
    }));
    
    // Grant sexnode to sexc (Slot 2)
    sexc_pd.grant(CapabilityData::IPC(IpcCapData { 
        node_id: 1, target_pd_id: 600, entry_point: VirtAddr::new(0x_4000_0000) 
    }));

    // Grant sexnet to sexstore (Slot 4)
    let sexstore_pd = DOMAIN_REGISTRY.get(700).unwrap();
    sexstore_pd.grant(CapabilityData::IPC(IpcCapData { 
        node_id: 1, target_pd_id: 400, entry_point: VirtAddr::new(0x_4000_0000) 
    }));

    // 3. Hardware Bootstrap (Registers with interrupts)
    pci::bootstrap_drivers();

    serial_println!("init: System services active. Spawning Self-Host environment...");

    // 4. Spawn User-Space Pipeline
    let _ash = create_protection_domain("/bin/ash\0", None);
    let _gemini = create_protection_domain("/bin/sex-gemini\0", None);

    serial_println!("init: Full Self-Hosting bootstrap COMPLETE.");
}
