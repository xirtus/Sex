use crate::hal::{HAL, HardwareAbstractionLayer};
use crate::serial_println;
use crate::ipc::DOMAIN_REGISTRY;
use crate::capability::{CapabilityData, PciCapData, InterruptCapData};

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
            (0x0c, 0x03) if dev.prog_if == 0x30 => { // USB XHCI
                let bar0 = dev.get_bar(0);
                let irq_line = ((dev.read_u32(0x3C) >> 0) & 0xFF) as u8;
                serial_println!(
                    "DevMgr: Discovered USB XHCI ({:02x}:{:02x}.{}) vendor={:04x} device={:04x} bar0={:#x} irq_line={:#x}",
                    dev.bus, dev.dev, dev.func, dev.vendor_id, dev.device_id, bar0, irq_line
                );
                if let Some(pd) = DOMAIN_REGISTRY.get(sexdrive_pd_id) {
                    // Strict lease boundary: one XHCI PCI capability to one driver PD slot.
                    pd.grant_capability(sex_pdx::SLOT_USB_HOST, CapabilityData::Pci(PciCapData {
                        bus: dev.bus,
                        dev: dev.dev,
                        func: dev.func,
                        vendor_id: dev.vendor_id,
                        device_id: dev.device_id,
                    }));
                    pd.grant(CapabilityData::Interrupt(InterruptCapData { irq: irq_line }));
                    serial_println!(
                        "DevMgr: Leased XHCI to sexdrive pd={} slot={}",
                        sexdrive_pd_id,
                        sex_pdx::SLOT_USB_HOST
                    );
                }
            }
            (0x0c, _) => { // USB (other)
                serial_println!("DevMgr: Discovered USB controller class={:02x}:{:02x} ({:02x}:{:02x}.{}) vendor={:04x} device={:04x}", dev.class_id, dev.subclass_id, dev.bus, dev.dev, dev.func, dev.vendor_id, dev.device_id);
            }
            _ => {}
        }
    }

    serial_println!("DevMgr: Hardware discovery complete.");
}
