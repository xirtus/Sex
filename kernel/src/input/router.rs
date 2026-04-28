use crate::input::{FocusGraph, FocusNode};
use crate::ipc::messages::MessageType;
use crate::ipc::DOMAIN_REGISTRY;

pub struct InputRouter {
    pub focus: FocusGraph,
}

impl InputRouter {
    pub fn route_event(&self, event: MessageType) {
        if let Some(target) = self.focus.get_focus() {
            if let Some(pd) = DOMAIN_REGISTRY.get(target.pd_id) {
                unsafe {
                    let _ = (*pd.message_ring).enqueue(event);
                }
            }
        }
    }
}
