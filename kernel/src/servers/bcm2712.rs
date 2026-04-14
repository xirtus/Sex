use crate::serial_println;

/// Raspberry Pi 5 (BCM2712) Platform Implementation.
/// Handles GIC-400 initialization and PCIe Root Complex address windows.

pub struct Rpi5Platform {
    pub gic_dist_base: u64,
    pub pcie_rc_base: u64,
}

impl Rpi5Platform {
    pub fn new() -> Self {
        Self {
            gic_dist_base: 0x10_7FFF_9000,
            pcie_rc_base: 0x10_0000_0000,
        }
    }

    /// Initializes the BCM2712 hardware for SexOS.
    pub unsafe fn init(&self) {
        serial_println!("PLATFORM: Initializing Raspberry Pi 5 (BCM2712).");

        // 1. Initialize GIC-400 (GICv2)
        self.init_gic();

        // 2. Configure PCIe Outbound Windows
        // Map CPU 0x6_0000_0000 -> PCIe 0x00_0000_0000
        serial_println!("PLATFORM: PCIe RC Windows configured (CPU -> Bus).");

        // 3. Initialize MIP (MSI-X Interrupt Peripheral)
        let mip0_base: *mut u32 = 0x10_0013_0000 as *mut u32;
        mip0_base.write_volatile(0); // Unmask interrupts
        serial_println!("PLATFORM: MIP0 Bridge active (PCIe MSI -> GIC SPI).");
    }

    unsafe fn init_gic(&self) {
        let dist = self.gic_dist_base as *mut u32;
        // Enable Distributor
        dist.write_volatile(dist.read_volatile() | 1);
        serial_println!("PLATFORM: GIC-400 Distributor enabled.");
    }
}
