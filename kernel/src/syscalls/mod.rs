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

    let result = match rax {
        0 => { // SYSCALL_PDX_CALL
            let slot = rdi as u32;
            let num = rsi;
            let arg0 = rdx;
            let arg1 = r10;
            let arg2 = r8;

            if slot == 0 {
                match num {
                    0xE3 => { // PDX_GET_DISPLAY_INFO
                        let ptr = arg0 as *mut DisplayInfo;
                        if let Some(fb_res) = crate::FB_REQUEST.response() {
                            if let Some(fb) = fb_res.framebuffers().iter().next() {
                                unsafe {
                                    let fb_virt = fb.address() as u64;
                                    let hhdm = crate::HHDM_REQUEST.response().unwrap().offset;
                                    (*ptr).virt_addr = fb_virt - hhdm;
                                    (*ptr).width = fb.width as u32;
                                    (*ptr).height = fb.height as u32;
                                    (*ptr).pitch = (fb.pitch / 4) as u32;
                                }
                                0u64
                            } else { u64::MAX }
                        } else { u64::MAX }
                    }
                    69 => { // raw_print
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
                    Err(e) => {
                        crate::serial_println!("[pdx_call FAIL] slot={} opcode={:#x} err={}", slot, num, e);
                        u64::MAX
                    }
                }
            }
        },

        28 => { // SYSCALL_PDX_LISTEN (Phase 25)
            let core_local = crate::core_local::CoreLocal::get();
            let current_pd = core_local.current_pd_ref();

            // 1. Check priority reply buffer (Syscall 29 source)
            let mut replies = current_pd.incoming_replies.lock();
            if let Some(reply) = replies.pop_front() {
                regs.rax = 0x1; // type_id = Response
                regs.rsi = 1;   // caller_pd = kernel
                regs.rdx = reply.value;
                regs.r10 = 0;
                regs.r8  = 0;
                return 0x1;
            }
            drop(replies);

            // 2. Check standard message ring
            // Register layout: rax=type_id (0=EMPTY), rsi=caller_pd, rdx=arg0, r10=arg1, r8=arg2
            unsafe {
                if let Some(msg) = (*current_pd.message_ring).dequeue() {
                    match msg {
                        MessageType::IpcCall { func_id, arg0, arg1, arg2: _, caller_pd } => {
                            regs.rsi = caller_pd as u64;
                            regs.rdx = arg0;
                            regs.r10 = arg1;
                            regs.r8  = 0;
                            func_id // rax = type_id (opcode, e.g. 0xDE)
                        }
                        MessageType::DisplayPrimaryFramebuffer { virt_addr, width, height, pitch } => {
                            regs.rsi = 1;
                            regs.rdx = virt_addr;
                            regs.r10 = (width as u64) | ((height as u64) << 32);
                            regs.r8  = pitch as u64;
                            0x11 // rax = type_id
                        }
                        _ => {
                            let type_id: u64 = match msg {
                                MessageType::WindowCreate       => 0xDE,
                                MessageType::CompositorCommit   => 0xDD,
                                MessageType::SetWindowRoundness => 0xDF,
                                MessageType::SetWindowBlur      => 0xE0,
                                MessageType::GetDisplayInfo     => 0xE3,
                                _                               => 0xFF,
                            };
                            regs.rsi = 1;
                            regs.rdx = 0; regs.r10 = 0; regs.r8 = 0;
                            type_id // rax
                        }
                    }
                } else {
                    0 // rax = 0 = EMPTY
                }
            }
        },

        29 => { // SYSCALL_PDX_REPLY
            let target_pd_id = rdi as u32;
            let val = rsi;
            if crate::ipc::router::send_reply(target_pd_id, val).is_ok() { 0 } else { 1 }
        },

        32 => { // SYSCALL_YIELD
            crate::scheduler::yield_now();
            0
        },

        _ => u64::MAX,
    };

    regs.rax = result;
    result
}
