use crate::pd::create::create_protection_domain;
use crate::serial_println;
use crate::drivers::pci;

/// init: The root initialization sequence for the microkernel.
/// Phase 13: Final Native Self-Hosting and Store bootstrap.
pub fn init() {
    serial_println!("init: Bootstrapping system Protection Domains...");

    // 1. Spawn Core System Servers
    create_protection_domain("/servers/sext/bin/sext\0", Some(2)).expect("sext lost");
    create_protection_domain("/servers/sexc/bin/sexc\0", Some(3)).expect("sexc lost");
    create_protection_domain("/servers/sexvfs/bin/sexvfs\0", Some(100)).expect("sexvfs lost");
    create_protection_domain("/servers/sexdrives/bin/sexdrives\0", Some(200)).expect("sexdrives lost");
    create_protection_domain("/servers/sexinput/bin/sexinput\0", Some(300)).expect("sexinput lost");
    create_protection_domain("/servers/sexnet/bin/sexnet\0", Some(400)).expect("sexnet lost");
    create_protection_domain("/servers/sexdisplay/bin/sexdisplay\0", Some(500)).expect("sexdisplay lost");
    create_protection_domain("/servers/sexnode/bin/sexnode\0", Some(600)).expect("sexnode lost");

    // Phase 13: sexstore (Package Manager & Self-Host Daemon)
    create_protection_domain("/servers/sexstore/bin/sexstore\0", Some(700)).expect("sexstore lost");

    // 2. Hardware Bootstrap
    pci::bootstrap_drivers();

    serial_println!("init: System services active. Spawning Self-Host environment...");

    // 3. Spawn User-Space Pipeline
    let _ash = create_protection_domain("/bin/ash\0", None);
    let _gemini = create_protection_domain("/bin/sex-gemini\0", None); // Self-repair agent

    serial_println!("init: Full Self-Hosting bootstrap COMPLETE.");
}
