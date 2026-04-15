#![no_std]
#![no_main]

use libsys::pdx::{pdx_listen, pdx_reply};
use crate::ipc::messages::MessageType;
use crate::memory::allocator::GLOBAL_ALLOCATOR;
use crate::capability::{CapabilityData, MemLendCapData};
use crate::ipc::DOMAIN_REGISTRY;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // sext: Standalone Pager / Global VAS Manager
    loop {
        // Wait for PageFault messages from the kernel
        let req = pdx_listen(0);
        
        // Handle demand paging
        match req {
            MessageType::PageFault { fault_addr, pd_id, .. } => {
                handle_page_fault(fault_addr, pd_id as u32);
            },
            _ => (),
        }
    }
}

fn handle_page_fault(fault_addr: u64, pd_id: u32) {
    // 1. Allocate a 4 KiB frame from the lock-free buddy
    if let Some(phys_addr) = GLOBAL_ALLOCATOR.alloc(0) {
        // 2. Identify the target PD
        let registry = DOMAIN_REGISTRY.get(pd_id);
        if let Some(target_pd) = registry {
            // 3. Grant a Lent-Memory capability to the faulting PD
            // In a real system, we'd also update the hardware page tables.
            target_pd.grant(CapabilityData::MemLend(MemLendCapData {
                base: fault_addr & !0xFFF,
                length: 4096,
                pku_key: target_pd.pku_key,
                permissions: 3, // R/W
            }));
            
            crate::serial_println!("sext: Paged in {:#x} for PD {} -> Phys {:#x}", 
                fault_addr, pd_id, phys_addr);
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe { core::arch::asm!("syscall", in("rax") 24); }
    }
}
