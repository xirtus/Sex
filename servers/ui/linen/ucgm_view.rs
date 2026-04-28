#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use spin::RwLock;

/// NodeState: Pure representation of a UCGM node's execution state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeState {
    Idle,
    Pending,
    Ready,
    Consumed,
}

/// NodeView: Pure representation of a UCGM node (PD/Slot).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeView {
    pub id: u32,
    pub slot: u32,
    pub state: NodeState,
    pub is_sync: bool,
}

/// EdgeView: Pure representation of a capability edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeView {
    pub from_pd: u32,
    pub to_pd: u32,
    pub slot: u32,
    pub is_async: bool,
}

/// ViewModel: The current active state of the visible graph projection.
/// Send + Sync safe.
#[derive(Clone, Debug, Default)]
pub struct ViewModel {
    pub nodes: BTreeMap<u32, NodeView>,
    pub edges: Vec<EdgeView>,
}

/// FrameDiff: Minimal delta representing changes in the graph.
#[derive(Clone, Debug)]
pub enum FrameDiff {
    NodeAdded(NodeView),
    NodeRemoved(u32),
    NodeUpdated(NodeView),
    EdgeAdded(EdgeView),
    EdgeRemoved(EdgeView),
    SnapshotUpdated(u64),
}

/// UCGMView: SMP-safe projection layer.
/// Double-buffered via Arc<RwLock<ViewModel>> for concurrent multi-core rendering.
pub struct UCGMView {
    model: Arc<RwLock<ViewModel>>,
}

impl UCGMView {
    pub fn new() -> Self {
        Self {
            model: Arc::new(RwLock::new(ViewModel::default())),
        }
    }

    /// update: Transforms snapshot into ViewModel state.
    /// Thread-safe write access ensuring no races with concurrent renderers.
    pub fn update(&self, snapshot: &sex_pdx::SceneSnapshot) -> Vec<FrameDiff> {
        let mut diffs = Vec::new();
        let mut next_model = ViewModel::default();

        let layers = unsafe {
            core::slice::from_raw_parts(
                snapshot.layers_ptr as *const sex_pdx::Layer,
                snapshot.layers_len as usize,
            )
        };

        {
            // 1. Read-only diff computation
            let current = self.model.read();
            for layer in layers {
                let node = NodeView {
                    id: layer.win_id,
                    slot: 0,
                    state: NodeState::Ready,
                    is_sync: false,
                };
                
                if let Some(prev) = current.nodes.get(&node.id) {
                    if prev != &node {
                        diffs.push(FrameDiff::NodeUpdated(node.clone()));
                    }
                } else {
                    diffs.push(FrameDiff::NodeAdded(node.clone()));
                }
                next_model.nodes.insert(node.id, node);
            }

            for id in current.nodes.keys() {
                if !next_model.nodes.contains_key(id) {
                    diffs.push(FrameDiff::NodeRemoved(*id));
                }
            }
        }

        // 2. Atomic swap / update
        {
            let mut current = self.model.write();
            *current = next_model;
        }

        diffs
    }

    /// render: PURE projection function.
    /// Safe for concurrent invocation across multiple rendering cores.
    pub fn render(&self, diffs: &[FrameDiff]) {
        // Projection logic only. 
        // Side-effect free relative to UCGM/Kernel state.
        for diff in diffs {
            match diff {
                FrameDiff::NodeAdded(_node) => {
                    // Render Node
                }
                FrameDiff::NodeUpdated(_node) => {
                    // Update Node View
                }
                FrameDiff::NodeRemoved(_id) => {
                    // Remove Node View
                }
                _ => {}
            }
        }
    }

    /// get_snapshot: Returns a thread-safe read handle to the current graph state.
    pub fn get_snapshot(&self) -> Arc<RwLock<ViewModel>> {
        self.model.clone()
    }
}
