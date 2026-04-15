use crate::serial_println;
use alloc::vec::Vec;
use crate::ipc::DOMAIN_REGISTRY;
use crate::capability::CapabilityData;

#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    pub bus: u8, pub dev: u8, pub func: u8, pub vendor_id: u16, pub device_id: u16, pub class_id: u8, pub subclass_id: u8,
}

impl PciDevice {
    pub fn read_u32(&self, offset: u8) -> u32 {
        let address: u32 = 0x80000000 | ((self.bus as u32) << 16) | ((self.dev as u32) << 11) | ((self.func as u32) << 8) | (offset as u32 & 0xFC);
        unsafe {
            x86_64::instructions::port::Port::new(0xCF8).write(address);
            x86_64::instructions::port::Port::new(0xCFC).read()
        }
    }

    pub fn write_u32(&self, offset: u8, value: u32) {
        let address: u32 = 0x80000000 | ((self.bus as u32) << 16) | ((self.dev as u32) << 11) | ((self.func as u32) << 8) | (offset as u32 & 0xFC);
        unsafe {
            x86_64::instructions::port::Port::new(0xCF8).write(address);
            x86_64::instructions::port::Port::new(0xCFC).write(value);
        }
    }

    pub fn get_bar(&self, index: u8) -> u64 {
        let offset = 0x10 + (index * 4);
        let bar_lo = self.read_u32(offset);
        if bar_lo & 0x4 != 0 {
            let bar_hi = self.read_u32(offset + 4);
            ((bar_hi as u64) << 32) | (bar_lo as u64 & !0xF)
        } else {
            bar_lo as u64 & !0xF
        }
    }

    pub fn setup_msix(&self, vector: u8, target_core: u8) {
        let mut cap_ptr = (self.read_u32(0x34) & 0xFF) as u8;
        while cap_ptr != 0 {
            let cap_header = self.read_u32(cap_ptr);
            if (cap_header & 0xFF) == 0x11 {
                let table_info = self.read_u32(cap_ptr + 4);
                let table_vaddr = self.get_bar((table_info & 0x7) as u8) + (table_info & !0x7) as u64;
                let phys_offset = 0xFFFF_8000_0000_0000;
                let entry = (table_vaddr + phys_offset) as *mut u32;
                unsafe {
                    entry.write_volatile(0xFEE0_0000 | ((target_core as u32) << 12));
                    entry.add(1).write_volatile(0);
                    entry.add(2).write_volatile(vector as u32);
                    entry.add(3).write_volatile(0);
                }
                self.write_u32(cap_ptr, cap_header | (1 << 31));
                return;
            }
            cap_ptr = ((cap_header >> 8) & 0xFF) as u8;
        }
    }
}

pub fn bootstrap_drivers() {
    serial_println!("pci: Starting hardware enumeration...");
    for bus in 0..8 {
        for dev in 0..32 {
            let d = PciDevice { bus, dev, func: 0, vendor_id: 0, device_id: 0, class_id: 0, subclass_id: 0 };
            let vendor = d.read_u32(0) & 0xFFFF;
            if vendor == 0xFFFF { continue; }
            
            let class = (d.read_u32(0x08) >> 24) as u8;
            let subclass = (d.read_u32(0x08) >> 16) as u8;

            if class == 0x01 && subclass == 0x08 { // NVMe
                serial_println!("pci: Found NVMe at {:02x}:{:02x}.0", bus, dev);
                if let Some(pd) = DOMAIN_REGISTRY.get(200) {
                    pd.grant(CapabilityData::Pci(crate::capability::PciCapData { bus, dev, func: 0, vendor_id: vendor as u16, device_id: 0 }));
                }
            } else if class == 0x03 { // Display Controller (GPU)
                serial_println!("pci: Found GPU at {:02x}:{:02x}.0", bus, dev);
                if let Some(pd) = DOMAIN_REGISTRY.get(500) {
                    pd.grant(CapabilityData::Pci(crate::capability::PciCapData { bus, dev, func: 0, vendor_id: vendor as u16, device_id: 0 }));
                }
            }
        }
    }
}
