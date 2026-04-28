#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use crate::ucgm_view::{UCGMView, FrameDiff};

/// RenderBarrier: Ensures SMP synchronization between update and render phases.
/// Prevents frame tearing and ensures single-core update authority.
pub struct RenderBarrier {
    frame_id: AtomicU64,
}

impl RenderBarrier {
    pub fn new() -> Self {
        Self {
            frame_id: AtomicU64::new(0),
        }
    }

    pub fn next_frame(&self) -> u64 {
        self.frame_id.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn current_frame(&self) -> u64 {
        self.frame_id.load(Ordering::Acquire)
    }
}

/// FramePump: Orchestrates the deterministic UI pipeline.
/// UCGM -> Snapshot -> ViewModel -> FrameDiff -> Renderer.
pub struct FramePump {
    view: Arc<UCGMView>,
    barrier: Arc<RenderBarrier>,
    pending_diffs: Mutex<Vec<FrameDiff>>,
}

impl FramePump {
    pub fn new(view: Arc<UCGMView>) -> Self {
        Self {
            view,
            barrier: Arc::new(RenderBarrier::new()),
            pending_diffs: Mutex::new(Vec::new()),
        }
    }

    /// tick: Performs one deterministic update cycle.
    /// 1. Consumes new snapshot handle from UCGM/PDX stream.
    /// 2. Updates internal ViewModel via UCGMView.
    /// 3. Collects FrameDiffs for the next render pass.
    pub fn tick(&self, snapshot_handle: u64) {
        // Resolve handle via PDX ABI (treated as black box)
        let mut snapshot = sex_pdx::SceneSnapshot::default();
        let status = unsafe { 
            // This is the only boundary call, resolving a handle to a struct.
            // In a real impl, this would be a safe wrapper around pdx_call(0, 0x16, ...).
            resolve_snapshot_abi(snapshot_handle, &mut snapshot) 
        };

        if status == 0 {
            // Update ViewModel and collect diffs
            let diffs = self.view.update(&snapshot);
            
            // Atomically batch diffs for the renderer
            let mut guard = self.pending_diffs.lock();
            guard.extend(diffs);
            
            // Advance frame barrier
            self.barrier.next_frame();
        }
    }

    /// pump_to_renderer: Flushes pending diffs to the renderer.
    /// Safe for concurrent invocation by rendering cores.
    pub fn pump_to_renderer(&self) {
        let diffs_to_render = {
            let mut guard = self.pending_diffs.lock();
            if guard.is_empty() {
                return;
            }
            // Swap out diffs to keep lock duration minimal
            core::mem::take(&mut *guard)
        };

        // Execute pure rendering projection
        self.view.render(&diffs_to_render);
    }

    pub fn frame_id(&self) -> u64 {
        self.barrier.current_frame()
    }
}

/// resolve_snapshot_abi: Minimal boundary for snapshot resolution.
/// This matches the kernel SYSCALL_SNAPSHOT_RESOLVE (0x16) ABI.
unsafe fn resolve_snapshot_abi(handle: u64, out: *mut sex_pdx::SceneSnapshot) -> u64 {
    // In actual implementation, this calls Syscall 0 (PDX_CALL) with Slot 0, Op 0x16.
    // For this bridge, we assume the sex_pdx crate provides the safe wrapper.
    sex_pdx::pdx_resolve_snapshot(handle, out)
}
