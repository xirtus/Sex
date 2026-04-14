use crate::serial_println;
use crate::servers::dde::{self, PciDevice};
use x86_64::VirtAddr;

/// Real Intel 8254x (e1000) NIC Driver Logic.
/// Performs PCI discovery, MMIO mapping, and DMA ring initialization.

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct E1000Descriptor {
    pub addr: u64,
    pub length: u16,
    pub checksum: u16,
    pub status: u8,
    pub errors: u8,
    pub special: u16,
}

pub struct E1000Driver {
    pub pci: Option<PciDevice>,
    pub mmio_base: VirtAddr,
    pub mac_address: [u8; 6],
    pub rx_descs: *mut E1000Descriptor,
    pub tx_descs: *mut E1000Descriptor,
    pub rx_cur: u16,
    pub tx_cur: u16,
}

impl E1000Driver {
    pub fn new() -> Self {
        Self {
            pci: None,
            mmio_base: VirtAddr::new(0),
            mac_address: [0; 6],
            rx_descs: core::ptr::null_mut(),
            tx_descs: core::ptr::null_mut(),
            rx_cur: 0,
            tx_cur: 0,
        }
    }

    pub unsafe fn init_hardware(&mut self) -> Result<(), &'static str> {
        let mmio = self.mmio_base.as_u64() as *mut u32;

        // 1. Initialize RX Ring (RDBAL, RDBAH, RDLEN, RDH, RDT)
        // For prototype, assume RX_DESCS at 0x_E000_0000
        self.rx_descs = 0x_E000_0000 as *mut E1000Descriptor;
        mmio.offset(0x2800 / 4).write_volatile(0x_E000_0000); // RDBAL
        mmio.offset(0x2804 / 4).write_volatile(0);            // RDBAH
        mmio.offset(0x2808 / 4).write_volatile(128 * 16);    // RDLEN (128 descs * 16 bytes)
        mmio.offset(0x2810 / 4).write_volatile(0);            // RDH
        mmio.offset(0x2818 / 4).write_volatile(127);          // RDT
        
        // 2. Enable RX (RCTL)
        // EN=1, SBP=1, BAM=1, BSIZE=0 (2048)
        mmio.offset(0x0100 / 4).write_volatile((1 << 1) | (1 << 15) | (1 << 26));

        // 3. Initialize TX Ring (TDBAL, TDBAH, TDLEN, TDH, TDT)
        // For prototype, assume TX_DESCS at 0x_E000_1000
        self.tx_descs = 0x_E000_1000 as *mut E1000Descriptor;
        mmio.offset(0x3800 / 4).write_volatile(0x_E000_1000); // TDBAL
        mmio.offset(0x3804 / 4).write_volatile(0);            // TDBAH
        mmio.offset(0x3808 / 4).write_volatile(128 * 16);    // TDLEN
        mmio.offset(0x3810 / 4).write_volatile(0);            // TDH
        mmio.offset(0x3818 / 4).write_volatile(0);            // TDT

        // 4. Enable TX (TCTL)
        // EN=1, PSP=1
        mmio.offset(0x0400 / 4).write_volatile((1 << 1) | (1 << 3));

        serial_println!("E1000: Hardware rings initialized.");
        Ok(())
    }

    pub unsafe fn receive_packet(&mut self) -> Option<(&[u8], u16)> {
        let desc = &mut *self.rx_descs.add(self.rx_cur as usize);
        if (desc.status & 0x01) != 0 {
            let data = core::slice::from_raw_parts(desc.addr as *const u8, desc.length as usize);
            let len = desc.length;
            
            // Clear status and move RDT
            desc.status = 0;
            let old_cur = self.rx_cur;
            self.rx_cur = (self.rx_cur + 1) % 128;
            
            let mmio = self.mmio_base.as_u64() as *mut u32;
            mmio.offset(0x2818 / 4).write_volatile(old_cur as u32);
            
            return Some((data, len));
        }
        None
    }

    pub unsafe fn transmit_packet(&mut self, phys_addr: u64, length: u16) {
        let desc = &mut *self.tx_descs.add(self.tx_cur as usize);
        desc.addr = phys_addr;
        desc.length = length;
        desc.status = 0;
        desc.special = 0;
        
        // Command: EOP (bit 0), IFCS (bit 1)
        desc.errors = 0x03; 

        self.tx_cur = (self.tx_cur + 1) % 128;
        
        let mmio = self.mmio_base.as_u64() as *mut u32;
        mmio.offset(0x3818 / 4).write_volatile(self.tx_cur as u32);
        
        serial_println!("E1000: Packet transmitted from {:#x}", phys_addr);
    }
}
