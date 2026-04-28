use core::marker::PhantomData;
use core::sync::atomic::{AtomicU64, Ordering};
use alloc::vec::Vec;
use sex_pdx::pdx_call_checked;

pub struct ScanoutCap {
    _private: (),
}

impl ScanoutCap {
    pub(crate) const fn issue() -> Self {
        Self { _private: () }
    }
}

#[derive(Clone)]
pub struct Window {
    pub fb_ptr: u64,
    pub width: u32,
    pub height: u32,
}

pub type WindowTable = Vec<Window>;

pub struct WindowSnapshot {
    pub epoch: u64,
    pub windows: Vec<Window>,
}

pub struct ScanoutExecutor {
    _cap: ScanoutCap,
    vblank_counter: u64,
    snapshot_epoch: u64,
    fb_ptr: u64, // Must be valid
}

impl ScanoutExecutor {
    pub fn new(cap: ScanoutCap, fb_ptr: u64) -> Self {
        // Enforce Phase 5 (SCANOUT_ENABLED = 5) via syscall
        let res = pdx_call_checked(0, 0x60, 5, 0, 0);
        assert!(res.is_ok(), "SCANOUT_ENABLED_VIOLATION");
        
        assert!(fb_ptr != 0, "ScanoutExecutor: Invalid Framebuffer Pointer");
        Self {
            _cap: cap,
            vblank_counter: 0,
            snapshot_epoch: 0,
            fb_ptr,
        }
    }

    pub fn snapshot(&mut self, windows: &WindowTable) -> WindowSnapshot {
        self.snapshot_epoch += 1;
        WindowSnapshot {
            epoch: self.snapshot_epoch,
            windows: windows.iter().cloned().collect(),
        }
    }

    pub fn scanout_tick(&mut self, snapshot: &WindowSnapshot) {
        // Opcode 55: SYS_WAIT_VBLANK
        let _ = pdx_call_checked(0, 55, 0, 0, 0);

        let top = snapshot.windows.last();
        match top {
            Some(win) => self.commit(win),
            None => self.commit_fb(),
        }

        self.vblank_counter += 1;
    }

    fn commit(&self, win: &Window) {
        // PDX Slot 5: Display Compositor Entry
        let _ = pdx_call_checked(5, win.fb_ptr, 0, 0, 0);
    }

    fn commit_fb(&self) {
        let _ = pdx_call_checked(5, self.fb_ptr, 0, 0, 0);
    }
}
