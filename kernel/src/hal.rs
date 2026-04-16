use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use x86_64::registers::control::Cr3;
use crate::serial_println;
use crate::gdt;
use crate::interrupts;

/// Global SAS page-table synchronization (lock-free).
/// BSP publishes its CR3 here so all cores can enter the same SAS.
pub static PAGING_CR3: AtomicU64 = AtomicU64::new(0);
pub static PAGING_READY: AtomicBool = AtomicBool::new(false);

pub fn init() {
    serial_println!("hal::init() — setting up SAS page tables (BSP only)");

    // In a real SEX impl, we would ensure PML4 is at a known physical address (e.g. 0x1000).
    // For now, we capture current CR3 (set by early memory init or bootloader).
    let (p4_frame, _) = Cr3::read();
    let cr3_val = p4_frame.start_address().as_u64();
    
    PAGING_CR3.store(cr3_val, Ordering::Release);
    PAGING_READY.store(true, Ordering::Release);

    serial_println!("hal::init() — SAS page tables ready (CR3 = {:#x})", cr3_val);

    serial_println!("HAL: Initializing GDT...");
    gdt::init();
    serial_println!("HAL: Initializing IDT...");
    interrupts::init_idt();
}

pub fn tlb_flush_local() {
    unsafe {
        let (p4_frame, flags) = Cr3::read();
        Cr3::write(p4_frame, flags);
    }
}
