#![no_std]

use sex_pdx::pdx_call;
use core::sync::atomic::{AtomicU16, Ordering};

#[repr(C)]
pub enum DriverType {
    Native,
    Dde,
}

pub struct DriverRegistry {
    nvme_bar: u64,
    sq_tail: AtomicU16,
}

impl DriverRegistry {
    pub fn new() -> Self {
        let nvme_bar = pdx_call(1, 13, 0, 0);
        Self {
            nvme_bar,
            sq_tail: AtomicU16::new(0),
        }
    }

    pub fn dispatch(&self, cmd: u32, lba: u64, size: u64, buf_cap: u32) -> i64 {
        let phys_addr = pdx_call(1, 12, buf_cap as u64, 0);
        if phys_addr == 0 { return -1; }

        let tail = self.sq_tail.load(Ordering::Acquire);
        let sq_base = (self.nvme_bar + 0x2000) as *mut u32;
        
        unsafe {
            let entry = sq_base.add(tail as usize * 16);
            entry.offset(0).write_volatile(if cmd == 1 { 0x02 } else { 0x01 });
            entry.offset(1).write_volatile(1);
            entry.offset(6).write_volatile(phys_addr as u32);
            entry.offset(7).write_volatile((phys_addr >> 32) as u32);
            entry.offset(10).write_volatile(lba as u32);
            entry.offset(11).write_volatile((lba >> 32) as u32);
            entry.offset(12).write_volatile((size / 512 - 1) as u32);

            let doorbell = (self.nvme_bar + 0x1008) as *mut u32;
            doorbell.write_volatile(tail as u32 + 1);
        }

        self.sq_tail.store((tail + 1) % 32, Ordering::Release);
        0
    }

    pub fn handle_irq(&self, _vector: u8) {
        // Handle MSI-X completion
    }
}
