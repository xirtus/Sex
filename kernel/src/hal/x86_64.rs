use crate::hal::{HardwareAbstractionLayer, pci::PciDevice};
use alloc::vec::Vec;
use crate::serial_println;
use crate::gdt;
use crate::interrupts;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use x86_64::registers::control::Cr3;

pub struct X86Hal {
    paging_ready: AtomicBool,
    paging_cr3: AtomicU64,
}

impl X86Hal {
    pub const fn new() -> Self {
        Self {
            paging_ready: AtomicBool::new(false),
            paging_cr3: AtomicU64::new(0),
        }
    }

    pub fn is_paging_ready(&self) -> bool {
        self.paging_ready.load(Ordering::Acquire)
    }

    pub fn get_paging_cr3(&self) -> u64 {
        self.paging_cr3.load(Ordering::Acquire)
    }

    pub fn tlb_flush_local(&self) {
        unsafe {
            let (p4_frame, flags) = Cr3::read();
            Cr3::write(p4_frame, flags);
        }
    }
}

impl HardwareAbstractionLayer for X86Hal {
    fn init(&self) {
        if !self.is_paging_ready() {
            serial_println!("X86Hal: Initializing foundation (BSP)...");
            
            // SAS Page Table Sync
            let (p4_frame, _) = Cr3::read();
            let cr3_val = p4_frame.start_address().as_u64();
            self.paging_cr3.store(cr3_val, Ordering::Release);
            self.paging_ready.store(true, Ordering::Release);
            
            serial_println!("X86Hal: SAS Page Tables ready (CR3 = {:#x})", cr3_val);
        } else {
            serial_println!("X86Hal: Initializing foundation (AP)...");
        }

        if crate::pku::is_pku_supported() {
            unsafe { crate::pku::enable_pku(); }
        }

        serial_println!("X86Hal: Initializing GDT/IDT...");
        gdt::init();
        interrupts::init_idt();
    }

    fn enumerate_pci(&self) -> Vec<PciDevice> {
        crate::hal::pci::enumerate_bus()
    }

    fn setup_timer(&self, _hz: u64) {
        // TODO: LAPIC Timer or PIT
        serial_println!("X86Hal: Timer setup pending...");
    }

    fn configure_interrupts(&self) {
        serial_println!("X86Hal: Interrupt configuration pending...");
    }
}
