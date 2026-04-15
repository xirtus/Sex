#![no_std]
#![no_main]

use libsys::pdx::{pdx_listen, pdx_reply, pdx_call};
use libsys::messages::MessageType;

/// sexdrives: Standalone Storage Driver (NVMe/AHCI)
/// IPCtax: Pure PDX implementation, NO globals, NO busy-wait.
/// 100% Zero-Copy DMA via lent-memory capabilities.

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Phase 5: Hardware & sexdrives.
    // Initialize hardware using capabilities granted at spawn.
    // E.g., request PCI MMIO capability resolution from kernel via PDX.
    
    loop {
        // Blocks with FLSCHED::park() on the SPSC control ring
        unsafe { core::arch::asm!("syscall", in("rax") 24 /* SYS_PARK */); }

        let req = pdx_listen(0);
        let msg = unsafe { *(req.arg0 as *const MessageType) };

        match msg {
            MessageType::DmaCall { command, offset, size, buffer_cap, device_cap } => {
                let status = handle_storage_request(command, offset, size, buffer_cap, device_cap);
                let reply = MessageType::DmaReply { status, size };
                pdx_reply(req.caller_pd, &reply as *const _ as u64);
            },
            _ => {
                pdx_reply(req.caller_pd, u64::MAX);
            }
        }
    }
}

fn handle_storage_request(cmd: u32, offset: u64, size: u64, buffer_cap: u32, device_cap: u32) -> i64 {
    // In a real implementation:
    // 1. Use buffer_cap to securely resolve the physical address of the lent buffer via PDX.
    //    let phys_addr = pdx_call(1 /* kernel */, RESOLVE_CAP, buffer_cap, 0);
    // 2. Submit the NVMe/AHCI command using the resolved physical address (Zero-copy DMA).
    // 3. Park thread and wait for MSI-X interrupt completion on a dedicated interrupt ring.

    match cmd {
        1 => { // FS_READ
            // Simulated NVMe DMA Read
            0
        },
        2 => { // FS_WRITE
            // Simulated NVMe DMA Write
            0
        },
        _ => -1,
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("syscall", in("rax") 24); } }
}
