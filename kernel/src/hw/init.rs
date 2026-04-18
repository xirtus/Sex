// kernel/src/hw/init.rs
use acpi::{Handler, AcpiTables, PhysicalMapping};
use core::ptr::NonNull;
use x86_64::PhysAddr;

#[derive(Clone)]
pub struct SexAcpiHandler {
    hhdm_offset: u64,
}

impl SexAcpiHandler {
    pub fn new(hhdm_offset: u64) -> Self {
        Self { hhdm_offset }
    }
}

impl Handler for SexAcpiHandler {
    unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> PhysicalMapping<Self, T> {
        let virtual_address = physical_address as u64 + self.hhdm_offset;
        PhysicalMapping {
            physical_start: physical_address,
            virtual_start: NonNull::new(virtual_address as *mut T).unwrap(),
            region_length: size,
            mapped_length: size,
            handler: self.clone(),
        }
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {
        // SAS: Mapping is persistent in higher half, no explicit unmap needed for boot
    }

    fn read_u8(&self, _: usize) -> u8 { todo!() }
    fn read_u16(&self, _: usize) -> u16 { todo!() }
    fn read_u32(&self, _: usize) -> u32 { todo!() }
    fn read_u64(&self, _: usize) -> u64 { todo!() }
    fn write_u8(&self, _: usize, _: u8) { todo!() }
    fn write_u16(&self, _: usize, _: u16) { todo!() }
    fn write_u32(&self, _: usize, _: u32) { todo!() }
    fn write_u64(&self, _: usize, _: u64) { todo!() }
    fn read_io_u8(&self, _: u16) -> u8 { todo!() }
    fn read_io_u16(&self, _: u16) -> u16 { todo!() }
    fn read_io_u32(&self, _: u16) -> u32 { todo!() }
    fn write_io_u8(&self, _: u16, _: u8) { todo!() }
    fn write_io_u16(&self, _: u16, _: u16) { todo!() }
    fn write_io_u32(&self, _: u16, _: u32) { todo!() }
    fn read_pci_u8(&self, _: acpi::PciAddress, _: u16) -> u8 { todo!() }
    fn read_pci_u16(&self, _: acpi::PciAddress, _: u16) -> u16 { todo!() }
    fn read_pci_u32(&self, _: acpi::PciAddress, _: u16) -> u32 { todo!() }
    fn write_pci_u8(&self, _: acpi::PciAddress, _: u16, _: u8) { todo!() }
    fn write_pci_u16(&self, _: acpi::PciAddress, _: u16, _: u16) { todo!() }
    fn write_pci_u32(&self, _: acpi::PciAddress, _: u16, _: u32) { todo!() }
    fn nanos_since_boot(&self) -> u64 { todo!() }
    fn stall(&self, _: u64) { todo!() }
    fn sleep(&self, _: u64) { todo!() }
    fn create_mutex(&self) -> acpi::Handle { todo!() }
    fn acquire(&self, _: acpi::Handle, _: u16) -> Result<(), acpi::aml::AmlError> { todo!() }
    fn release(&self, _: acpi::Handle) { todo!() }
}

pub fn init(rsdp_addr: u64, hhdm_offset: u64) {
    let handler = SexAcpiHandler::new(hhdm_offset);
    let tables = unsafe { AcpiTables::from_rsdp(handler.clone(), rsdp_addr as usize).expect("ACPI parse failed") };

    // 1. Discover MADT for APIC config
    let platform = acpi::platform::AcpiPlatform::new(tables, handler).expect("Platform info failed");
    crate::serial_println!("Sex: ACPI platform info discovered.");

    // 2. Discover PCIe Configuration Space (MCFG)
    if let Some(mcfg) = platform.tables.find_table::<acpi::sdt::mcfg::Mcfg>() {
        crate::serial_println!("Sex: PCIe MCFG found.");
        // TODO: Pass to PCI bus enumerator
    }
}
