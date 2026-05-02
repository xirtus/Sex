use core::sync::atomic::{AtomicU32, Ordering, AtomicPtr, AtomicUsize, AtomicU64};
use crate::capability::ProtectionDomain;
use crate::ipc_ring::RingBuffer;
use crate::serial_println;
use core::ptr;

/// The execution state of a vThread (Task).
#[repr(C)]
pub struct TaskContext {
    pub r15: u64, pub r14: u64, pub r13: u64, pub r12: u64,
    pub r11: u64, pub r10: u64, pub r9: u64, pub r8: u64,
    pub rdi: u64, pub rsi: u64, pub rbp: u64, pub rdx: u64,
    pub rcx: u64, pub rbx: u64, pub rax: u64,
    pub dummy_error_code: u64,
    pub pkru: u64,
    pub pd_id: u64,
    pub rip: u64, pub cs: u64, pub rflags: u64, pub rsp: u64, pub ss: u64,
    pub pd_ptr: *const ProtectionDomain,
    pub kstack_top: u64,
}

const _: () = assert!(core::mem::offset_of!(TaskContext, kstack_top) == 0xC0);

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready = 0,
    Running = 1,
    Blocked = 2,
    Exited = 3,
}

pub const STATE_READY: u32 = 0;
pub const STATE_RUNNING: u32 = 1;
pub const STATE_BLOCKED: u32 = 2;

pub struct InitArg {
    pub display_lease: crate::capability::DisplayHardwareLease,
}

/// A vThread (Task) in the SASOS model.
pub struct Task {
    pub id: u32,
    pub context: TaskContext,
    pub state: AtomicU32,
    pub signal_ring: *mut RingBuffer<u8, 32>,
    pub kstack_top: u64,
    pub(crate) ext_init: Option<InitArg>,
}

impl Task {
    pub fn new(id: u32, entry_point: u64, stack_top: u64, pd: &ProtectionDomain, is_user: bool) -> Self {
        let pkru = pd.current_pkru_mask.load(Ordering::SeqCst);
        let selectors = crate::gdt::get_selectors();

        let (cs, ss, rflags) = if is_user {
            (0x2Bu64, 0x23u64, 0x202u64)  // CS=index5|RPL3, SS=index4|RPL3
        } else {
            (selectors.kernel_cs.0 as u64, selectors.kernel_ss.0 as u64, 0x202u64)
        };

        let kstack = alloc::vec![0u8; 65536];
        let kstack_alloc_top = kstack.as_ptr() as u64 + 65536;
        core::mem::forget(kstack);

        // Pre-seed kstack with IRETQ frame + Dummy Error + GPR zeros.
        // Layout low→high: [r15=0..rax=0][dummy=0][RIP][CS][RFLAGS][RSP][SS]
        let forged_ksp = unsafe {
            let mut ksp = kstack_alloc_top as *mut u64;
            ksp = ksp.sub(1); *ksp = ss;
            ksp = ksp.sub(1); *ksp = stack_top;
            ksp = ksp.sub(1); *ksp = rflags;
            ksp = ksp.sub(1); *ksp = cs;
            ksp = ksp.sub(1); *ksp = entry_point;
            ksp = ksp.sub(1); *ksp = 0; // dummy error code
            for _ in 0..15 { ksp = ksp.sub(1); *ksp = 0; } // rax..r15 zeros
            ksp as u64
        };

        let context = TaskContext {
            r15: 0, r14: 0, r13: 0, r12: 0, r11: 0, r10: 0, r9: 0, r8: 0,
            rdi: 0, rsi: 0, rbp: 0, rdx: 0, rcx: 0, rbx: 0, rax: 0,
            dummy_error_code: 0,
            pkru: pkru as u64,
            pd_id: pd.id as u64,
            rip: entry_point,
            cs,
            rflags,
            rsp: stack_top,
            ss,
            pd_ptr: pd as *const _,
            kstack_top: forged_ksp,  // active ksp; switch_to saves/loads here
        };

        Self {
            id,
            context,
            state: AtomicU32::new(TaskState::Ready as u32),
            signal_ring: pd.signal_ring,
            kstack_top: kstack_alloc_top,  // initial alloc top (for TSS RSP0)
            ext_init: None,
        }
    }
}

