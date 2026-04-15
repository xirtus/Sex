use crate::serial_println;
use alloc::vec::Vec;
use crate::capability::{PciCapData, CapabilityData};

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
        unsafe { pci_config_read(self.bus, self.dev, self.func, offset) }
    }

    pub fn write_u32(&self, offset: u8, value: u32) {
        unsafe { pci_config_write(self.bus, self.dev, self.func, offset, value) }
    }

    pub fn setup_msix(&self, vector: u8, target_core: u8) {
        serial_println!("PCI: Setting up MSI-X for {:02x}:{:02x}.{:x} -> Vector {:#x}", 
            self.bus, self.dev, self.func, vector);
        
        // 1. Find MSI-X Capability (ID 0x11)
        let mut cap_ptr = (self.read_u32(0x34) & 0xFF) as u8;
        while cap_ptr != 0 {
            let cap_header = self.read_u32(cap_ptr);
            if (cap_header & 0xFF) == 0x11 {
                // MSI-X Found
                let msg_ctrl = (cap_header >> 16) as u16;
                let table_info = self.read_u32(cap_ptr + 4);
                let table_bir = (table_info & 0x7) as u8;
                let table_offset = table_info & !0x7;

                let bar_addr = self.get_bar(table_bir);
                let table_vaddr = bar_addr + table_offset as u64;

                // 2. Configure Entry 0 (Simplified for prototype)
                // Message Address: 0xFEE00000 | (target_core << 12)
                // Message Data: vector (edge-triggered, fixed delivery)
                unsafe {
                    let entry_ptr = table_vaddr as *mut u32;
                    entry_ptr.write_volatile(0xFEE0_0000 | ((target_core as u32) << 12)); // Msg Addr
                    entry_ptr.add(1).write_volatile(0);                                  // Msg Upper Addr
                    entry_ptr.add(2).write_volatile(vector as u32);                      // Msg Data
                    entry_ptr.add(3).write_volatile(0);                                  // Vector Control (Unmask)
                }

                // 3. Enable MSI-X in Message Control register
                self.write_u32(cap_ptr, cap_header | (1 << 31));
                serial_println!("PCI: MSI-X Enabled for vector {:#x}", vector);
                return;
            }
            cap_ptr = ((cap_header >> 8) & 0xFF) as u8;
        }
        serial_println!("PCI: Warning - MSI-X capability not found for device!");
    }

    pub fn get_bar(&self, index: u8) -> u64 {
        let offset = 0x10 + (index * 4);
        let bar = self.read_u32(offset);
        if bar & 0x1 != 0 {
            // I/O Space (Not supported in SASOS MMIO model usually)
            (bar & 0xFFFF_FFFC) as u64
        } else {
            // Memory Space
            let type_bits = (bar >> 1) & 0x3;
            if type_bits == 0x2 { // 64-bit
                let bar_high = self.read_u32(offset + 4);
                ((bar_high as u64) << 32) | (bar as u64 & 0xFFFF_FFF0)
            } else {
                (bar & 0xFFFF_FFF0) as u64
            }
        }
    }
}

pub unsafe fn pci_config_read(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    let address = ((bus as u32) << 16) | ((slot as u32) << 11) |
                  ((func as u32) << 8) | (offset as u32 & 0xFC) | 0x8000_0000;
    x86_64::instructions::port::Port::new(0xCF8).write(address);
    x86_64::instructions::port::Port::new(0xCFC).read()
}

pub unsafe fn pci_config_write(bus: u8, slot: u8, func: u8, offset: u8, value: u32) {
    let address = ((bus as u32) << 16) | ((slot as u32) << 11) |
                  ((func as u32) << 8) | (offset as u32 & 0xFC) | 0x8000_0000;
    x86_64::instructions::port::Port::new(0xCF8).write(address);
    x86_64::instructions::port::Port::new(0xCFC).write(value);
}

pub fn enumerate_bus() -> Vec<PciDevice> {
    let mut devices = Vec::new();
    for bus in 0..256 {
        for slot in 0..32 {
            for func in 0..8 {
                let vendor_id = unsafe { pci_config_read(bus as u8, slot as u8, func as u8, 0) } as u16;
                if vendor_id != 0xFFFF {
                    let device_id = (unsafe { pci_config_read(bus as u8, slot as u8, func as u8, 0) } >> 16) as u16;
                    let class_rev = unsafe { pci_config_read(bus as u8, slot as u8, func as u8, 8) };
                    let class_id = (class_rev >> 24) as u8;
                    let subclass_id = (class_rev >> 16) as u8;

                    devices.push(PciDevice {
                        bus: bus as u8,
                        dev: slot as u8,
                        func: func as u8,
                        vendor_id,
                        device_id,
                        class_id,
                        subclass_id,
                    });
                }
            }
        }
    }
    devices
}
