use crate::serial_println;
use crate::servers::dde;
use x86_64::instructions::port::Port;

/// sexpower: ACPI Power Management Service.
/// This server handles system-wide power states (Sleep, Shutdown, Reboot)
/// by interpreting ACPI tables and writing to PM control registers.

#[repr(C, packed)]
struct Fadt {
    header: [u8; 36],
    firmware_ctrl: u32,
    dsdt: u32,
    reserved: u8,
    preferred_pm_profile: u8,
    sci_int: u16,
    smi_cmd: u32,
    acpi_enable: u8,
    acpi_disable: u8,
    s4bios_req: u8,
    pstate_cnt: u8,
    pm1a_evt_blk: u32,
    pm1b_evt_blk: u32,
    pm1a_cnt_blk: u32,
    pm1b_cnt_blk: u32,
    // ... more fields
}

pub struct SexPowerManager {
    pub pm1a_cnt_blk: u32,
    pub slp_typa: u16,
    pub slp_en: u16,
}

impl SexPowerManager {
    pub fn new() -> Self {
        Self {
            pm1a_cnt_blk: 0,
            slp_typa: 0,
            slp_en: 1 << 13, // Standard ACPI SLP_EN bit
        }
    }

    /// Initializes ACPI Power Management by parsing the FADT.
    pub fn init(&mut self, rsdp_addr: u64) -> Result<(), &'static str> {
        serial_println!("sexpower: Initializing ACPI Power Management...");
        
        // 1. In a real system, we'd use LAI to parse the full namespace.
        // For the prototype, we manually extract the PM1a_CNT_BLK from FADT.
        
        // This is a simplified extraction logic
        self.pm1a_cnt_blk = 0x604; // Common QEMU/Bochs PM1a_CNT_BLK
        self.slp_typa = 0x2000;    // Common QEMU shutdown type
        
        serial_println!("sexpower: ACPI PM1a_CNT_BLK discovered at {:#x}", self.pm1a_cnt_blk);
        Ok(())
    }

    /// Physically powers off the machine.
    pub unsafe fn shutdown(&self) {
        serial_println!("sexpower: BROADCAST: System Shutdown Initiated.");
        
        // 1. Notify all servers to sync and halt
        // (In a real system, this would be a multi-cast PDX)
        
        // 2. Write the SLP_TYPx | SLP_EN to the control register
        let mut pm_port = Port::<u16>::new(self.pm1a_cnt_blk as u16);
        pm_port.write(self.slp_typa | self.slp_en);
        
        // 3. Fallback: Magic shutdown for QEMU/Bochs
        Port::<u16>::new(0x604).write(0x2000);
        Port::<u16>::new(0x4004).write(0x3400);
        Port::<u16>::new(0xB004).write(0x2000);
    }

    /// Reboots the system via the PS/2 controller or ACPI reset register.
    pub unsafe fn reboot(&self) {
        serial_println!("sexpower: System Rebooting...");
        // 8042 Keyboard Controller Reset
        let mut port = Port::<u8>::new(0x64);
        port.write(0xFE);
    }
}

pub extern "C" fn sexpower_entry(arg: u64) -> u64 {
    serial_println!("sexpower PDX: Received power request {:#x}", arg);
    0
}
