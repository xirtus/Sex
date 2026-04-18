use crate::hal::{HAL, HardwareAbstractionLayer};
use crate::serial_println;
use crate::ipc::DOMAIN_REGISTRY;
use crate::capability::{CapabilityData, PciCapData};

pub fn init(sexdrive_pd_id: u32, sexdisplay_pd_id: u32) {
    serial_println!("DevMgr: Starting hardware discovery...");
    
    let devices = HAL.enumerate_pci();
    serial_println!("DevMgr: Enumerated {} PCI devices.", devices.len());

    for dev in devices {
        match (dev.class_id, dev.subclass_id) {
            (0x01, 0x08) => { // NVMe
                serial_println!("DevMgr: Assigning NVMe ({:02x}:{:02x}.{}) to sexdrive", dev.bus, dev.dev, dev.func);
                if let Some(pd) = DOMAIN_REGISTRY.get(sexdrive_pd_id) {
                    pd.grant(CapabilityData::Pci(PciCapData {
                        bus: dev.bus,
                        dev: dev.dev,
                        func: dev.func,
                        vendor_id: dev.vendor_id,
                        device_id: dev.device_id,
                    }));
                    // Route IRQ (Simplified vector mapping)
                    crate::interrupts::register_irq_route(0x22, sexdrive_pd_id);
                }
            }
            (0x03, _) => { // GPU
                serial_println!("DevMgr: Assigning GPU ({:02x}:{:02x}.{}) to sexdisplay", dev.bus, dev.dev, dev.func);
                if let Some(pd) = DOMAIN_REGISTRY.get(sexdisplay_pd_id) {
                    pd.grant(CapabilityData::Pci(PciCapData {
                        bus: dev.bus,
                        dev: dev.dev,
                        func: dev.func,
                        vendor_id: dev.vendor_id,
                        device_id: dev.device_id,
                    }));
                    crate::interrupts::register_irq_route(0x23, sexdisplay_pd_id);
                }
            }
            _ => {}
        }
    }

    serial_println!("DevMgr: Hardware discovery complete.");
}