pub const QUEUE_SIZE: usize = 512;
pub const QUEUE_MASK: usize = QUEUE_SIZE - 1;

pub struct WorkStealingQueue {
    pub top: AtomicUsize,
    pub bottom: AtomicUsize,
    buffer: [AtomicPtr<Task>; QUEUE_SIZE],
}

impl WorkStealingQueue {
    pub const fn new() -> Self {
        const EMPTY_PTR: AtomicPtr<Task> = AtomicPtr::new(ptr::null_mut());
        Self {
            top: AtomicUsize::new(0),
            bottom: AtomicUsize::new(0),
            buffer: [EMPTY_PTR; QUEUE_SIZE],
        }
    }

    pub fn push(&self, task: *mut Task) {
        let b = self.bottom.load(Ordering::Relaxed);
        let t = self.top.load(Ordering::Acquire);
        if b.wrapping_sub(t) < QUEUE_SIZE {
            self.buffer[b & QUEUE_MASK].store(task, Ordering::Release);
            self.bottom.store(b.wrapping_add(1), Ordering::Release);
        }
    }

    pub fn steal(&self) -> *mut Task {
        let t = self.top.load(Ordering::Acquire);
        let b = self.bottom.load(Ordering::Acquire);
        if t == b { return ptr::null_mut(); }
        let task = self.buffer[t & QUEUE_MASK].load(Ordering::Acquire);
        if self.top.compare_exchange(t, t.wrapping_add(1), Ordering::SeqCst, Ordering::Relaxed).is_ok() {
            task
        } else {
            ptr::null_mut()
        }
    }
}

pub struct Scheduler {
    pub runqueue: WorkStealingQueue,
    pub current_task: AtomicPtr<Task>,
    pub core_id: u32,
}

static SCHED_NO_RUNNABLE_LOG_BUDGET: AtomicU64 = AtomicU64::new(16);
static FIRST_SCHEDULE_LOGGED: AtomicU64 = AtomicU64::new(0);
static SCHED_TICK_ENTER_LOG_BUDGET: AtomicU64 = AtomicU64::new(32);
static SCHED_PICK_NEXT_LOG_BUDGET: AtomicU64 = AtomicU64::new(32);
static TASK_LIFECYCLE_LOG_BUDGET: AtomicU64 = AtomicU64::new(128);

#[no_mangle]
pub static mut ACTUAL_IRET_RSP: u64 = 0;
#[no_mangle]
pub static mut ACTUAL_IRET_Q0_RIP: u64 = 0;
#[no_mangle]
pub static mut ACTUAL_IRET_Q1_CS: u64 = 0;
#[no_mangle]
pub static mut ACTUAL_IRET_Q2_RFLAGS: u64 = 0;
#[no_mangle]
pub static mut ACTUAL_IRET_Q3_RSP: u64 = 0;
#[no_mangle]
pub static mut ACTUAL_IRET_Q4_SS: u64 = 0;
#[no_mangle]
pub static mut SWITCH_NEXT_CTX_PTR: u64 = 0;

impl Scheduler {
    pub const fn new(core_id: u32) -> Self {
        Self {
            runqueue: WorkStealingQueue::new(),
            current_task: AtomicPtr::new(ptr::null_mut()),
            core_id,
        }
    }

