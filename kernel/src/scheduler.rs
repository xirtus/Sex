use alloc::collections::VecDeque;
use alloc::sync::Arc;
use spin::Mutex;
use x86_64::VirtAddr;
use crate::capability::ProtectionDomain;

/// The execution state of a vThread (Task).
#[repr(C)]
pub struct TaskContext {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
    pub rsp: u64,
    /// The current PKRU mask for this task.
    pub pkru: u32,
    /// The Protection Domain this task belongs to.
    pub pd: Arc<ProtectionDomain>,
}

impl TaskContext {
    pub fn new(rsp: u64, pd: Arc<ProtectionDomain>) -> Self {
        let pkru = *pd.current_pkru_mask.lock();
        Self {
            r15: 0, r14: 0, r13: 0, r12: 0, rbx: 0, rbp: 0,
            rsp,
            pkru,
            pd,
        }
    }
}

pub enum TaskState {
    Ready,
    Running,
    Blocked,
}

/// A vThread (Task) in the SASOS model.
pub struct Task {
    pub id: u32,
    pub context: TaskContext,
    pub state: TaskState,
}

/// Per-core Lockless Scheduler.
pub struct Scheduler {
    pub runqueue: VecDeque<Arc<Mutex<Task>>>,
    pub current_task: Option<Arc<Mutex<Task>>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            runqueue: VecDeque::new(),
            current_task: None,
        }
    }

    pub fn spawn(&mut self, task: Task) {
        self.runqueue.push_back(Arc::new(Mutex::new(task)));
    }

    /// The hardware-accelerated context switch.
    /// This routine saves the current task's registers and loads the next task's registers and PKRU.
    #[naked]
    pub unsafe extern "C" fn switch_to(old_context: *mut TaskContext, next_context: *const TaskContext) {
        core::arch::asm!(
            // 1. Save current task's state
            "mov [rdi + 0x00], r15",
            "mov [rdi + 0x08], r14",
            "mov [rdi + 0x10], r13",
            "mov [rdi + 0x18], r12",
            "mov [rdi + 0x20], rbx",
            "mov [rdi + 0x28], rbp",
            "mov [rdi + 0x30], rsp",
            "rdpkru",
            "mov [rdi + 0x38], eax",

            // 2. Load next task's state
            "mov r15, [rsi + 0x00]",
            "mov r14, [rsi + 0x08]",
            "mov r13, [rsi + 0x10]",
            "mov r12, [rsi + 0x18]",
            "mov rbx, [rsi + 0x20]",
            "mov rbp, [rsi + 0x28]",
            "mov rsp, [rsi + 0x30]",
            "mov eax, [rsi + 0x38]",
            "xor edx, edx",
            "xor ecx, ecx",
            "wrpkru",

            "ret",
            options(noreturn)
        );
    }
}

pub static mut SCHEDULERS: [Option<Scheduler>; 128] = [None; 128];

pub fn init_core(core_id: usize) {
    unsafe {
        SCHEDULERS[core_id] = Some(Scheduler::new());
    }
}
