use crate::surface::surface::SurfaceNode;
use alloc::vec::Vec;
use spin::RwLock;
use alloc::sync::Arc;

pub struct SurfaceManager {
    pub surfaces: Arc<RwLock<Vec<SurfaceNode>>>,
}

impl SurfaceManager {
    pub fn new() -> Self {
        Self {
            surfaces: Arc::new(RwLock::new(Vec::new())),
        }
    }
}
