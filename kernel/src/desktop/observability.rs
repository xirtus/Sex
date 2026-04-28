use crate::desktop::graph::DesktopGraph;

pub struct DesktopObservability;

impl DesktopObservability {
    pub fn dump_summary(graph: &DesktopGraph) -> (u64, u64) {
        (1, graph.workspace_id as u64)
    }
}
