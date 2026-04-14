use crate::serial_println;
use crate::memory::GlobalVas;
use crate::interrupts::SEXT_QUEUE;
use x86_64::VirtAddr;
use x86_64::structures::paging::PageTableFlags;

/// Real sext Pager (Demand Sexting).
/// This server runs as a background task, processing page faults 
/// and mapping them into the Global SAS.

pub struct SextPager {
    pub local_node_id: u32,
}

impl SextPager {
    /// The main event loop for the pager.
    pub fn run_loop(&self, vas: &mut GlobalVas) {
        serial_println!("sext: Pager loop started.");
        loop {
            if let Some(event) = SEXT_QUEUE.dequeue() {
                serial_println!("sext: Handling Fault at {:#x}", event.addr);
                
                // 1. Allocate a fresh physical frame
                let fault_addr = VirtAddr::new(event.addr);
                let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
                
                // 2. Map the page (Demand Paging)
                match vas.map_range(fault_addr, 4096, flags) {
                    Ok(_) => {
                        serial_println!("sext: Resolved fault for {:#x} (Task: {})", 
                            event.addr, event.task_id);
                        
                        // 3. Unblock the faulted task
                        unsafe {
                            if let Some(ref mut sched) = crate::scheduler::SCHEDULERS[0] {
                                sched.unblock(event.task_id);
                            }
                        }
                    },
                    Err(e) => serial_println!("sext: Failed to resolve fault: {}", e),
                }
            }
            // In a real system, this would be a "wait for notification" block
            x86_64::instructions::hlt();
        }
    }
}

/// A request to the sext to map or fetch a memory range.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MapRequest {
    pub node_id: u32, // The node that owns the physical memory
    pub start: u64,
    pub size: u64,
    pub pku_key: u8,
    pub writable: bool,
    pub is_shm: bool, // Wayland Shared Memory flag
}

/// The PDX interface for the sext.
pub fn sext_request(req: MapRequest, vas: &mut GlobalVas) -> u64 {
    if req.is_shm {
        serial_println!("sext: [SHM] Creating Shared Memory Segment at {:#x}", req.start);
        // In SASOS, SHM is just granting a PKU key to multiple domains.
        let flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
        let mut final_flags = flags;
        if req.writable { final_flags |= PageTableFlags::WRITABLE; }
        
        match vas.map_pku_range(VirtAddr::new(req.start), req.size, final_flags, req.pku_key) {
            Ok(_) => 0xC0DE_BEEF,
            Err(_) => u64::MAX,
        }
    } else if req.node_id != 1 {
        serial_println!("sext: [DSM] Remote Page Fault for Node {} (addr: {:#x})", 
            req.node_id, req.start);
        fetch_remote_page(req.node_id, req.start)
    } else {
        serial_println!("sext: Mapping local range {:#x} (size: {}) with Key {}", 
            req.start, req.size, req.pku_key);
        
        let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
        if req.writable { flags |= PageTableFlags::WRITABLE; }
        
        match vas.map_pku_range(VirtAddr::new(req.start), req.size, flags, req.pku_key) {
            Ok(_) => 0,
            Err(_) => u64::MAX,
        }
    }
}

use crate::servers::sexnet::{RDMA_QUEUE, RdmaDescriptor, RdmaOp};

fn fetch_remote_page(node_id: u32, addr: u64) -> u64 {
    serial_println!("sext: [DSM] Fetching Page {:#x} from Node {} via RDMA Engine.", 
        addr, node_id);

    // 1. Construct RDMA Read Descriptor
    let desc = RdmaDescriptor {
        op: RdmaOp::Read,
        target_node: node_id,
        local_phys: addr, // In a SASOS, we use the same address
        remote_vaddr: addr,
        length: 4096,
        completion_flag: core::sync::atomic::AtomicBool::new(false),
    };

    // 2. Enqueue to SexNet RDMA Engine
    if RDMA_QUEUE.enqueue(desc).is_ok() {
        serial_println!("sext: [DSM] RDMA Read request enqueued.");
        // 3. The pager thread would normally yield here until the completion_flag is set.
        // For the prototype, we return success.
        return 0;
    }

    u64::MAX
}
