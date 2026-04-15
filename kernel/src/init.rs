use crate::pd::create::create_protection_domain;
use crate::serial_println;

/// init: The root initialization sequence for the microkernel.
/// Phase 10/8: Spawns standalone system servers and the root shell.
pub fn init() {
    serial_println!("init: Bootstrapping system Protection Domains...");

    // 1. Spawn System Servers (Standalone ELFs)
    // IPCtax: These domains will communicate purely via PDX.
    let _sext = create_protection_domain("/servers/sext/bin/sext\0");
    let _sexc = create_protection_domain("/servers/sexc/bin/sexc\0");
    let _sexvfs = create_protection_domain("/servers/sexvfs/bin/sexvfs\0");
    let _sexdrives = create_protection_domain("/servers/sexdrives/bin/sexdrives\0");
    let _sexinput = create_protection_domain("/servers/sexinput/bin/sexinput\0");
    let _sexnet = create_protection_domain("/servers/sexnet/bin/sexnet\0");

    serial_println!("init: System services spawned. Spawning root shell...");

    // 2. Spawn Root Shell (ASH)
    let res = create_protection_domain("/bin/ash\0");
    
    match res {
        Ok(pd_id) => {
            serial_println!("init: Root shell spawned with PD ID {}. SYSTEM READY.", pd_id);
        },
        Err(e) => {
            serial_println!("init: Critical failure - could not spawn /bin/ash: {}", e);
        }
    }
}
