use alloc::collections::VecDeque;
use alloc::sync::Arc;
use spin::Mutex;
use x86_64::VirtAddr;
use crate::capability::ProtectionDomain;

/// The execution state of a vThread (Task).
#[repr(C)]
pub struct TaskContext {
    // 1. General Purpose Registers (saved on context switch)
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
    
    // 2. PKRU register state
    pub pkru: u32,
    pub pd_id: u32,

    // 3. iretq Frame (order required by hardware)
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,

    // 4. Protection Domain metadata
    pub pd: Arc<ProtectionDomain>,
}

impl TaskContext {
    pub fn new(entry_point: u64, stack_top: u64, pd: Arc<ProtectionDomain>, is_user: bool) -> Self {
        let pkru = pd.current_pkru_mask.load(core::sync::atomic::Ordering::SeqCst);
        let selectors = crate::gdt::get_selectors();
        
        let (cs, ss) = if is_user {
            (selectors.user_code_selector.0 as u64 | 3, selectors.user_data_selector.0 as u64 | 3)
        } else {
            (selectors.code_selector.0 as u64, 0)
        };

        Self {
            r15: 0, r14: 0, r13: 0, r12: 0, rbx: 0, rbp: 0,
            pkru,
            pd_id: pd.id,
            rip: entry_point,
            cs,
            rflags: 0x202, // IF (Interrupt Flag) enabled
            rsp: stack_top,
            ss,
            pd,
        }
    }
}

pub enum TaskState {
    Ready,
    Running,
    Blocked,
}

use crate::ipc_ring::RingBuffer;

