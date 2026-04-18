use crate::serial_println;

pub fn init() {
    serial_println!("HAL: ACPI discovery stub... parsing MADT/FADT soon.");
}

// In the future, this will return CPU topology and IOAPIC info.
pub struct AcpiInfo {
    pub lapic_addr: u64,
    pub ioapic_addr: u64,
}
