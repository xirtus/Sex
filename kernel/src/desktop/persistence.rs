use alloc::sync::Arc;
use crate::desktop::graph::DesktopGraph;

#[derive(Clone, Debug)]
pub struct DesktopGraphSnapshot {
    pub focus_pd: Option<u32>,
    pub workspace_id: u32,
    pub monotonic_event_counter: u64,
}

pub struct PersistenceEngine;

impl PersistenceEngine {
    pub fn snapshot(graph: &DesktopGraph) -> DesktopGraphSnapshot {
        DesktopGraphSnapshot {
            focus_pd: graph.focus.map(|f| f.pd_id),
            workspace_id: graph.workspace_id,
            monotonic_event_counter: crate::hal::get_monotonic_counter(),
        }
    }
}

