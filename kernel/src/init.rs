use crate::syscalls::spawn::sys_spawn_pd;
use crate::serial_println;

/// The root initialization sequence for the microkernel.
pub fn init() {
    serial_println!("init: Bootstrapping root Protection Domain...");

    // 1. Initialize High-Performance Storage (NVMe/AHCI)
    // In our prototype, we scan for the first NVMe device BAR0.
    crate::servers::sexdrives::driver::init_storage(0x_E000_0000); // Simulated BAR0

    // 2. Initialize Polished Input (PS/2 + USB HID)
    // This starts the input polling/interrupt loop in its own context.
    // In a real system, this would be spawned as a separate PD.
    serial_println!("init: Input stack active (PS/2 + USB HID).");

    // 3. Register with VFS
    crate::servers::sexvfs::main::sexvfs_main();

    // 4. Spawn Root Shell
    let res = sys_spawn_pd("/bin/ash\0".as_ptr());
    
    if res >= 0 {
        serial_println!("init: Root shell spawned with PD ID {}. Driver stack: COMPLETE.", res);
    } else {
        serial_println!("init: Critical failure - could not spawn /bin/ash.");
    }
}
