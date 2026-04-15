use core::sync::atomic::{AtomicU32, Ordering};
use x86_64::registers::model_specific::GsBase;
use x86_64::VirtAddr;

/// Per-core local data for SexOS.
#[repr(C)]
pub struct CoreLocal {
    pub current_pd_id: AtomicU32,
    pub core_id: u32,
}

impl CoreLocal {
    pub unsafe fn init(core_id: u32) {
        let core_local = alloc::boxed::Box::new(CoreLocal {
            current_pd_id: AtomicU32::new(0),
            core_id,
        });
        
        let ptr = alloc::boxed::Box::into_raw(core_local);
        GsBase::write(VirtAddr::from_ptr(ptr));
    }

    pub fn get() -> &'static CoreLocal {
        unsafe {
            &*(GsBase::read().as_ptr() as *const CoreLocal)
        }
    }

    pub fn current_pd(&self) -> u32 {
        self.current_pd_id.load(Ordering::SeqCst)
    }

    pub fn set_pd(&self, id: u32) {
        self.current_pd_id.store(id, Ordering::SeqCst);
    }

    pub fn current_pd_ref(&self) -> &'static crate::capability::ProtectionDomain {
        let id = self.current_pd();
        crate::ipc::DOMAIN_REGISTRY.get(id).expect("CoreLocal: Current PD lost")
    }
}
