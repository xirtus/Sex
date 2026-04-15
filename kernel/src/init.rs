use crate::pd::create::create_protection_domain;
use crate::serial_println;
use crate::drivers::pci;
use crate::ipc::DOMAIN_REGISTRY;
use crate::capability::{CapabilityData, IpcCapData};
use x86_64::VirtAddr;

/// init: The root initialization sequence for the microkernel.
/// Phase 13.2.1: Final hardware polish and capability-based routing bootstrap.
pub fn init() {
    serial_println!("init: Bootstrapping system Protection Domains...");

    // 1. Spawn Core System Servers (Dynamic IDs)
    let sext = create_protection_domain("/servers/sext/bin/sext\0", None).expect("sext lost");
    let sexc = create_protection_domain("/servers/sexc/bin/sexc\0", None).expect("sexc lost");
    let sexvfs = create_protection_domain("/servers/sexvfs/bin/sexvfs\0", None).expect("sexvfs lost");
    let sexdrives = create_protection_domain("/servers/sexdrives/bin/sexdrives\0", None).expect("sexdrives lost");
    let sexinput = create_protection_domain("/servers/sexinput/bin/sexinput\0", None).expect("sexinput lost");
    let sexnet = create_protection_domain("/servers/sexnet/bin/sexnet\0", None).expect("sexnet lost");
    let sexdisplay = create_protection_domain("/servers/sexdisplay/bin/sexdisplay\0", None).expect("sexdisplay lost");
    let sexnode = create_protection_domain("/servers/sexnode/bin/sexnode\0", None).expect("sexnode lost");
    let sexstore = create_protection_domain("/servers/sexstore/bin/sexstore\0", None).expect("sexstore lost");
    let sexgemini = create_protection_domain("/servers/sexgemini/bin/sexgemini\0", None).expect("sexgemini lost");

    // Phase 13.2.1: Grant sext to root PD (Slot 2) for #PF routing
    let root_pd = DOMAIN_REGISTRY.get(0).unwrap();
    root_pd.grant(CapabilityData::IPC(IpcCapData { 
        node_id: 1, target_pd_id: sext, entry_point: VirtAddr::new(0x_4000_0000) 
    }));

    // 2. Cross-grant IPC capabilities (Standardize slots)
    // Grant sexvfs to sexc (Slot 1)
    let sexc_pd = DOMAIN_REGISTRY.get(sexc).unwrap();
    sexc_pd.grant(CapabilityData::IPC(IpcCapData { 
        node_id: 1, target_pd_id: sexvfs, entry_point: VirtAddr::new(0x_4000_0000) 
    }));
    
    // Grant sexnode to sexc (Slot 2)
    sexc_pd.grant(CapabilityData::IPC(IpcCapData { 
        node_id: 1, target_pd_id: sexnode, entry_point: VirtAddr::new(0x_4000_0000) 
    }));

    // Grant sexnet to sexstore (Slot 4)
    let sexstore_pd = DOMAIN_REGISTRY.get(sexstore).unwrap();
    sexstore_pd.grant(CapabilityData::IPC(IpcCapData { 
        node_id: 1, target_pd_id: sexnet, entry_point: VirtAddr::new(0x_4000_0000) 
    }));

    // Grant sexdisplay to sexinput (Slot 5)
    let sexinput_pd = DOMAIN_REGISTRY.get(sexinput).unwrap();
    sexinput_pd.grant(CapabilityData::IPC(IpcCapData { 
        node_id: 1, target_pd_id: sexdisplay, entry_point: VirtAddr::new(0x_4000_0000) 
    }));
    
    // Grant sexstore, sexc, and sexnode to sexgemini for self-repair (Slots 4, 2, 5)
    let sexgemini_pd = DOMAIN_REGISTRY.get(sexgemini).unwrap();
    sexgemini_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexstore, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 1 -> 4
    sexgemini_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexc, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 2 -> 2
    sexgemini_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexnode, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 3 -> 5
    
    // Wire Capability Violation fault interrupt (0x8E) to sexgemini
    crate::interrupts::register_irq_route(0x8E, sexgemini);

    // Pass PDs down to PCI for hardware grants
    pci::bootstrap_drivers(sexdrives, sexdisplay);

    serial_println!("init: System services active. Spawning Self-Host environment...");

    // 4. Spawn User-Space Pipeline
    let _ash = create_protection_domain("/bin/ash\0", None);

    serial_println!("init: Full Self-Hosting bootstrap COMPLETE.");
}
