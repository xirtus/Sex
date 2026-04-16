#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(sex_kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use sex_kernel::serial_println;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_main();
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("TEST FAILED: {}", info);
    sex_kernel::exit_qemu(sex_kernel::QemuExitCode::Failed);
    loop {}
}

#[test_case]
fn test_elf_pd_spawn_lockfree() {
    serial_println!("test: Attempting to spawn ash PD (Lock-Free Path)...");
    
    // 1. Setup mock environment (Ensures sexvfs is 'present' in registry)
    // In our SAS bootstrap, we already spawned sexvfs at PD 100.
    
    // 2. Call sys_spawn_pd
    let path = "/bin/ash\0";
    let res = sex_kernel::syscalls::spawn::sys_spawn_pd(path.as_ptr());
    
    assert!(res >= 4000, "Spawn failed: expected PD ID >= 4000, got {}", res);
    
    // 3. Verify PD presence in RCU Registry
    let pd = sex_kernel::ipc::DOMAIN_REGISTRY.get(res as u32).expect("PD not in registry");
    assert_eq!(pd.id, res as u32);
    
    // 4. Check initial capabilities
    let vfs_cap = unsafe { (*pd.cap_table).find(1).expect("VFS capability missing") };
    match vfs_cap.data {
        sex_kernel::capability::CapabilityData::Node(_) => (),
        _ => panic!("Expected Node capability at slot 1"),
    }

    serial_println!("test: PD Spawn SUCCESS (PD ID {}).", res);
}
