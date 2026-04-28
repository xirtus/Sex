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
    let r9  = regs.r9;

    let result = match rax {
        0 => { // SYSCALL_PDX_CALL
            let slot = rdi as u32;
            let num = rsi;
            let arg0 = rdx;
            let arg1 = r10;
            let arg2 = r8;
            let resp_ptr = r9 as *mut sex_pdx::PdxResponse;

            let (status, value) = if slot == 0 {
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
                                (0u64, 0u64)
                            } else { (sex_pdx::ERR_CAP_INVALID, 0) }
                        } else { (sex_pdx::ERR_CAP_INVALID, 0) }
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
                        (0u64, 0u64)
                    }
                    0x12 => { // pdx_get_init_arg
                        let core_local = crate::core_local::CoreLocal::get();
                        let sched = &crate::scheduler::SCHEDULERS[core_local.core_id as usize];
                        let current_task_ptr = sched.current_task.load(core::sync::atomic::Ordering::Acquire);
                        if !current_task_ptr.is_null() {
                            let task = unsafe { &mut *current_task_ptr };
                            // Phase 1.5: Pure structural ownership via move semantics.
                            // No identity/PD-ID checks here.
                            if let Some(init_arg) = task.ext_init.take() {
                                let lease = init_arg.display_lease;
                                // Pack lease: [8:bus][8:dev][8:func][16:vendor][16:device]
                                let packed = (lease.bus as u64) 
                                           | ((lease.dev as u64) << 8)
                                           | ((lease.func as u64) << 16)
                                           | ((lease.vendor_id as u64) << 24)
                                           | ((lease.device_id as u64) << 40);
                                (0u64, packed)
                            } else { (sex_pdx::ERR_CAP_INVALID, 0) }
                        } else { (sex_pdx::ERR_CAP_INVALID, 0) }
                    }
                    0xBB => { // Phase 2: FRAME_PRESENT debug hook
                        crate::serial_println!("PD1: FRAME_PRESENT event");
                        (0u64, 0u64)
                    }
                    _ => (sex_pdx::ERR_CAP_INVALID, 0),
                }
            } else {
                match crate::ipc::safe_pdx_call(slot, num, arg0, arg1, arg2) {
                    Ok(val) => (0u64, val),
                    Err(e)  => (e, 0u64),
                }
            };

            if !resp_ptr.is_null() {
                unsafe {
                    (*resp_ptr).status = status;
                    (*resp_ptr).value  = value;
                }
            }
            status
        },

        28 => { // SYSCALL_PDX_LISTEN (Phase 25)
            let resp_ptr = r9 as *mut sex_pdx::PdxListenResult;
            let core_local = crate::core_local::CoreLocal::get();
            let current_pd = core_local.current_pd_ref();

            let (type_id, s_caller_pd, s_arg0, s_arg1, s_arg2): (u64, u64, u64, u64, u64) = {
                // 1. Check priority reply buffer (Syscall 29 source)
                let mut replies = current_pd.incoming_replies.lock();
                if let Some(reply) = replies.pop_front() {
                    drop(replies);
                    (0x1, 1, reply.value, 0, 0)
                } else {
                    drop(replies);
                    // 2. Check standard message ring
                    unsafe {
                        if let Some(msg) = (*current_pd.message_ring).dequeue() {
                            match msg {
                                MessageType::IpcCall { func_id, arg0, arg1, arg2: _, caller_pd } => {
                                    (func_id, caller_pd as u64, arg0, arg1, 0)
                                }
                                MessageType::DisplayPrimaryFramebuffer { virt_addr, width, height, pitch } => {
                                    (0x11, 1, virt_addr, (width as u64) | ((height as u64) << 32), pitch as u64)
                                }
                                _ => {
                                    let tid: u64 = match msg {
                                        MessageType::WindowCreate       => 0xDE,
                                        MessageType::CompositorCommit   => 0xDD,
                                        MessageType::SetWindowRoundness => 0xDF,
                                        MessageType::SetWindowBlur      => 0xE0,
                                        MessageType::GetDisplayInfo     => 0xE3,
                                        _                               => 0xFF,
                                    };
                                    (tid, 1, 0, 0, 0)
                                }
                            }
                        } else {
                            (0, 0, 0, 0, 0) // EMPTY
                        }
                    }
                }
            };

            regs.rsi = s_caller_pd;
            regs.rdx = s_arg0;
            regs.r10 = s_arg1;
            regs.r8  = s_arg2;

            if !resp_ptr.is_null() {
                unsafe {
                    if type_id != 0 {
                        (*resp_ptr).has_message = 1;
                        (*resp_ptr).type_id    = type_id;
                        (*resp_ptr).caller_pd  = s_caller_pd;
                        (*resp_ptr).arg0       = s_arg0;
                        (*resp_ptr).arg1       = s_arg1;
                        (*resp_ptr).arg2       = s_arg2;
                    } else {
                        (*resp_ptr).has_message = 0;
                    }
                }
            }

            type_id
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

        42 => { // SYS_SET_STATE
            let state = rdx as u8;
            let core_local = crate::core_local::CoreLocal::get();
            let pd_id = core_local.current_pd_ref().id;
            if state == crate::ipc::state::SVC_STATE_LISTENING {
                crate::ipc::state::set_service_listening(pd_id);
            }
            0
        },

        _ => u64::MAX,
    };

    regs.rax = result;
    result
}
