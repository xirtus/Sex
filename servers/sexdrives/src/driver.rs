#![no_std]
#![no_main]

use libsys::pdx::{pdx_listen, pdx_reply, pdx_call};
use libsys::messages::MessageType;

/// sexdrives: Standalone Storage Driver (NVMe/AHCI)
/// IPCtax: Pure PDX implementation, NO globals, NO busy-wait.
/// 100% Zero-Copy DMA via lent-memory capabilities.

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Initialize Controller (Normally would resolve PCI cap via PDX to kernel)
    // For this prototype, we assume BAR0 is already mapped in SAS or lent.
    
    loop {
        // Wait-free park until work arrives in control ring
        unsafe { core::arch::asm!("syscall", in("rax") 24 /* SYS_PARK */); }

        let req = pdx_listen(0);
        let msg = unsafe { *(req.arg0 as *const MessageType) };

        match msg {
            MessageType::DmaCall { command, offset, size, buffer_cap, device_cap } => {
                let status = submit_nvme_command(command, offset, size, buffer_cap);
                let reply = MessageType::DmaReply { status, size };
                pdx_reply(req.caller_pd, &reply as *const _ as u64);
            },
            _ => {
                pdx_reply(req.caller_pd, u64::MAX);
            }
        }
    }
}

fn submit_nvme_command(cmd: u32, lba: u64, size: u64, buffer_cap: u32) -> i64 {
    // 1. Resolve Physical Address from Lent Capability
    // IPCtax: zero-copy DMA requires the hardware-facing phys addr.
    let phys_addr = pdx_call(1 /* kernel */, 12 /* RESOLVE_PHYS */, buffer_cap as u64, 0);
    if phys_addr == 0 { return -1; }

    // 2. Write to NVMe Submission Queue (Simplified)
    // Assume BAR0 at hardcoded SAS offset for prototype demo.
    let bar0 = 0x_A000_0000_0000; 
    let sq_base = (bar0 + 0x2000) as *mut u32; // IO SQ
    
    unsafe {
        sq_base.add(0).write_volatile(if cmd == 1 { 0x02 } else { 0x01 }); // Opcode
        sq_base.add(1).write_volatile(1); // NSID
        sq_base.add(6).write_volatile(phys_addr as u32); // PRP1
        sq_base.add(7).write_volatile((phys_addr >> 32) as u32);
        sq_base.add(10).write_volatile(lba as u32);
        sq_base.add(11).write_volatile((lba >> 32) as u32);
        
        // 3. Ring Doorbell
        let doorbell = (bar0 + 0x1000 + 8) as *mut u32;
        doorbell.write_volatile(1);
    }

    // 4. Return success (In real driver, we'd wait for MSI-X/CQ)
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("syscall", in("rax") 24); } }
}