    pub fn tick(&self) -> Option<(*mut TaskContext, *const TaskContext)> {
        if SCHED_TICK_ENTER_LOG_BUDGET
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_sub(1))
            .is_ok()
        {
            let b = self.runqueue.bottom.load(Ordering::Acquire);
            let t = self.runqueue.top.load(Ordering::Acquire);
            serial_println!(
                "scheduler.tick.enter core={} phase={} rq_depth={}",
                self.core_id,
                unsafe { crate::ipc::BOOT_CONTROLLER.phase() as u8 },
                b.wrapping_sub(t)
            );
        }
        // Enforce hard execution gate
        assert!(
            unsafe { crate::ipc::BOOT_CONTROLLER.phase() as u8 } >= crate::ipc::BootPhase::SchedulerRunning as u8,
            "SCHEDULER_RUNNING_VIOLATION"
        );

        // Boot Strap Gate
        assert!(
            crate::core_local::INITIALIZED.load(Ordering::Acquire) == true,
            "SCHEDULER_BOOTSTRAP_VIOLATION"
        );

        let next_task = self.runqueue.steal();
        let next_task = if next_task.is_null() { self.attempt_steal() } else { next_task };
        let old_task = self.current_task.swap(next_task, Ordering::AcqRel);

        if !next_task.is_null() {
            unsafe {
                (*next_task).state.store(TaskState::Running as u32, Ordering::Release);
                if TASK_LIFECYCLE_LOG_BUDGET
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_sub(1))
                    .is_ok()
                {
                    serial_println!(
                        "task.running id={} pd_id={} rip={:#x} rsp={:#x}",
                        (*next_task).id,
                        (*next_task).context.pd_id,
                        (*next_task).context.rip,
                        (*next_task).context.rsp
                    );
                }
            }
        }

        unsafe {
            if !next_task.is_null() {
                let core = crate::core_local::CoreLocal::get();
                let next_pd_ptr = (*next_task).context.pd_ptr as *mut crate::capability::ProtectionDomain;

                // Enforce strict atomic ordering: bind -> wrpkru -> switch_to
                core.set_pd(next_pd_ptr);
            }
        }

        if next_task.is_null() {
            if SCHED_NO_RUNNABLE_LOG_BUDGET.fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_sub(1)).is_ok() {
                serial_println!(
                    "sched.no_runnable core={} phase={} runqueue_empty=true",
                    self.core_id,
                    unsafe { crate::ipc::BOOT_CONTROLLER.phase() as u8 }
                );
            }
            return None;
        }
        if SCHED_PICK_NEXT_LOG_BUDGET
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_sub(1))
            .is_ok()
        {
            let next_pd_id = unsafe { (*next_task).context.pd_id };
            serial_println!("scheduler.pick_next pd_id={}", next_pd_id);
        }

        if !old_task.is_null() {
            let old_state = unsafe { (*old_task).state.load(Ordering::Acquire) };
            if old_state == TaskState::Running as u32 {
                unsafe { (*old_task).state.store(TaskState::Ready as u32, Ordering::Release); }
                self.runqueue.push(old_task);
                if TASK_LIFECYCLE_LOG_BUDGET
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_sub(1))
                    .is_ok()
                {
                    unsafe {
                        serial_println!(
                            "task.requeued id={} pd_id={}",
                            (*old_task).id,
                            (*old_task).context.pd_id
                        );
                    }
                }
            } else if TASK_LIFECYCLE_LOG_BUDGET
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_sub(1))
                .is_ok()
            {
                unsafe {
                    serial_println!(
                        "task.not_requeued id={} pd_id={} reason_state={}",
                        (*old_task).id,
                        (*old_task).context.pd_id,
                        old_state
                    );
                }
            }
        }

        let old_ctx = if old_task.is_null() {
            core::ptr::null_mut()
        } else {
            unsafe { &mut (*old_task).context as *mut _ }
        };
        unsafe { Some((old_ctx, &(*next_task).context)) }
    }


    fn attempt_steal(&self) -> *mut Task {
        for i in 1..128 {
            let victim_id = (self.core_id + i) % 128;
            let task = SCHEDULERS[victim_id as usize].runqueue.steal();
            if !task.is_null() { return task; }
        }
        ptr::null_mut()
    }

    #[unsafe(naked)]
    pub unsafe extern "C" fn switch_to(_old_context: *mut TaskContext, next_context: *const TaskContext) {
        core::arch::naked_asm!(
            // rdi = old_ctx (*mut TaskContext) — kstack_top already set by timer_interrupt_handler
            // rsi = next_ctx (*const TaskContext)

            // old_ctx.kstack_top is set before this call in timer_interrupt_handler.
            // Do NOT save RSP here — that would overwrite the correct stub-frame base
            // with the kernel call-stack depth at time of switch_to entry.

            // 1. Switch to next_ctx.kstack_top and restore PKRU
            "mov rsp, [rsi + 0xC0]",
            "cmp byte ptr [rip + {pku_enabled}], 0",
            "je 2f",
            "mov eax, [rsi + 0x80]", // TaskContext.pkru at offset 0x80
            "xor ecx, ecx",
            "xor edx, edx",
            "wrpkru",

            "2:",
            // 3. Debug-log IRET frame for GP fault logger (before pops, R11 scratch).
            //    IRET frame is at RSP+128 (16 qwords of GPRs+dummy above RIP).
            "lea r11, [rsp + 128]",
            "mov [rip + {actual_iret_rsp}], r11",
            "mov r11, [rsp + 128]", "mov [rip + {actual_q0}], r11",
            "mov r11, [rsp + 136]", "mov [rip + {actual_q1}], r11",
            "mov r11, [rsp + 144]", "mov [rip + {actual_q2}], r11",
            "mov r11, [rsp + 152]", "mov [rip + {actual_q3}], r11",
            "mov r11, [rsp + 160]", "mov [rip + {actual_q4}], r11",

            // 4. Check CS.RPL for userspace (swapgs needed before iretq).
            "mov r11, [rsp + 136]",
            "test r11, 3",
            "jz 3f",
            "swapgs",
            "3:",

            // 5. Pop GPRs in saved-stack order: [rax] at lowest address (kstack_top).
            //    Saved (preempted) tasks have correct register values here;
            //    forged (first-run) tasks have all zeros, so any pop order works.
            "pop rax", "pop rbx", "pop rcx", "pop rdx",
            "pop rbp", "pop rsi", "pop rdi",
            "pop r8", "pop r9", "pop r10", "pop r11",
            "pop r12", "pop r13", "pop r14", "pop r15",
            "add rsp, 8",  // skip dummy error code
            "iretq",

            pku_enabled = sym crate::pku::PKU_ENABLED,
            actual_iret_rsp = sym ACTUAL_IRET_RSP,
            actual_q0 = sym ACTUAL_IRET_Q0_RIP,
            actual_q1 = sym ACTUAL_IRET_Q1_CS,
            actual_q2 = sym ACTUAL_IRET_Q2_RFLAGS,
            actual_q3 = sym ACTUAL_IRET_Q3_RSP,
            actual_q4 = sym ACTUAL_IRET_Q4_SS,
        );
    }
}

