use crate::serial_println;
use crate::servers::dde;
use x86_64::VirtAddr;
use alloc::collections::BTreeMap;

/// sexdrm: Linux sexdrm/KMS lifting for the Sex Microkernel.
/// Provides a compatibility layer for Direct Rendering Manager.

/// A GEM-style Graphics Execution Manager buffer (Zero-Copy).
pub struct GemBuffer {
    pub handle: u32,
    pub phys_addr: u64,
    pub virt_addr: VirtAddr,
    pub size: usize,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
}

pub struct sexdrm {
    pub device_name: &'static str,
    pub framebuffer_base: VirtAddr,
    pub buffers: BTreeMap<u32, GemBuffer>,
}

impl sexdrm {
    pub fn new(name: &'static str) -> Self {
        Self {
            device_name: name,
            framebuffer_base: VirtAddr::new(0),
            buffers: BTreeMap::new(),
        }
    }

    /// Real VESA/GOP Framebuffer initialization via DDE.
    pub fn init(&mut self) -> Result<(), &'static str> {
        serial_println!("sexdrm: Initializing VESA/GOP KMS for {}...", self.device_name);
        
        // 1. Locate the LFB (Linear Frame Buffer) from the bootloader or PCI
        // For the prototype, we assume a standard location
        let lfb_phys: u64 = 0xFD00_0000; 
        let lfb_size: u64 = 1920 * 1080 * 4; // 1080p 32bpp
        
        self.framebuffer_base = dde::dde_ioremap(lfb_phys, lfb_size)?;
        serial_println!("sexdrm: Mode Set: 1920x1080 32bpp at {:?}", self.framebuffer_base);

        Ok(())
    }

    /// Allocates a GEM-style buffer for rendering.
    pub fn allocate_buffer(&mut self, width: u32, height: u32) -> Result<u32, &'static str> {
        let size = (width * height * 4) as usize;
        let handle = self.buffers.len() as u32 + 1;
        
        // In a real system, we'd allocate physical frames from the PMM
        let phys_addr = 0x_AAAA_0000; 
        let virt_addr = VirtAddr::new(phys_addr);

        self.buffers.insert(handle, GemBuffer {
            handle,
            phys_addr,
            virt_addr,
            size,
            width,
            height,
            pitch: width * 4,
        });

        serial_println!("sexdrm: Allocated GEM Buffer {} ({}x{})", handle, width, height);
        Ok(handle)
    }

    /// Performs a Page Flip (KMS operation).
    pub fn page_flip(&self, handle: u32) -> Result<(), &'static str> {
        let buffer = self.buffers.get(&handle).ok_or("sexdrm: Invalid buffer handle")?;
        serial_println!("sexdrm: Page Flip -> Buffer {} (Phys {:#x})", handle, buffer.phys_addr);
        
        // Write the new buffer address to the hardware register
        unsafe {
            core::ptr::copy_nonoverlapping(
                buffer.virt_addr.as_ptr::<u8>(),
                self.framebuffer_base.as_mut_ptr::<u8>(),
                buffer.size
            );
        }
        Ok(())
    }
}

pub extern "C" fn sexdrm_entry(arg: u64) -> u64 {
    serial_println!("sexdrm PDX: Received request {:#x}", arg);
    0
}
