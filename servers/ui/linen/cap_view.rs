use crate::ucgm_view::ViewModel;
use alloc::sync::Arc;
use spin::RwLock;

pub struct CapabilityView {
    pub projection: Arc<RwLock<ViewModel>>,
}

impl CapabilityView {
    pub fn new() -> Self {
        Self {
            projection: Arc::new(RwLock::new(ViewModel::default())),
        }
    }

    pub fn query_system(&self) -> (u64, u64) {
        // Wrapper for SYSCALL_QUERY_UCGM (33)
        // returns (service_count, active_domains)
        unsafe {
            let res: u64;
            core::arch::asm!(
                "syscall",
                in("rax") 33u64,
                out("rax") res,
                options(nostack)
            );
            (res & 0xFFFF, res >> 16)
        }
    }
}
