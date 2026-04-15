use crate::serial_println;
use crate::servers::dde;
use crate::ipc_ring::SpscRing;
use crate::interrupts::InterruptEvent;
use alloc::sync::Arc;
use x86_64::VirtAddr;

/// sexsound: Intel HDA / High-Performance Audio for SexOS.
/// Provides high-performance, isolated sound support with real-time IRQ routing.

pub struct sexsound {
    pub name: &'static str,
    pub interrupt_ring: Arc<SpscRing<InterruptEvent>>,
}

pub struct HdaController {
    pub mmio_base: VirtAddr,
}

impl HdaController {
    pub fn new(mmio_base: VirtAddr) -> Self {
        Self { mmio_base }
    }

    /// Initializes the HDA controller (CORB/RIRB logic).
    pub unsafe fn init_hardware(&self, pd_id: u32) -> Result<(), &'static str> {
        let mmio_ptr = self.mmio_base.as_u64() as *mut u32;
        
        // 1. Reset Controller
        let gctl = mmio_ptr.offset(0x08 / 4);
        gctl.write_volatile(gctl.read_volatile() | 1); 
        while (gctl.read_volatile() & 1) == 0 {}
        
        // 2. Grant DMA Capability for Audio Buffers
        let buffer_phys = 0x_BBBB_0000;
        let registry = crate::ipc::DOMAIN_REGISTRY.read();
        let pd = registry.get(&pd_id).ok_or("sexsound: PD not found")?;
        
        pd.grant(crate::capability::CapabilityData::DMA(crate::capability::DmaCapData {
            phys_addr: buffer_phys,
            length: 64 * 1024, // 64KB audio ring
            pku_key: pd.pku_key,
        }));

        // 3. Set CORB/RIRB base (Simulated)
        let corblbase = mmio_ptr.offset(0x40 / 4);
        corblbase.write_volatile(buffer_phys as u32);
        
        serial_println!("sexsound: Intel HDA Controller ready with DMA at {:#x}.", buffer_phys);
        Ok(())
    }
}

impl sexsound {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            interrupt_ring: Arc::new(SpscRing::new()),
        }
    }

    pub fn init(&mut self, pd_id: u32) -> Result<(), &'static str> {
        serial_println!("sexsound: Scanning for Audio via PCI...");
        
        // 1. Find HDA Controller via real PCI
        let devices = crate::pci::enumerate_bus();
        let pci = devices.into_iter().find(|d| d.class_id == 0x04) // Multimedia Controller
            .ok_or("sexsound: Audio Controller not found")?;

        // 2. Register IRQ (Vector 0x23 for Audio)
        crate::interrupts::register_irq_route(0x23, pd_id, self.interrupt_ring.clone());

        // 3. Map MMIO and Init Hardware
        let mmio_base = dde::dde_ioremap(pci.get_bar(0), 0x4000)?;
        let hda = HdaController::new(mmio_base);
        unsafe { hda.init_hardware(pd_id)?; }

        Ok(())
    }
}

pub extern "C" fn sexsound_entry(arg: u64) -> u64 {
    serial_println!("sexsound PDX: Received audio request {:#x}", arg);
    0
}