/// A vThread (Task) in the SASOS model.
pub struct Task {
    pub id: u32,
    pub context: TaskContext,
    pub state: TaskState,
    /// Asynchronous Signal Ring (Signum)
    pub signal_ring: Arc<RingBuffer<u8, 32>>,
}
/// Per-core Lockless Scheduler.
pub struct Scheduler {
    pub runqueue: VecDeque<Arc<Mutex<Task>>>,
    pub wait_queue: VecDeque<Arc<Mutex<Task>>>,
    pub current_task: Option<Arc<Mutex<Task>>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            runqueue: VecDeque::new(),
            wait_queue: VecDeque::new(),
            current_task: None,
        }
    }

    pub fn spawn(&mut self, task: Task) {
        self.runqueue.push_back(Arc::new(Mutex::new(task)));
    }

    /// Blocks the current task and adds it to the wait queue.
    pub fn block_current(&mut self) {
        if let Some(task_mutex) = self.current_task.take() {
            {
                let mut task = task_mutex.lock();
                task.state = TaskState::Blocked;
            }
            self.wait_queue.push_back(task_mutex);
        }
    }

    /// Unblocks a task by its ID and moves it to the runqueue.
    pub fn unblock(&mut self, task_id: u32) {
        if let Some(pos) = self.wait_queue.iter().position(|t| t.lock().id == task_id) {
            let task_mutex = self.wait_queue.remove(pos).unwrap();
            {
                let mut task = task_mutex.lock();
                task.state = TaskState::Ready;
            }
            self.runqueue.push_back(task_mutex);
        }
    }

    /// Picks the next task to run and returns pointers for the context switch.
    /// Returns (old_context, new_context) if a switch is needed.
    pub fn tick(&mut self) -> Option<(*mut TaskContext, *const TaskContext)> {
        // 1. Simple round-robin for the prototype
        if let Some(next_task_mutex) = self.runqueue.pop_front() {
            let old_task_mutex = self.current_task.take().unwrap();
            
            // Save pointer to old context before re-enqueueing
            let old_ctx_ptr = unsafe { &mut (*old_task_mutex.as_ptr()).context as *mut TaskContext };
            
            self.runqueue.push_back(old_task_mutex);
            
            let next_task = next_task_mutex.clone();
            let next_ctx_ptr = unsafe { &(*next_task.as_ptr()).context as *const TaskContext };
            
            self.current_task = Some(next_task);
            
            return Some((old_ctx_ptr, next_ctx_ptr));
        }
        
        // 2. If runqueue is empty, try to steal from another core
        if let Some((old_ctx_ptr, next_ctx_ptr)) = self.try_load_balance() {
            return Some((old_ctx_ptr, next_ctx_ptr));
        }

        None
    }

    /// Attempts to steal a task from another core if this one is idle.
    fn try_load_balance(&mut self) -> Option<(*mut TaskContext, *const TaskContext)> {
        if self.current_task.is_none() { return None; }

        unsafe {
            for i in 0..128 {
                if let Some(ref mut other_sched) = SCHEDULERS[i] {
                    // Very basic "steal" - take from the back of their runqueue
                    if other_sched.runqueue.len() > 1 {
                        if let Some(stolen_task) = other_sched.runqueue.pop_back() {
                            serial_println!("SCHED: Core stealing Task {} from Core {}.", 
                                stolen_task.lock().id, i);
                            
                            let old_task_mutex = self.current_task.take().unwrap();
                            let old_ctx_ptr = unsafe { &mut (*old_task_mutex.as_ptr()).context as *mut TaskContext };
                            
                            self.runqueue.push_back(old_task_mutex);
                            self.current_task = Some(stolen_task.clone());
                            let next_ctx_ptr = unsafe { &(*stolen_task.as_ptr()).context as *const TaskContext };
                            
                            return Some((old_ctx_ptr, next_ctx_ptr));
                        }
                    }
                }
            }
        }
        None
    }

    /// The hardware-accelerated context switch.
    /// This routine saves the current task's registers and performs an `iretq` 
    /// transition to the next task's privilege level (Ring 0 or Ring 3).
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
            "rdpkru",
            "mov [rdi + 0x30], eax",

            // 2. Load next task's state
            "mov r15, [rsi + 0x00]",
            "mov r14, [rsi + 0x08]",
            "mov r13, [rsi + 0x10]",
            "mov r12, [rsi + 0x18]",
            "mov rbx, [rsi + 0x20]",
            "mov rbp, [rsi + 0x28]",
            
            // 3. Load PKRU and Update CoreLocal PD Identity
            "mov eax, [rsi + 0x30]",
            "xor edx, edx",
            "xor ecx, ecx",
            "wrpkru",
            
            "mov eax, [rsi + 0x34]",
            "mov [gs:0], eax",

            // 4. Prepare iretq frame
            "push [rsi + 0x58]", // ss
            "push [rsi + 0x50]", // rsp
            "push [rsi + 0x48]", // rflags
            "push [rsi + 0x40]", // cs
            "push [rsi + 0x38]", // rip

            // 5. Jump to next task
            "iretq",
            options(noreturn)
        );
    }
}

pub static mut SCHEDULERS: [Option<Scheduler>; 128] = [None; 128];

pub fn balanced_spawn(task: Task) {
    unsafe {
        let mut min_load = usize::MAX;
        let mut target_core = 0;

        for i in 0..128 {
            if let Some(ref sched) = SCHEDULERS[i] {
                let load = sched.runqueue.len();
                if load < min_load {
                    min_load = load;
                    target_core = i;
                }
            }
        }

        if let Some(ref mut sched) = SCHEDULERS[target_core] {
            serial_println!("SCHED: Spawning Task {} on Core {}.", task.id, target_core);
            sched.spawn(task);
        }
    }
}

/// Blocks the current thread (Phase 6 Signal Trampoline).
pub fn park_current_thread() {
    unsafe {
        if let Some(ref mut sched) = SCHEDULERS[0] { // Assuming single core for park/unpark prototype
            sched.block_current();
        }
    }
}

/// Wakes up a specific thread by its ID.
pub fn unpark_thread(tid: u32) {
    unsafe {
        if let Some(ref mut sched) = SCHEDULERS[0] {
            sched.unblock(tid);
        }
    }
}
