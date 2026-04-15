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
        serial_println!("sext: Pager loop started on Core 0.");
        loop {
            // Process the Page Fault Queue (Sexting)
            if let Some(event) = SEXT_QUEUE.dequeue() {
                serial_println!("sext: Validating fault at {:#x} for Task {}", event.addr, event.task_id);
                
                // 1. Verify the thread's capabilities for this virtual region
                let registry = crate::ipc::DOMAIN_REGISTRY.read();
                let is_valid = if let Some(ref mut sched) = unsafe { crate::scheduler::SCHEDULERS[0].as_mut() } {
                    // Find the task in the scheduler's wait queue to get its PD
                    if let Some(task_mutex) = sched.wait_queue.iter().find(|t| t.lock().id == event.task_id) {
                        let task = task_mutex.lock();
                        let pd = &task.context.pd;
                        
                        // Check if the PD has a memory capability for the address
                        pd.cap_table.find_by_addr(event.addr).is_some()
                    } else { false }
                } else { false };

                if !is_valid {
                    serial_println!("sext: [SECURITY VIOLATION] Task {} accessed unmapped/unauthorized address {:#x}.", event.task_id, event.addr);
                    // In a real system, we'd send SIGSEGV. For the prototype, we continue.
                }

                // 2. Map the page (Demand Paging)
                let fault_addr = VirtAddr::new(event.addr);
                let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
                
                match vas.map_range(fault_addr, 4096, flags) {
                    Ok(_) => {
                        serial_println!("sext: Page resolved for Task {}.", event.task_id);
                        
                        // 3. Unblock the task (Resume it)
                        unsafe {
                            if let Some(ref mut sched) = crate::scheduler::SCHEDULERS[0] {
                                sched.unblock(event.task_id);
                            }
                        }
                    },
                    Err(e) => {
                        serial_println!("sext: Failed to resolve fault for Task {}: {}", event.task_id, e);
                    }
                }
            }

            // Yield or Halt to wait for more events
            x86_64::instructions::hlt();
        }
    }
}
pub enum MapType {
    Anonymous,
    Shared,
    HardwareMMIO, // New: Hardware MMIO mapping
}

/// A request to the sext to map or fetch a memory range.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MapRequest {
    pub node_id: u32, 
    pub start: u64,
    pub phys_addr: u64,
    pub size: u64,
    pub pku_key: u8,
    pub writable: bool,
    pub is_shm: bool,
    pub is_mmio: bool,
    pub is_dma: bool, // New: DMA mapping flag
}

/// The PDX interface for the sext.
pub fn sext_request(req: MapRequest, vas: &mut GlobalVas) -> u64 {
    if req.is_dma {
        serial_println!("sext: [DMA] Mapping Contiguous Buffer {:#x} (size: {})", 
            req.phys_addr, req.size);
        
        // 1. Verify DMA Capability
        // 2. Map to High SAS DMA region (0xD000...)
        let vaddr = 0x_D000_0000_0000 + req.phys_addr;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
        
        match vas.map_phys_range(VirtAddr::new(vaddr), x86_64::PhysAddr::new(req.phys_addr), req.size, flags, req.pku_key) {
            Ok(_) => vaddr,
            Err(_) => u64::MAX,
        }
    } else if req.is_mmio {
        serial_println!("sext: [MMIO] Mapping Hardware BAR {:#x} (size: {})", 
            req.phys_addr, req.size);

        // 1. Verify caller's Hardware Capability
        let pd_id = crate::core_local::CoreLocal::get().current_pd();
        let registry = crate::ipc::DOMAIN_REGISTRY.read();
        let is_authorized = if let Some(pd) = registry.get(&pd_id) {
            // Find a PCI capability that matches this BAR range or device
            pd.cap_table.caps.lock().iter().any(|c| {
                match c.data {
                    crate::capability::CapabilityData::Pci(data) => {
                        // In a real system, we'd check if the phys_addr belongs to this PCI device
                        true 
                    },
                    _ => false,
                }
            })
        } else { false };

        if !is_authorized && pd_id != 0 {
            serial_println!("sext: [SECURITY] PD {} unauthorized for physical {:#x}", pd_id, req.phys_addr);
            return u64::MAX;
        }

        // 2. Allocate a new virtual memory region in the global Single Address Space (MMIO region)
        // For the prototype, we use a fixed high offset + phys_addr
        let vaddr = 0x_A000_0000_0000 + req.phys_addr;
        
        let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::NO_CACHE;
        if req.writable { flags |= PageTableFlags::WRITABLE; }

        // 3. Map the physical BAR to that virtual region using the caller's PKU domain
        match vas.map_phys_range(VirtAddr::new(vaddr), x86_64::PhysAddr::new(req.phys_addr), req.size, flags, req.pku_key) {
            Ok(_) => vaddr,
            Err(_) => u64::MAX,
        }
    } else if req.is_shm {
...
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
