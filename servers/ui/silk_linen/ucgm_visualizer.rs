#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use crate::ucgm_view::{NodeView, EdgeView, NodeState, FrameDiff};

/// ViewGraph: Frame-local, deterministic graph projection.
pub struct ViewGraph {
    pub nodes: Vec<NodeView>,
    pub edges: Vec<EdgeView>,
    pub epoch: u64,
}

pub struct UCGMVisualizer;

impl UCGMVisualizer {
    pub const fn new() -> Self {
        Self
    }

    /// process_frame: Reconstructs the graph model for a single VBlank tick.
    /// Performs latest-wins coalescing and stable node ordering.
    pub fn process_frame(&self, epoch: u64, snapshot_handle: u64, diffs: &[FrameDiff]) -> ViewGraph {
        let mut node_map: BTreeMap<u32, NodeView> = BTreeMap::new();

        // 1. Snapshot Resolution Phase (Reconstruction)
        let mut snapshot = sex_pdx::SceneSnapshot::default();
        // SAFETY: snapshot_resolve is a kernel ABI boundary for zero-copy read.
        if sex_pdx::pdx_resolve_snapshot(snapshot_handle, &mut snapshot) == 0 {
            let layers = unsafe {
                core::slice::from_raw_parts(
                    snapshot.layers_ptr as *const sex_pdx::Layer,
                    snapshot.layers_len as usize,
                )
            };
            for layer in layers {
                node_map.insert(layer.win_id, NodeView {
                    id: layer.win_id,
                    slot: 0,
                    state: NodeState::Ready,
                    is_sync: false,
                });
            }
        }

        // 2. Coalescing Engine (Latest-wins per tick)
        for diff in diffs {
            match diff {
                FrameDiff::NodeAdded(nv) | FrameDiff::NodeUpdated(nv) => {
                    node_map.insert(nv.id, nv.clone());
                }
                FrameDiff::NodeRemoved(id) => {
                    node_map.remove(id);
                }
                _ => {}
            }
        }

        // 3. Stable Ordering: sort by node_id
        let mut nodes: Vec<NodeView> = node_map.into_values().collect();
        nodes.sort_by_key(|n| n.id);

        ViewGraph {
            nodes,
            edges: Vec::new(),
            epoch,
        }
    }

    /// render: Pure deterministic projection to framebuffer.
    /// No locks. No mutation. Stable layout.
    pub fn render(&self, graph: &ViewGraph) {
        // Deterministic grid layout mapping
        // Render Node count: graph.nodes.len()
        // Render Frame: graph.epoch
    }
}
