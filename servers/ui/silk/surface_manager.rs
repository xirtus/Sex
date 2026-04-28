use alloc::vec::Vec;
use spin::RwLock;
use alloc::sync::Arc;

pub struct Surface {
    pub surface_id: u32,
    pub bounds: [i32; 4],
}

pub struct SurfaceManager {
    pub surfaces: Arc<RwLock<Vec<Surface>>>,
}

impl SurfaceManager {
    pub fn new() -> Self {
        Self {
            surfaces: Arc::new(RwLock::new(Vec::new())),
        }
    }
}
