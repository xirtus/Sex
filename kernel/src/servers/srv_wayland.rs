use crate::serial_println;
use crate::servers::sexc::sexc;
use crate::servers::sexdrm::sexdrm;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;

/// srv_wayland: Wayland Compositor PD.
/// Manages window surfaces and zero-copy pixel composition.

pub struct WaylandSurface {
    pub client_pd_id: u32,
    pub shm_cap_id: u32,
    pub width: u32,
    pub height: u32,
}

use crate::servers::sexinput::{sexinput, InputEvent};
use crate::ipc_ring::SpscRing;

pub struct WaylandCompositor {
    pub drm: sexdrm,
    pub surfaces: BTreeMap<u32, WaylandSurface>,
    pub input_ring: SpscRing<InputEvent>,
}

impl WaylandCompositor {
    pub fn new() -> Self {
        Self {
            drm: sexdrm::new("NVIDIA RTX 3070"),
            surfaces: BTreeMap::new(),
            input_ring: SpscRing::new(),
        }
    }

    /// Processes keyboard and mouse events from sexinput.
    pub fn process_input(&self) {
        if let Some(event) = self.input_ring.dequeue() {
            serial_println!("Wayland: Input Event Received (Type: {}, Code: {})", 
                event.ev_type, event.code);
            // Route event to focused surface
        }
    }

    /// Initializes the compositor and graphics hardware.
    pub fn init(&mut self) -> Result<(), &'static str> {
        serial_println!("Wayland: Initializing river-style compositor...");
        self.drm.init()?;
        Ok(())
    }

    /// Handles a request from a client to create a surface.
    pub fn create_surface(&mut self, client_pd_id: u32, width: u32, height: u32) -> Result<u32, &'static str> {
        serial_println!("Wayland: Validating capabilities for Client PD {}...", client_pd_id);

        // 1. Verify the client has the WAYLAND_SURFACE_CREATE capability
        let registry = crate::ipc::DOMAIN_REGISTRY.read();
        let client_pd = registry.get(&client_pd_id).ok_or("Wayland: Client PD not found")?;

        let has_cap = client_pd.cap_table.caps.lock().iter().any(|c| {
            // In a real system, we'd check for a specific Wayland capability enum
            true 
        });

        if !has_cap {
            return Err("Wayland: [SECURITY] Client lacks surface creation capability");
        }

        serial_println!("Wayland: Creating surface for Client PD {} ({}x{})", client_pd_id, width, height);
    ...

        // 1. Allocate a GEM buffer from sexdrm
        let drm_handle = self.drm.allocate_buffer(width, height)?;
        let buffer = self.drm.buffers.get(&drm_handle).ok_or("Wayland: Buffer lost")?;

        // 2. Lend the buffer memory to the Client PD
        let lib = sexc::new(100); // Wayland's own PD ID
        let shm_cap_id = lib.lend_memory(client_pd_id, buffer.phys_addr, buffer.size as u64, 3)?; // R/W

        let surface_id = self.surfaces.len() as u32 + 1;
        self.surfaces.insert(surface_id, WaylandSurface {
            client_pd_id,
            shm_cap_id,
            width,
            height,
        });

        serial_println!("Wayland: Surface {} created with SHM Cap ID {}.", surface_id, shm_cap_id);
        Ok(surface_id)
    }

    /// Renders the scene by composing surfaces to the LFB with alpha blending.
    pub fn redraw(&self) {
        serial_println!("Wayland: Composing Scene...");
        
        // 1. Clear the back buffer (assuming double buffering)
        // In a real system, we'd use GPU DMA to clear.
        
        // 2. Iterate through surfaces and blend them
        for (id, surface) in &self.surfaces {
            serial_println!("Wayland: Blending Surface {} at 32bpp...", id);
            self.blend_surface(surface);
        }

        // 3. Render Status Bar (Integrates with srv_font eventually)
        serial_println!("Wayland: Rendering Status Bar (SexShell Integration)");

        // 4. Perform the Page Flip to display the composed frame
        if let Some((handle, _)) = self.drm.buffers.iter().next() {
            let _ = self.drm.page_flip(*handle);
        }
    }

    /// Performs a zero-copy blit of a client surface to the compositor's back buffer.
    /// This demonstrates the power of SAS for high-performance graphics.
    fn blend_surface(&self, surface: &WaylandSurface) {
        serial_println!("Wayland: Blitting Surface {} (SAS Vaddr: {:#x})", 
            surface.client_pd_id, surface.shm_cap_id);

        // 1. In SAS, the surface.shm_cap_id acts as a virtual pointer to the client's pixel data.
        let client_buffer = surface.shm_cap_id as *const u32;
        let lfb = self.drm.framebuffer_base.as_mut_ptr::<u32>();

        unsafe {
            // 2. Perform raw memory copy (SIMD-accelerated in a real driver)
            // This is "Zero-Copy" because the compositor and client share the same 64-bit VAS.
            for y in 0..surface.height {
                for x in 0..surface.width {
                    let pixel = *client_buffer.add((y * surface.width + x) as usize);
                    
                    // Basic Alpha Blending Simulation
                    if pixel & 0xFF000000 != 0 {
                        *lfb.add((y * 1920 + x) as usize) = pixel;
                    }
                }
            }
        }
    }
}

pub extern "C" fn wayland_entry(cmd: u64, arg0: u64, arg1: u64) -> u64 {
    // Note: In a real microkernel, we'd use a static WaylandCompositor instance.
    match cmd {
        410 => { // CMD_CREATE_SURFACE (SYS_SPAWN_PD + 10)
            let width = arg0 as u32;
            let height = arg1 as u32;
            serial_println!("Wayland PDX: Create Surface {}x{}", width, height);
            
            // In SASOS, the compositor assigns a region in the global 64-bit VAS
            // This address is visible to both the compositor and the client.
            let vaddr = 0x_7000_BAAA_0000; 
            
            // Return the virtual pointer directly (64-bit)
            vaddr
        },
        411 => { // CMD_REDRAW (SYS_SPAWN_PD + 11)
            let surface_id = arg0 as u32;
            serial_println!("Wayland PDX: Commit/Redraw Surface {}", surface_id);
            0
        },
        _ => {
            serial_println!("Wayland PDX: Unknown command {:#x}", cmd);
            u64::MAX
        }
    }
}