macro_rules! generate_schedulers {
    ($($idx:expr),*) => { [$(Scheduler::new($idx)),*] };
}

pub static SCHEDULERS: [Scheduler; 128] = generate_schedulers!(
    0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,
    32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,
    64,65,66,67,68,69,70,71,72,73,74,75,76,77,78,79,80,81,82,83,84,85,86,87,88,89,90,91,92,93,94,95,
    96,97,98,99,100,101,102,103,104,105,106,107,108,109,110,111,112,113,114,115,116,117,118,119,120,121,122,123,124,125,126,127
);

pub fn park_current_thread() {
    let core_id = crate::core_local::CoreLocal::get().core_id;
    let sched = &SCHEDULERS[core_id as usize];
    let current = sched.current_task.load(Ordering::Acquire);
    if !current.is_null() {
        unsafe { (*current).state.store(TaskState::Blocked as u32, Ordering::Release); }
    }
}

pub fn unpark_thread(task_ptr: *mut Task) {
    unsafe { (*task_ptr).state.store(TaskState::Ready as u32, Ordering::Release); }
}

pub fn bootstrap_enqueue_all_domains() -> usize {
    serial_println!("scheduler.bootstrap_enqueue.begin");
    let sched = &SCHEDULERS[0];
    let mut count = 0usize;

    for pd_id in 0..crate::ipc::MAX_DOMAINS as u32 {
        if let Some(pd) = crate::ipc::DOMAIN_REGISTRY.get(pd_id) {
            let task_ptr = pd.main_task.load(Ordering::Acquire);
            if task_ptr.is_null() {
                continue;
            }
            unsafe { (*task_ptr).state.store(TaskState::Ready as u32, Ordering::Release); }
            sched.runqueue.push(task_ptr);
            count += 1;
            unsafe {
                serial_println!("scheduler.enqueue pd_id={} task={:#x}", pd.id, task_ptr as u64);
                serial_println!(
                    "task.created id={} pd_id={} rip={:#x} rsp={:#x}",
                    (*task_ptr).id,
                    (*task_ptr).context.pd_id,
                    (*task_ptr).context.rip,
                    (*task_ptr).context.rsp
                );
            }
        }
    }

    serial_println!("scheduler.bootstrap_enqueue.done count={}", count);
    count
}

