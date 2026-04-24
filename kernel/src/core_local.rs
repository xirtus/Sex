use core::sync::atomic::{AtomicU32, Ordering};
use x86_64::registers::model_specific::GsBase;
use x86_64::VirtAddr;

/// Per-core local data for SexOS.
#[repr(C)]
pub struct CoreLocal {
    pub current_pd_id: AtomicU32,
    pub core_id: u32,
    pub kernel_stack: u64,
    pub user_stack: u64,
    pub syscall_msg_ptr: u64,
    pub pdx_ring: crate::ipc_ring::SpscRing<crate::ipc::messages::Message>,
}

impl CoreLocal {
    pub unsafe fn init(core_id: u32) {
        use x86_64::structures::paging::PageTableFlags;
        use crate::memory::manager::GLOBAL_VAS;

        // Allocate a page for syscall message returns (accessible by userland)
        let msg_va = crate::memory::va_allocator::allocate_va(4096).expect("CoreLocal: VA alloc fail");
        {
            let mut vas = GLOBAL_VAS.lock();
            let vas_ref = vas.as_mut().expect("VAS not initialized");
            // Map with PKEY 15 for shared syscall buffer.
            vas_ref.map_pku_range(
                VirtAddr::new(msg_va),
                4096,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
                15 // PKEY 15
            ).expect("CoreLocal: Map fail");
        }

        let mut core_local = alloc::boxed::Box::new(CoreLocal {
            current_pd_id: AtomicU32::new(0),
            core_id,
            kernel_stack: 0,
            user_stack: 0,
            syscall_msg_ptr: msg_va,
            pdx_ring: crate::ipc_ring::SpscRing::new(),
        });
        
        // Allocate 64KB kernel stack
        let stack = alloc::vec![0u8; 65536];
        let stack_ptr = stack.as_ptr() as u64 + 65536;
        core_local.kernel_stack = stack_ptr;
        core::mem::forget(stack); // Leak stack for simplicity
        
        // Phase 32: Update TSS for Ring 3 -> Ring 0 transition safety
        crate::gdt::update_tss_rsp0(VirtAddr::new(stack_ptr));

        let ptr = alloc::boxed::Box::into_raw(core_local);
        GsBase::write(VirtAddr::from_ptr(ptr));
        x86_64::registers::model_specific::KernelGsBase::write(VirtAddr::from_ptr(ptr));
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
