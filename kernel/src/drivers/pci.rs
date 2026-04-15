use crate::serial_println;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_id: u8,
    pub subclass_id: u8,
}

impl PciDevice {
    pub fn read_u32(&self, offset: u8) -> u32 {
        let address: u32 = 0x80000000 
            | ((self.bus as u32) << 16) 
            | ((self.dev as u32) << 11) 
            | ((self.func as u32) << 8) 
            | (offset as u32 & 0xFC);
        
        unsafe {
            x86_64::instructions::port::Port::new(0xCF8).write(address);
            x86_64::instructions::port::Port::new(0xCFC).read()
        }
    }

    pub fn write_u32(&self, offset: u8, value: u32) {
        let address: u32 = 0x80000000 
            | ((self.bus as u32) << 16) 
            | ((self.dev as u32) << 11) 
            | ((self.func as u32) << 8) 
            | (offset as u32 & 0xFC);
        
        unsafe {
            x86_64::instructions::port::Port::new(0xCF8).write(address);
            x86_64::instructions::port::Port::new(0xCFC).write(value);
        }
    }

    pub fn get_bar(&self, index: u8) -> u64 {
        let offset = 0x10 + (index * 4);
        let bar_lo = self.read_u32(offset);
        
        // Simplified for prototype: Assume 64-bit BARs and ignore flags
        if bar_lo & 0x4 != 0 {
            let bar_hi = self.read_u32(offset + 4);
            ((bar_hi as u64) << 32) | (bar_lo as u64 & !0xF)
        } else {
            bar_lo as u64 & !0xF
        }
    }

    pub fn setup_msix(&self, vector: u8, target_core: u8) {
        serial_println!("PCI: Configuring MSI-X for {:02x}:{:02x}.{:x} -> Vector {:#x}", 
            self.bus, self.dev, self.func, vector);
        
        // 1. Find MSI-X Capability (ID 0x11)
        let mut cap_ptr = (self.read_u32(0x34) & 0xFF) as u8;
        while cap_ptr != 0 {
            let cap_header = self.read_u32(cap_ptr);
            if (cap_header & 0xFF) == 0x11 { // MSI-X
                let table_info = self.read_u32(cap_ptr + 4);
                let table_vaddr = self.get_bar((table_info & 0x7) as u8) + (table_info & !0x7) as u64;

                // For the lock-free prototype, we write directly to the physical address 
                // assuming a 1:1 mapping or an offset in the SAS.
                let phys_offset = 0xFFFF_8000_0000_0000;
                let entry = (table_vaddr + phys_offset) as *mut u32;

                unsafe {
                    entry.write_volatile(0xFEE0_0000 | ((target_core as u32) << 12)); // Msg Addr
                    entry.add(1).write_volatile(0);                                  // Msg Upper
                    entry.add(2).write_volatile(vector as u32);                      // Msg Data
                    entry.add(3).write_volatile(0);                                  // Unmask
                }

                // Enable MSI-X
                self.write_u32(cap_ptr, cap_header | (1 << 31));
                serial_println!("PCI: MSI-X Enabled.");
                return;
            }
            cap_ptr = ((cap_header >> 8) & 0xFF) as u8;
        }
        serial_println!("PCI: Warning - No MSI-X capability.");
    }
}

pub fn enumerate_pci() -> Vec<PciDevice> {
    let mut devices = Vec::new();
    // Simplified scan for bus 0, device 1, func 0 (common NVMe spot in QEMU)
    // In a real system, we iterate all buses/devices.
    let dev = PciDevice { bus: 0, dev: 1, func: 0, vendor_id: 0, device_id: 0, class_id: 1, subclass_id: 8 };
    let vendor = dev.read_u32(0) & 0xFFFF;
    if vendor != 0xFFFF {
        devices.push(dev);
    }
    devices
}