pub fn log_first_scheduled_pd(pd_id: u64) {
    if FIRST_SCHEDULE_LOGGED
        .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        serial_println!("first scheduled pd_id={}", pd_id);
    }
}

#[inline]
fn is_canonical(addr: u64) -> bool {
    let sign = (addr >> 47) & 1;
    let high = addr >> 48;
    (sign == 0 && high == 0) || (sign == 1 && high == 0xFFFF)
}

pub unsafe fn debug_dump_iret_frame(next_ctx: *const TaskContext) {
    // switch_to pops 15 GPR qwords + 1 dummy error, then iretq reads RIP/CS/RFLAGS/RSP/SS.
    let iret_rsp = ((*next_ctx).kstack_top + (16 * 8)) as *const u64;
    let rip = *iret_rsp.add(0);
    let cs = *iret_rsp.add(1);
    let rflags = *iret_rsp.add(2);
    let rsp = *iret_rsp.add(3);
    let ss = *iret_rsp.add(4);

    serial_println!(
        "iret.frame.qwords rsp={:#x} [0]={:#x} [1]={:#x} [2]={:#x} [3]={:#x} [4]={:#x}",
        iret_rsp as u64, rip, cs, rflags, rsp, ss
    );
    serial_println!(
        "iret.frame.check rip_canon={} rsp_canon={} rflags.bit1={} rflags.if={} cs={:#x} ss={:#x}",
        is_canonical(rip),
        is_canonical(rsp),
        (rflags & 0x2) != 0,
        (rflags & 0x200) != 0,
        cs,
        ss
    );
}

pub unsafe fn debug_dump_user_entry_bytes(next_ctx: *const TaskContext) {
    let rip = (*next_ctx).rip as *const u8;
    let mut bytes = [0u8; 32];
    core::ptr::copy_nonoverlapping(rip, bytes.as_mut_ptr(), bytes.len());

    serial_println!(
        "user.entry.bytes rip={:#x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
        rip as u64,
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
        bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30], bytes[31]
    );
}

pub fn yield_now() {
    let core_id = crate::core_local::CoreLocal::get().core_id;
    let sched = &SCHEDULERS[core_id as usize];
    let current = sched.current_task.load(Ordering::Acquire);
    if !current.is_null() {
        unsafe {
            let state = (*current).state.load(Ordering::Acquire);
            if state == TaskState::Running as u32 {
                (*current).state.store(TaskState::Ready as u32, Ordering::Release);
                sched.runqueue.push(current);
                // Clear current_task so tick() won't see Running and
                // re-requeue a task already placed in the runqueue.
                sched.current_task.store(ptr::null_mut(), Ordering::Release);
            }
        }
    }
}
