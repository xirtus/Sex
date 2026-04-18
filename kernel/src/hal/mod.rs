use alloc::vec::Vec;
use crate::hal::pci::PciDevice;

pub mod pci;
pub mod x86_64;
pub mod acpi;

pub trait HardwareAbstractionLayer {
    fn init(&self);
    fn enumerate_pci(&self) -> Vec<PciDevice>;
    fn setup_timer(&self, hz: u64);
    fn configure_interrupts(&self);
}

pub fn init() {
    use crate::hal::HardwareAbstractionLayer;
    HAL.init();
}

pub fn tlb_flush_local() {
    HAL.tlb_flush_local();
}

// Global HAL instance for the kernel.
pub static HAL: x86_64::X86Hal = x86_64::X86Hal::new();
