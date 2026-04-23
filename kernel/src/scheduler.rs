use core::sync::atomic::{AtomicU32, Ordering, AtomicPtr, AtomicUsize};
use crate::capability::ProtectionDomain;
use crate::ipc_ring::RingBuffer;
use core::ptr;

/// The execution state of a vThread (Task).
#[repr(C)]
pub struct TaskContext {
    pub r15: u64, pub r14: u64, pub r13: u64, pub r12: u64,
    pub rbx: u64, pub rbp: u64,
    pub pkru: u32,
    pub pd_id: u32,
    pub rip: u64, pub cs: u64, pub rflags: u64, pub rsp: u64, pub ss: u64,
    pub pd_ptr: *const ProtectionDomain,
}

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

/// A vThread (Task) in the SASOS model.
pub struct Task {
    pub id: u32,
    pub context: TaskContext,
    pub state: AtomicU32,
    pub signal_ring: *mut RingBuffer<u8, 32>,
}

impl Task {
    pub fn new(id: u32, entry_point: u64, stack_top: u64, pd: &ProtectionDomain, is_user: bool) -> Self {
        let pkru = pd.current_pkru_mask.load(Ordering::SeqCst);
        let selectors = crate::gdt::get_selectors();
        
        let aligned_rsp = stack_top & !0xFu64;

        let (cs, ss, rflags) = if is_user {
            (selectors.user_code_selector.0 as u64 | 3, selectors.user_data_selector.0 as u64 | 3, 0x3202)
        } else {
            (selectors.code_selector.0 as u64, selectors.data_selector.0 as u64, 0x202)
        };

        Self {
            id,
            context: TaskContext {
                r15: 0, r14: 0, r13: 0, r12: 0, rbx: 0, rbp: 0,
                pkru, pd_id: pd.id,
                rip: entry_point, cs, rflags, rsp: aligned_rsp, ss,
                pd_ptr: pd as *const _,
            },
            state: AtomicU32::new(TaskState::Ready as u32),
            signal_ring: pd.signal_ring,
        }
    }
}

pub const QUEUE_SIZE: usize = 512;
pub const QUEUE_MASK: usize = QUEUE_SIZE - 1;

pub struct WorkStealingQueue {
    top: AtomicUsize,
    bottom: AtomicUsize,
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
        if b.wrapping_sub(t) >= QUEUE_SIZE { return; }
        self.buffer[b & QUEUE_MASK].store(task, Ordering::Relaxed);
        core::sync::atomic::compiler_fence(Ordering::Release);
        self.bottom.store(b.wrapping_add(1), Ordering::Release);
    }

    pub fn pop(&self) -> *mut Task {
        let b = self.bottom.load(Ordering::Relaxed);
        if b == 0 { return ptr::null_mut(); }
        let b = b.wrapping_sub(1);
        self.bottom.store(b, Ordering::Relaxed);
        core::sync::atomic::fence(Ordering::SeqCst);
        let t = self.top.load(Ordering::Relaxed);
        if t <= b {
            let task = self.buffer[b & QUEUE_MASK].load(Ordering::Relaxed);
            if t < b { return task; }
            if self.top.compare_exchange(t, t + 1, Ordering::SeqCst, Ordering::Relaxed).is_ok() {
                self.bottom.store(t + 1, Ordering::Relaxed);
                return task;
            }
            self.bottom.store(t + 1, Ordering::Relaxed);
            ptr::null_mut()
        } else {
            self.bottom.store(t, Ordering::Relaxed);
            ptr::null_mut()
        }
    }

    pub fn steal(&self) -> *mut Task {
        loop {
            let t = self.top.load(Ordering::Acquire);
            core::sync::atomic::fence(Ordering::SeqCst);
            let b = self.bottom.load(Ordering::Acquire);
            if t >= b { return ptr::null_mut(); }
            let task = self.buffer[t & QUEUE_MASK].load(Ordering::Acquire);
            if self.top.compare_exchange(t, t + 1, Ordering::SeqCst, Ordering::Relaxed).is_ok() {
                return task;
            }
        }
    }
}

pub struct Scheduler {
    pub runqueue: WorkStealingQueue,
    pub current_task: AtomicPtr<Task>,
    pub core_id: u32,
}

impl Scheduler {
    pub const fn new(core_id: u32) -> Self {
        Self {
            runqueue: WorkStealingQueue::new(),
            current_task: AtomicPtr::new(ptr::null_mut()),
            core_id,
        }
    }

    pub fn tick(&self) -> Option<(*mut TaskContext, *const TaskContext)> {
        let next_task = self.runqueue.steal();
        let next_task = if next_task.is_null() { self.attempt_steal() } else { next_task };
        if next_task.is_null() { return None; }

        let old_task = self.current_task.swap(next_task, Ordering::AcqRel);
        
        if old_task.is_null() {
            unsafe {
                (*next_task).state.store(TaskState::Running as u32, Ordering::Release);
                return Some((ptr::null_mut(), &(*next_task).context));
            }
        }

        let old_state = unsafe { (*old_task).state.load(Ordering::Acquire) };
        if old_state == TaskState::Running as u32 {
            unsafe { (*old_task).state.store(TaskState::Ready as u32, Ordering::Release); }
            self.runqueue.push(old_task);
        }

        unsafe {
            (*next_task).state.store(TaskState::Running as u32, Ordering::Release);
            Some((&mut (*old_task).context, &(*next_task).context))
        }
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
    pub unsafe extern "C" fn switch_to(old_context: *mut TaskContext, next_context: *const TaskContext) {
        core::arch::naked_asm!(
            "xor eax, eax", "xor edx, edx", "xor ecx, ecx", "wrpkru",

            "test rdi, rdi",
            "jz 2f", 
            "mov qword ptr [rdi + 0x00], r15",
            "mov qword ptr [rdi + 0x08], r14",
            "mov qword ptr [rdi + 0x10], r13",
            "mov qword ptr [rdi + 0x18], r12",
            "mov qword ptr [rdi + 0x20], rbx",
            "mov qword ptr [rdi + 0x28], rbp",

            "2:",
            "mov r15, [rsi + 0x00]", "mov r14, [rsi + 0x08]",
            "mov r13, [rsi + 0x10]", "mov r12, [rsi + 0x18]",
            "mov rbx, [rsi + 0x20]", "mov rbp, [rsi + 0x28]",
            
            "mov eax, [rsi + 0x30]", "xor edx, edx", "xor ecx, ecx", "wrpkru",
            
            "push [rsi + 0x58]",
            "push [rsi + 0x50]",
            "push [rsi + 0x48]",
            "push [rsi + 0x40]",
            "push [rsi + 0x38]",

            "mov rax, [rsi + 0x40]",
            "test al, 3",
            "jz 3f",
            "swapgs",
            "3:",
            
            "xor rax, rax", 
            "iretq",
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
    let core_id = crate::core_local::CoreLocal::get().core_id;
    SCHEDULERS[core_id as usize].runqueue.push(task_ptr);
}

pub fn yield_now() {
    unsafe {
        // Briefly enable interrupts so the timer can preempt us, then disable again
        core::arch::asm!("sti; nop; cli");
    }
}
