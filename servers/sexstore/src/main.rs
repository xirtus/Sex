#![feature(alloc_error_handler)]
extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::boxed::Box;
#![no_std]
#![no_main]

use sex_pdx::{pdx_listen, pdx_reply, pdx_call, MessageType, StoreProtocol, PageHandover};
use core::sync::atomic::{AtomicU64, Ordering};

/// sexstore: Standalone Package Manager and Self-Hosting Daemon.
/// Phase 20: StoreProtocol for zero-copy package loading.

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut store = Store::new();
    loop {
        sys_park();

        let req = pdx_listen(0);
        let msg = unsafe { *(req.arg0 as *const MessageType) };

        match msg {
            MessageType::Store(proto) => {
                let reply = store.handle_request(proto);
                pdx_reply(req.caller_pd, reply);
            },
            _ => {
                pdx_reply(req.caller_pd, u64::MAX);
            }
        }
    }
}

pub fn sys_park() {
    unsafe {
        core::arch::asm!("syscall", in("rax") 24);
    }
}

struct Store {
    package_loads: AtomicU64,
    zero_copy_handovers: AtomicU64,
}

impl Store {
    fn new() -> Self {
        Self {
            package_loads: AtomicU64::new(0),
            zero_copy_handovers: AtomicU64::new(0),
        }
    }

    fn handle_request(&mut self, proto: StoreProtocol) -> u64 {
        match proto {
            StoreProtocol::FetchPackage { name: _ } => {
                // Mock implementation: return a dummy PageHandover
                self.package_loads.fetch_add(1, Ordering::Relaxed);
                let page = self.fetch_from_ramfs();
                &page as *const _ as u64
            },
            StoreProtocol::CacheBinary { name: _, image } => {
                self.zero_copy_handovers.fetch_add(1, Ordering::Relaxed);
                self.cache_to_ramfs(image);
                0
            },
            StoreProtocol::Stats => {
                self.package_loads.load(Ordering::Relaxed)
            },
            _ => u64::MAX,
        }
    }

    fn fetch_from_ramfs(&self) -> PageHandover {
        // In a real implementation, this would interact with a RamFS driver.
        // For now, we request a page from the kernel and return it.
        let pfn = pdx_call(1, 12, 0, 0); // RESOLVE_PHYS
        PageHandover { pfn, pku_key: 6 } // sexstore domain key
    }

    fn cache_to_ramfs(&self, _image: PageHandover) {
        // In a real implementation, this would write the page to the RamFS.
    }
}


#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { sys_park(); }
}
