use alloc::sync::Arc;
use spin::RwLock;
use crate::input::FocusNode;
use crate::surface::SurfaceNode;

pub struct DesktopGraph {
    pub focus: Option<FocusNode>,
    pub active_surface: Option<SurfaceNode>,
    pub workspace_id: u32,
}

pub struct DesktopManager {
    pub graph: Arc<RwLock<DesktopGraph>>,
}

impl DesktopManager {
    pub fn new() -> Self {
        Self {
            graph: Arc::new(RwLock::new(DesktopGraph {
                focus: None,
                active_surface: None,
                workspace_id: 0,
            })),
        }
    }
}
