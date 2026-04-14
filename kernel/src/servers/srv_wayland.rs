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
        serial_println!("Wayland: Creating surface for Client PD {} ({}x{})", client_pd_id, width, height);
        
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

    /// Renders the scene by composing surfaces to the LFB.
    pub fn redraw(&self) {
        // In a real system, this would iterate through surfaces and perform 2D/3D blitting.
        if let Some((id, _)) = self.surfaces.iter().next() {
            serial_println!("Wayland: Redrawing Surface {} to LFB.", id);
            // Simulating page flip
            let _ = self.drm.page_flip(1);
        }
    }
}

pub extern "C" fn wayland_entry(arg: u64) -> u64 {
    serial_println!("Wayland PDX: Received compositor request {:#x}", arg);
    0
}
