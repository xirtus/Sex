use core::sync::atomic::{AtomicU32, Ordering};
use x86_64::registers::model_specific::GsBase;
use x86_64::VirtAddr;

/// Per-core local data for SexOS.
/// This structure is pointed to by the GS segment base.
#[repr(C)]
pub struct CoreLocal {
    /// The ID of the Protection Domain currently running on this core.
    pub current_pd_id: AtomicU32,
    /// The ID of the core (0 to 127).
    pub core_id: u32,
}

impl CoreLocal {
    /// Initializes the core-local data for the current CPU.
    pub unsafe fn init(core_id: u32) {
        let core_local = alloc::boxed::Box::new(CoreLocal {
            current_pd_id: AtomicU32::new(0),
            core_id,
        });
        
        let ptr = alloc::boxed::Box::into_raw(core_local);
        GsBase::write(VirtAddr::from_ptr(ptr));
    }

    /// Returns a reference to the current core's local data.
    pub fn get() -> &'static CoreLocal {
        unsafe {
            &*(GsBase::read().as_ptr() as *const CoreLocal)
        }
    }

    /// Safely gets the current PD ID.
    pub fn current_pd(&self) -> u32 {
        self.current_pd_id.load(Ordering::SeqCst)
    }

    /// Sets the current PD ID (used by scheduler).
    pub fn set_pd(&self, id: u32) {
        self.current_pd_id.store(id, Ordering::SeqCst);
    }
}
