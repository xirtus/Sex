use crate::serial_println;
use crate::memory::GlobalVas;
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::VirtAddr;
use x86_64::structures::paging::PageTableFlags;

/// The Pager Server's state.
/// In a real system, this would manage physical frame pools and swap.
pub struct PagerState {
    // We'll need access to the Global VAS to perform mappings.
    // In a real user-space server, this would be done via a privileged 
    // system call or by owning the Page Table capabilities.
}

/// The Pager Server's entry point.
pub extern "C" fn pager_entry(arg: u64) -> u64 {
    // This is the "upcall" or "forwarded" entry point.
    // arg might be the faulting address or a request type.
    
    let fault_addr = VirtAddr::new(arg);
    serial_println!("PAGER: Received request/fault for address: {:?}", fault_addr);

    // Demonstration of "Demand Paging" logic:
    // 1. Validate the faulting address.
    // 2. Allocate a physical frame (or large page).
    // 3. Map it into the Global VAS.
    
    // For this demo, we'll just return a success code.
    0
}

/// A request to the Pager to map a memory range.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MapRequest {
    pub start: u64,
    pub size: u64,
    pub pku_key: u8,
    pub writable: bool,
}

/// The PDX interface for the Pager.
pub fn handle_map_request(req: MapRequest) -> u64 {
    serial_println!("PAGER: Mapping range {:#x} (size: {}) with Key {}", 
        req.start, req.size, req.pku_key);
    
    // In a real system, the Pager would call the kernel or use its capabilities 
    // to update the page tables.
    0
}
