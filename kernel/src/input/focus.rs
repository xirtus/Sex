use alloc::sync::Arc;
use spin::RwLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FocusNode {
    pub pd_id: u32,
}

pub struct FocusGraph {
    pub active_focus: Arc<RwLock<Option<FocusNode>>>,
}

impl FocusGraph {
    pub fn new() -> Self {
        Self {
            active_focus: Arc::new(RwLock::new(None)),
        }
    }

    pub fn switch_focus(&self, pd_id: u32) {
        let mut focus = self.active_focus.write();
        *focus = Some(FocusNode { pd_id });
    }

    pub fn get_focus(&self) -> Option<FocusNode> {
        *self.active_focus.read()
    }
}
