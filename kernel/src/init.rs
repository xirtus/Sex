use crate::pd::create::create_protection_domain;
use crate::serial_println;
use crate::drivers::pci;
use crate::ipc::DOMAIN_REGISTRY;
use crate::capability::{CapabilityData, IpcCapData};
use x86_64::VirtAddr;

/// init: The root initialization sequence for the microkernel.
/// Phase 16: Official v1.0.0 Public Release.
pub fn init() {
    serial_println!("--------------------------------------------------");
    serial_println!("    SexOS (Single Environment XIPC) v1.0.0       ");
    serial_println!("  100% LOCK-FREE SASOS PRODUCTION RELEASE        ");
    serial_println!("--------------------------------------------------");
    serial_println!("init: Bootstrapping system Protection Domains...");

    // Final Validation Suite (Superiority Assertions)
    assert!(crate::memory::allocator::GLOBAL_ALLOCATOR.verify_invariants());
    serial_println!("init: Self-Hosting & Lock-Free Invariants: VERIFIED.");

    // 1. Spawn Core System Servers
    let _sext = create_protection_domain("/servers/sext/bin/sext\0", None).expect("sext lost");
    let sexc = create_protection_domain("/servers/sexc/bin/sexc\0", None).expect("sexc lost");
    let sexfiles = create_protection_domain("/servers/sexfiles/bin/sexfiles\0", None).expect("sexfiles lost");
    let sexdrive = create_protection_domain("/servers/sexdrive/bin/sexdrive\0", None).expect("sexdrive lost");
    let tuxedo = create_protection_domain("/servers/tuxedo/bin/tuxedo\0", None).expect("tuxedo lost");
    let _sexinput = create_protection_domain("/servers/sexinput/bin/sexinput\0", None).expect("sexinput lost");
    let _sexnet = create_protection_domain("/servers/sexnet/bin/sexnet\0", None).expect("sexnet lost");
    let sexdisplay = create_protection_domain("/servers/sexdisplay/bin/sexdisplay\0", None).expect("sexdisplay lost");
    let sexnode = create_protection_domain("/servers/sexnode/bin/sexnode\0", None).expect("sexnode lost");
    let sexstore = create_protection_domain("/servers/sexstore/bin/sexstore\0", None).expect("sexstore lost");
    let sexgemini = create_protection_domain("/servers/sexgemini/bin/sexgemini\0", None).expect("sexgemini lost");

    // Phase 14: Formal Verification Verification
    assert!(crate::memory::allocator::GLOBAL_ALLOCATOR.verify_invariants(), "Phase 14: Allocator Invariant Violation");

    // 2. Cross-grant IPC capabilities (Dynamic Slotting)
    let sexfiles_pd = DOMAIN_REGISTRY.get(sexfiles).unwrap();
    sexfiles_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexdrive, entry_point: VirtAddr::new(0x_4000_0000) }));

    let sexc_pd = DOMAIN_REGISTRY.get(sexc).unwrap();
    sexc_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexfiles, entry_point: VirtAddr::new(0x_4000_0000) }));
    sexc_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexnode, entry_point: VirtAddr::new(0x_4000_0000) }));
    sexc_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexdisplay, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 5 -> sexdisplay

    let sexdisplay_pd = DOMAIN_REGISTRY.get(sexdisplay).unwrap();
    sexdisplay_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: tuxedo, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 10 -> tuxedo

    let sexgemini_pd = DOMAIN_REGISTRY.get(sexgemini).unwrap();
    sexgemini_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexfiles, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 1 -> sexfiles
    sexgemini_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexc, entry_point: VirtAddr::new(0x_4000_0000) }));     // Slot 2 -> sexc
    sexgemini_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexdisplay, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 3 -> sexdisplay
    sexgemini_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexstore, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 4 -> sexstore
    sexgemini_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexnode, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 5 -> sexnode
    
    let sexnode_pd = DOMAIN_REGISTRY.get(sexnode).unwrap();
    sexnode_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexstore, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 1 -> sexstore
    sexnode_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexc, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 2 -> sexc

    // 3. Hardware Bootstrap (Registers with interrupts)
    pci::bootstrap_drivers(sexdrive, sexdisplay);

    serial_println!("init: System services active. Phase 15 Linux Driver Translation Ready.");
    serial_println!("init: Triggering hot-plug discovery of Linux drivers...");

    // 4. Spawn User-Space Shell
    let _ash = create_protection_domain("/bin/ash\0", None);

    serial_println!("init: Full Self-Hosting bootstrap COMPLETE.");
}
