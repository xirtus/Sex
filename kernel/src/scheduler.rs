use core::sync::atomic::{AtomicU32, Ordering, AtomicPtr, AtomicUsize};
use crate::capability::ProtectionDomain;
use crate::ipc_ring::RingBuffer;
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
    pub kstack_top: u64,
}

impl Task {
    pub fn new(id: u32, entry_point: u64, stack_top: u64, pd: &ProtectionDomain, is_user: bool) -> Self {
        let pkru = pd.current_pkru_mask.load(Ordering::SeqCst);
        let selectors = crate::gdt::get_selectors();

        let (cs, ss, rflags) = if is_user {
            (selectors.user_cs.0 as u64, selectors.user_ss.0 as u64, 0x202)
        } else {
            (selectors.kernel_cs.0 as u64, selectors.kernel_ss.0 as u64, 0x202)
        };

        // Phase 32: Allocate a dedicated 64KB kernel stack for each task.
        // This ensures that when the CPU switches from Ring 3 -> Ring 0,
        // it lands on a clean, private stack.
        let kstack = alloc::vec![0u8; 65536];
        let kstack_top = kstack.as_ptr() as u64 + 65536;
        core::mem::forget(kstack); // Leak stack for simplicity in Phase 32

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
            kstack_top,
        };

        Self {
            id,
            context,
            state: AtomicU32::new(TaskState::Ready as u32),
            signal_ring: pd.signal_ring,
            kstack_top,
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
    pub unsafe extern "C" fn switch_to(_old_context: *mut TaskContext, next_context: *const TaskContext) {
        core::arch::naked_asm!(
            // rdi = _old_context, rsi = next_context
            // 1. Check if _old_context is NULL (first boot)
            "test rdi, rdi",
            "jz 1f",

            // 2. Save current state into _old_context (rdi)
            "mov [rdi + 0x00], r15",
            "mov [rdi + 0x08], r14",
            "mov [rdi + 0x10], r13",
            "mov [rdi + 0x18], r12",
            "mov [rdi + 0x20], r11",
            "mov [rdi + 0x28], r10",
            "mov [rdi + 0x30], r9",
            "mov [rdi + 0x38], r8",
            "mov [rdi + 0x50], rbp",
            "mov [rdi + 0x58], rdx",
            "mov [rdi + 0x60], rcx",
            "mov [rdi + 0x68], rbx",
            "mov [rdi + 0x70], rax",

            // Save caller's RIP and RSP securely
            "mov rax, [rsp]",           // Caller return address
            "mov [rdi + 0x90], rax",    // context.rip
            "lea rax, [rsp + 8]",       // Stack pointer before call
            "mov [rdi + 0xA8], rax",    // context.rsp

            "1:",
            // 3. Restore next_context (rsi)
            "mov rdi, rsi",             // Use RDI as the struct base pointer

            // Restore PKRU correctly (Requires EAX, ECX=0, EDX=0)
            "mov rax, [rdi + 0x80]",
            "xor rcx, rcx",
            "xor rdx, rdx",
            "wrpkru",

            // Restore general registers
            "mov r15, [rdi + 0x00]",
            "mov r14, [rdi + 0x08]",
            "mov r13, [rdi + 0x10]",
            "mov r12, [rdi + 0x18]",
            "mov r11, [rdi + 0x20]",
            "mov r10, [rdi + 0x28]",
            "mov r9, [rdi + 0x30]",
            "mov r8, [rdi + 0x38]",
            "mov rbp, [rdi + 0x50]",
            "mov rdx, [rdi + 0x58]",
            "mov rcx, [rdi + 0x60]",
            "mov rbx, [rdi + 0x68]",
            "mov rax, [rdi + 0x70]",

            // 4. Secure the Stack & Prepare IRETQ
            // Safely switch to the task's private kernel stack (0xC0)
            "mov rsp, [rdi + 0xC0]",

            // Push the 5-QWORD IRETQ frame onto this clean stack
            "push qword ptr [rdi + 0xB0]", // SS
            "push qword ptr [rdi + 0xA8]", // RSP
            "push qword ptr [rdi + 0xA0]", // RFLAGS
            "push qword ptr [rdi + 0x98]", // CS
            "push qword ptr [rdi + 0x90]", // RIP

            // Restore the final two scratch registers
            "mov rsi, [rdi + 0x48]",
            "mov rdi, [rdi + 0x40]",

            // 5. Jump to Userland!
            "iretq"
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

pub fn yield_now() {
    // Phase 25: No-op. Preemption handles switching in Ring 3.
}
