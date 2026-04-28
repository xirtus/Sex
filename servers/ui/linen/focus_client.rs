use crate::ucgm_view::ViewModel;
use alloc::sync::Arc;
use spin::RwLock;

pub struct FocusClient {
    pub current_focus: Arc<RwLock<Option<u32>>>,
}

impl FocusClient {
    pub fn new() -> Self {
        Self {
            current_focus: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_focus(&self, pd_id: u32) {
        let mut focus = self.current_focus.write();
        *focus = Some(pd_id);
    }
}
