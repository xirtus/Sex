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
use core::sync::atomic::{AtomicUsize, Ordering};
use core::sync::atomic::AtomicU64;
use x86_64::structures::paging::PageTableFlags;
use x86_64::VirtAddr;

const SNAPSHOT_MAX: usize = 64;

static mut SNAP_RING: [sex_pdx::SceneSnapshot; SNAPSHOT_MAX] = [sex_pdx::SceneSnapshot {
    layers_ptr: 0, layers_len: 0, cursor_x: 0, cursor_y: 0,
    is_incremental: 0, damage_rects_ptr: 0, damage_rects_len: 0,
}; SNAPSHOT_MAX];

static SNAP_IDX: AtomicUsize = AtomicUsize::new(0);
static mut SNAP_GEN: [u32; SNAPSHOT_MAX] = [0u32; SNAPSHOT_MAX];
static SYSCALL_LOG_BUDGET: AtomicU64 = AtomicU64::new(256);

// SNAPSHOT_HANDLE = (gen << 32 | idx)
// prevents stale slot reuse attacks after ring wrap
unsafe fn snapshot_ingest(src: *const sex_pdx::SceneSnapshot) -> Result<u64, u64> {
    if src as usize % core::mem::align_of::<sex_pdx::SceneSnapshot>() != 0 {
        return Err(sex_pdx::ERR_CAP_INVALID);
    }
    let idx = SNAP_IDX.fetch_add(1, Ordering::AcqRel) % SNAPSHOT_MAX;
    SNAP_GEN[idx] = SNAP_GEN[idx].wrapping_add(1);
    core::ptr::copy_nonoverlapping(src, &mut SNAP_RING[idx], 1);
    Ok(((SNAP_GEN[idx] as u64) << 32) | (idx as u64))
}

// Single-path: validate → copy → return status. One unsafe block.
fn snapshot_resolve(handle: u64, out_ptr: *mut sex_pdx::SceneSnapshot) -> u64 {
    let idx = (handle & 0xFFFF_FFFF) as usize;
    let gen = (handle >> 32) as u32;
    if idx >= SNAPSHOT_MAX || out_ptr.is_null() {
        return sex_pdx::ERR_CAP_INVALID;
    }
    unsafe {
        if SNAP_GEN[idx] != gen {
            return sex_pdx::ERR_CAP_INVALID;  // stale handle
        }
        *out_ptr = SNAP_RING[idx];
    }
    0u64
}

#[repr(C)]
pub struct DisplayInfo {
    pub virt_addr: u64,
    pub width:     u32,
    pub height:    u32,
    pub pitch:     u32, // Pixels per row
}

fn size_to_order(size: usize) -> usize {
    let pages = (size + 4095) / 4096;
    let mut order = 0;
    while order < crate::memory::allocator::MAX_ORDER && (1 << order) < pages {
        order += 1;
    }
    order
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

            regs.rsi = value;
            status
        },

        28 => { // SYSCALL_PDX_LISTEN (Phase 25)
            let slot = rdi as u32;
            let core_local = crate::core_local::CoreLocal::get();
            let current_pd = core_local.current_pd_ref();

            let (type_id, s_caller_pd, s_arg0, s_arg1, s_arg2): (u64, u64, u64, u64, u64) = {
                // 1. Check priority reply buffer (Syscall 29 source)
                // Replies always come from Slot 0 (Self) logic conceptually
                let mut replies = current_pd.incoming_replies.lock();
                if slot == 0 && !replies.is_empty() {
                    if let Some(reply) = replies.pop_front() {
                        drop(replies);
                        (0x1, 1, reply.value, 0, 0)
                    } else {
                        drop(replies);
                        (0, 0, 0, 0, 0)
                    }
                } else {
                    drop(replies);
                    
                    // 2. Resolve Slot to Capability
                    use crate::capability::CapabilityData;
                    let cap = current_pd.find_capability(slot);
                    
                    match cap.map(|c| c.data) {
                        Some(CapabilityData::InputRing) => {
                            if let Some(scancode) = crate::interrupts::INPUT_RING.dequeue() {
                                (0x201, 1, scancode as u64, 0, 0)
                            } else {
                                (0, 0, 0, 0, 0)
                            }
                        }
                        Some(CapabilityData::MessageQueue) => {
                            // Slot 0 (MessageQueue) or any other explicitly bound queue
                            unsafe {
                                if let Some(msg) = (*current_pd.message_ring).dequeue() {
                                    match msg {
                                        MessageType::IpcCall { func_id, arg0, arg1, arg2: _, caller_pd } => {
                                            (func_id, caller_pd as u64, arg0, arg1, 0)
                                        }
                                        MessageType::DisplayPrimaryFramebuffer { virt_addr, width, height, pitch } => {
                                            (0x11, 1, virt_addr, (width as u64) | ((height as u64) << 32), pitch as u64)
                                        }
                                        MessageType::RawInput(scancode) => {
                                            (0x201, 1, scancode as u64, 0, 0)
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
                        _ => (0, 0, 0, 0, 0), // Unknown/None -> Empty
                    }
                }
            };

            regs.rsi = s_caller_pd;
            regs.rdx = s_arg0;
            regs.r10 = s_arg1;
            regs.r8  = s_arg2;

            type_id
        },

        29 => { // SYSCALL_PDX_REPLY
            let target_pd_id = rdi as u32;
            let val = rsi;
            if crate::ipc::router::send_reply(target_pd_id, val).is_ok() { 0 } else { 1 }
        },

        30 => { // MAP_MEMORY
            let pa = rdi;
            let size = rsi;
            if let Some(va) = crate::memory::va_allocator::allocate_va(size as usize) {
                let mut gvas_lock = crate::memory::manager::GLOBAL_VAS.lock();
                if let Some(ref mut gvas) = *gvas_lock {
                    let flags = PageTableFlags::PRESENT 
                              | PageTableFlags::WRITABLE 
                              | PageTableFlags::USER_ACCESSIBLE;
                    if gvas.map_physical_range(VirtAddr::new(va), pa, size, flags, 0).is_ok() {
                        va
                    } else {
                        u64::MAX
                    }
                } else {
                    u64::MAX
                }
            } else {
                u64::MAX
            }
        },

        31 => { // ALLOCATE_MEMORY
            let size = rdi as usize;
            let order = size_to_order(size);
            match crate::memory::allocator::GLOBAL_ALLOCATOR.alloc(order) {
                Some(phys) => phys,
                None => u64::MAX,
            }
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

        40 => { // SYSCALL_ALLOC_SHARED_BUFFER
            let size = rdi as usize;
            let consumer_id = rsi as u8;
            let current_pd = crate::core_local::CoreLocal::get().current_pd() as u8;
            
            // Validation: current_pd != consumer_id
            if current_pd == consumer_id {
                0
            } else {
                match crate::ipc::buffer::IPC_BUFFER_MANAGER.allocate_shared_buffer(size, current_pd, consumer_id) {
                    Ok(va) => va,
                    Err(_) => 0,
                }
            }
        },

        _ => u64::MAX,
    };

    regs.rax = result;
    if SYSCALL_LOG_BUDGET
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_sub(1))
        .is_ok()
    {
        crate::serial_println!(
            "syscall.exit num={} pd_id={} status={:#x} val_rsi={:#x}",
            rax,
            crate::core_local::CoreLocal::get().current_pd(),
            result,
            regs.rsi
        );
    }
    result
}
