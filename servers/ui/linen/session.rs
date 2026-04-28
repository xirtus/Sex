use crate::linen::fs::FileSystem;
use crate::linen::cap_view::CapabilityView;
use alloc::sync::Arc;
use spin::RwLock;

pub struct SessionGraph {
    pub cwd_node: u32,
    pub focus_node: Option<u32>,
    pub active_surfaces: Vec<u32>,
}

pub struct SessionManager {
    pub state: Arc<RwLock<SessionGraph>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(SessionGraph {
                cwd_node: 1,
                focus_node: None,
                active_surfaces: Vec::new(),
            })),
        }
    }
}
