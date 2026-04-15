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
