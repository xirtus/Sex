pub mod spawn;
pub mod fs;
pub mod net;
pub mod storage;
pub mod pipe;
pub mod fork;
pub mod translator;
pub mod store;

use crate::interrupts::SyscallRegs;
use crate::ipc::messages::MessageType;
use crate::ipc::DOMAIN_REGISTRY;

#[repr(C)]
pub struct DisplayInfo {
    pub virt_addr: u64,
    pub width:     u32,
    pub height:    u32,
    pub pitch:     u32, // Pixels per row
}

pub fn dispatch(regs: &mut SyscallRegs) -> u64 {
    let rax = regs.rax;
    let rdi = regs.rdi;
    let rsi = regs.rsi;
    let rdx = regs.rdx;
    let r10 = regs.r10;
    let r8  = regs.r8;

    // Bug 3 fix: dispatch() return value is discarded by syscall_entry's `pop rax`
    // (which restores the original syscall number). Result must be written to regs.rax
    // so that `pop rax` loads the correct value for the caller.
    let result = match rax {
        27 => { // pdx_call(slot, num, arg0, arg1, arg2)
            let slot = rdi as u32;
            let num = rsi;
            let arg0 = rdx;
            let arg1 = r10;
            let arg2 = r8;

            if slot == 0 {
                match num {
                    0x03 => { // PDX_GET_DISPLAY_INFO
                        let ptr = arg0 as *mut DisplayInfo;
                        if let Some(fb_res) = crate::FB_REQUEST.response() {
                            if let Some(fb) = fb_res.framebuffers().iter().next() {
                                unsafe {
                                    (*ptr).virt_addr = fb.address() as u64;
                                    (*ptr).width = fb.width as u32;
                                    (*ptr).height = fb.height as u32;
                                    (*ptr).pitch = (fb.pitch / 4) as u32;
                                }
                                0u64
                            } else { u64::MAX }
                        } else { u64::MAX }
                    }
                    69 => { // raw_print(ptr, len)
                        let ptr = arg0 as *const u8;
                        let len = arg1 as usize;
                        unsafe {
                            let slice = core::slice::from_raw_parts(ptr, len);
                            for &b in slice {
                                use x86_64::instructions::port::Port;
                                let mut port = Port::new(0x3f8);
                                port.write(b);
                            }
                        }
                        0u64
                    }
                    _ => u64::MAX,
                }
            } else {
                match crate::ipc::safe_pdx_call(slot, num, arg0, arg1, arg2) {
                    Ok(val) => val,
                    Err(_) => u64::MAX,
                }
            }
        },

        28 => { // pdx_listen(slot)
            let core_local = crate::core_local::CoreLocal::get();
            let current_pd = core_local.current_pd_ref();
            unsafe {
                if let Some(msg) = (*current_pd.message_ring).dequeue() {
                    match msg {
                        MessageType::IpcCall { func_id, arg0, arg1, arg2, caller_pd } => {
                            regs.rdi = caller_pd as u64;
                            regs.rsi = func_id;
                            regs.rdx = arg0;
                            regs.r10 = arg1;
                            regs.r8  = arg2;
                            func_id
                        }
                        MessageType::IpcReply(val) => {
                            // ABI: RSI=0xF for Reply, RDX=value
                            regs.rdi = 0; // No caller for reply
                            regs.rsi = 0xF; 
                            regs.rdx = val;
                            regs.r10 = 0;
                            regs.r8  = 0;
                            0xF // Reply status
                        }
                        _ => {
                            regs.rdi = 0; regs.rsi = 0; regs.rdx = 0; regs.r10 = 0; regs.r8 = 0;
                            0u64
                        }
                    }
                } else {
                    regs.rdi = 0; regs.rsi = 0; regs.rdx = 0; regs.r10 = 0; regs.r8 = 0;
                    0u64
                }
            }
        },

        29 => { // pdx_reply(target_pd, val)
            let target_pd_id = rdi as u32;
            let val = rsi;
            if let Some(target_pd) = DOMAIN_REGISTRY.get(target_pd_id) {
                let msg = MessageType::IpcReply(val);
                unsafe {
                    let _ = (*target_pd.message_ring).enqueue(msg);
                }
                0u64
            } else {
                u64::MAX
            }
        },

        32 | 100 => { // sys_yield
            0u64
        },

        35 => { // sys_get_input
            if let Some(byte) = crate::interrupts::INPUT_RING.dequeue() {
                byte as u64
            } else {
                u64::MAX // Return MAX to indicate no pending keystrokes
            }
        },

        69 => { // serial_print(ptr, len) - Direct legacy path
             let ptr = rdi as *const u8;
             let len = rsi as usize;
             unsafe {
                let slice = core::slice::from_raw_parts(ptr, len);
                for &b in slice {
                    use x86_64::instructions::port::Port;
                    let mut port = Port::new(0x3f8);
                    port.write(b);
                }
             }
             0u64
        },

        24 => { // sys_yield
             crate::scheduler::yield_now();
             0u64
        },

        _ => u64::MAX,
    };

    regs.rax = result;
    result
}
