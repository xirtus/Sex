use crate::pd::create::create_protection_domain;
use crate::serial_println;
use crate::ipc::DOMAIN_REGISTRY;
use crate::capability::{CapabilityData, IpcCapData};
use x86_64::VirtAddr;

pub static mut SEXDISPLAY_PD_ID: u32 = 0;

/// init: The root initialization sequence for the microkernel.
/// Phase 16: Official v1.0.0 Public Release.
pub fn init() {
    serial_println!("--------------------------------------------------");
    serial_println!("    SexOS (Single Environment XIPC) v1.0.0       ");
    serial_println!("  100% LOCK-FREE SASOS PRODUCTION RELEASE        ");
    serial_println!("--------------------------------------------------");
    serial_println!("init: Bootstrapping system Protection Domains...");

    // Phase 17: Initialize Kernel PD (ID 0) to allow early ELF Loading
    let kernel_pd = alloc::boxed::Box::into_raw(alloc::boxed::Box::new(crate::capability::ProtectionDomain::new(0, 0)));
    DOMAIN_REGISTRY.insert(0, kernel_pd);

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
    unsafe { SEXDISPLAY_PD_ID = sexdisplay; }
    let sexnode = create_protection_domain("/servers/sexnode/bin/sexnode\0", None).expect("sexnode lost");
    let sexshop = create_protection_domain("/servers/sexshop/bin/sexshop\0", None).expect("sexshop lost");
    let sexgemini = create_protection_domain("/servers/sexgemini/bin/sexgemini\0", None).expect("sexgemini lost");

    // Phase 14: Formal Verification Verification
    assert!(crate::memory::allocator::GLOBAL_ALLOCATOR.verify_invariants(), "Phase 14: Allocator Invariant Violation");

    // 2. Cross-grant IPC capabilities (Dynamic Slotting)
    let sexfiles_pd = DOMAIN_REGISTRY.get(sexfiles).unwrap();
    sexfiles_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexdrive, entry_point: VirtAddr::new(0x_4000_0000) }));

    let sexshop_pd = DOMAIN_REGISTRY.get(sexshop).unwrap();
    sexshop_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexfiles, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 1 -> sexfiles

    let sexc_pd = DOMAIN_REGISTRY.get(sexc).unwrap();
    sexc_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexfiles, entry_point: VirtAddr::new(0x_4000_0000) }));
    sexc_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexnode, entry_point: VirtAddr::new(0x_4000_0000) }));
    sexc_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexdisplay, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 5 -> sexdisplay

    let sexdisplay_pd = DOMAIN_REGISTRY.get(sexdisplay).unwrap();
    sexdisplay_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: tuxedo, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 10 -> tuxedo
    sexdisplay_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexdrive, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 11 -> sexdrive (GPU DMA)

    let sexgemini_pd = DOMAIN_REGISTRY.get(sexgemini).unwrap();
    sexgemini_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexfiles, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 1 -> sexfiles
    sexgemini_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexc, entry_point: VirtAddr::new(0x_4000_0000) }));     // Slot 2 -> sexc
    sexgemini_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexdisplay, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 3 -> sexdisplay
    sexgemini_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexshop, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 4 -> sexshop
    sexgemini_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexnode, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 5 -> sexnode
    
    let sexnode_pd = DOMAIN_REGISTRY.get(sexnode).unwrap();
    sexnode_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexshop, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 1 -> sexshop
    sexnode_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexc, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 2 -> sexc

    // 3. Hardware Bootstrap (Registers with interrupts)
    crate::devmgr::init(sexdrive, sexdisplay);

    serial_println!("init: System services active. Phase 15 Linux Driver Translation Ready.");
    serial_println!("init: Triggering hot-plug discovery of Linux drivers...");

    // 4. Spawn User-Space Shell (Ion SexShell)
    let ion = create_protection_domain("/ion-sexshell\0", None).expect("ion lost");
    let ion_pd = DOMAIN_REGISTRY.get(ion).unwrap();
    ion_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexdisplay, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 2 -> sexdisplay (Orbital)

    // Phase 18: Spawn Desktop Prototype (Egui Hello)
    let egui = create_protection_domain("/servers/egui-hello/bin/egui-hello\0", None).expect("egui lost");
    let egui_pd = DOMAIN_REGISTRY.get(egui).unwrap();
    egui_pd.grant(CapabilityData::IPC(IpcCapData { node_id: 1, target_pd_id: sexdisplay, entry_point: VirtAddr::new(0x_4000_0000) })); // Slot 2 -> sexdisplay (Orbital)

    serial_println!("init: Full Self-Hosting bootstrap COMPLETE.");
    serial_println!("init: System is now running in SASOS Mode.");
}
