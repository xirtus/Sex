use crate::xipc::messages::MessageType;
use crate::memory::allocator::GLOBAL_ALLOCATOR;
use x86_64::structures::paging::PageTableFlags;

/// The main entry point for the sext server (SASOS Pager).
pub fn sext_main() {
    let pd = crate::core_local::CoreLocal::get().current_pd_ref();
    let sexc_state_lock = pd.sexc_state.lock();
    let state = sexc_state_lock.as_ref().expect("sext: No state");
    let ring = &state.control_ring;

    loop {
        while ring.is_empty() {
            crate::scheduler::park_current_thread();
        }

        if let Ok(msg) = ring.dequeue() {
            match msg {
                MessageType::PageFault { fault_addr, error_code, pd_id } => {
                    handle_page_fault(fault_addr, error_code, pd_id);
                }
                _ => {}
            }
        }
    }
}

fn handle_page_fault(addr: u64, _err: u32, target_pd_id: u64) {
    // 1. Allocate frame via Buddy System
    let mut allocator = GLOBAL_ALLOCATOR.lock();
    if let Some(phys) = allocator.alloc(0) { // 4KiB
        // 2. Map into target PD's virtual address space (SAS Model)
        let mut gvas = crate::memory::GLOBAL_VAS.lock();
        if let Some(ref mut vas) = *gvas {
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
            let target_pku_key = (target_pd_id % 15) as u8 + 1;
            
            if vas.map_pku_range(x86_64::VirtAddr::new(addr), 4096, flags, target_pku_key).is_ok() {
                // 3. Resume the faulting task
                crate::scheduler::unpark_thread(target_pd_id as u32);
            }
        }
    }
}
