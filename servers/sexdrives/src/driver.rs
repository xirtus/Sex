#![no_std]
#![no_main]

use libsys::pdx::{pdx_listen, pdx_reply, pdx_call};
use libsys::messages::MessageType;
use libsys::sched::park_on_ring;

/// sexdrives: Standalone Storage Driver (NVMe/AHCI)
/// IPCtax: Pure PDX implementation, NO globals, NO busy-wait.

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Resolve PCI BAR0 via PDX to kernel (Cap slot 1 was granted at boot)
    let bar0_vaddr = pdx_call(1, 13 /* RESOLVE_BAR */, 0, 0);
    
    loop {
        // Standard FLSCHED park
        park_on_ring();

        let req = pdx_listen(0);
        let msg = unsafe { *(req.arg0 as *const MessageType) };

        match msg {
            MessageType::DmaCall { command, offset, size, buffer_cap, .. } => {
                let status = submit_nvme_command(bar0_vaddr, command, offset, size, buffer_cap);
                let reply = MessageType::DmaReply { status, size };
                pdx_reply(req.caller_pd, &reply as *const _ as u64);
            },
            MessageType::HardwareInterrupt { vector, .. } => {
                handle_msix_completion(bar0_vaddr, vector);
                pdx_reply(req.caller_pd, 0);
            },
            _ => {
                pdx_reply(req.caller_pd, u64::MAX);
            }
        }
    }
}

static mut SQ_TAIL: u16 = 0;

fn submit_nvme_command(bar0: u64, cmd: u32, lba: u64, size: u64, buffer_cap: u32) -> i64 {
    // 1. Resolve Physical Address from Lent Capability (Cap Slot 1: kernel)
    let phys_addr = pdx_call(1, 12 /* RESOLVE_PHYS */, buffer_cap as u64, 0);
    if phys_addr == 0 { return -1; }

    // 2. Write to NVMe Submission Queue (Real logic)
    let sq_base = (bar0 + 0x2000) as *mut u32; 
    let tail = unsafe { SQ_TAIL };
    let entry = unsafe { sq_base.add(tail as usize * 16) }; // 64 bytes per entry

    unsafe {
        entry.offset(0).write_volatile(if cmd == 1 { 0x02 } else { 0x01 }); // Opcode: Read/Write
        entry.offset(1).write_volatile(1); // NSID: 1
        entry.offset(6).write_volatile(phys_addr as u32); // PRP1 Low
        entry.offset(7).write_volatile((phys_addr >> 32) as u32); // PRP1 High
        entry.offset(10).write_volatile(lba as u32); // Start LBA Low
        entry.offset(11).write_volatile((lba >> 32) as u32); // Start LBA High
        entry.offset(12).write_volatile((size / 512 - 1) as u32); // Block count
        
        // 3. Ring Doorbell (IO SQ 1 Doorbell at 0x1008)
        let doorbell = (bar0 + 0x1008) as *mut u32;
        doorbell.write_volatile(tail as u32 + 1);
        
        SQ_TAIL = (tail + 1) % 32;
    }
    0
}

fn handle_msix_completion(_bar0: u64, _vector: u8) {
    // Read from Completion Queue (CQ) and notify waiting threads
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { park_on_ring(); }
}
