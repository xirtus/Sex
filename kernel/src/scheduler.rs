use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, Ordering, AtomicPtr};
use x86_64::VirtAddr;
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
    pub pd: Arc<ProtectionDomain>,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready = 0,
    Running = 1,
    Blocked = 2,
    Exited = 3,
}

/// A vThread (Task) in the SASOS model.
/// IPCtax-Compliant: No Mutex, Atomic State.
pub struct Task {
    pub id: u32,
    pub context: TaskContext,
    pub state: AtomicU32, // Stores TaskState
    pub signal_ring: Arc<RingBuffer<u8, 32>>,
    /// Next task in the lock-free runqueue
    pub next: AtomicPtr<Task>,
}

impl Task {
    pub fn new(id: u32, entry_point: u64, stack_top: u64, pd: Arc<ProtectionDomain>, is_user: bool) -> Self {
        let pkru = pd.current_pkru_mask.load(Ordering::SeqCst);
        let selectors = crate::gdt::get_selectors();
        let (cs, ss) = if is_user {
            (selectors.user_code_selector.0 as u64 | 3, selectors.user_data_selector.0 as u64 | 3)
        } else {
            (selectors.code_selector.0 as u64, 0)
        };

        Self {
            id,
            context: TaskContext {
                r15: 0, r14: 0, r13: 0, r12: 0, rbx: 0, rbp: 0,
                pkru, pd_id: pd.id,
                rip: entry_point, cs, rflags: 0x202, rsp: stack_top, ss,
                pd: pd.clone(),
            },
            state: AtomicU32::new(TaskState::Ready as u32),
            signal_ring: pd.signal_ring.clone(),
            next: AtomicPtr::new(ptr::null_mut()),
        }
    }
}

/// A Lock-Free MPSC (Multi-Producer, Single-Consumer) Queue.
/// IPCtax-Compliant: True lock-free CAS-based implementation.
pub struct MpscQueue {
    head: AtomicPtr<Task>,
    tail: AtomicPtr<Task>,
}

impl MpscQueue {
    pub const fn new() -> Self {
        Self {
            head: AtomicPtr::new(ptr::null_mut()),
            tail: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// Wait-free enqueue using an atomic swap on the head pointer.
    pub fn enqueue(&self, task: *mut Task) {
        unsafe { (*task).next.store(ptr::null_mut(), Ordering::Relaxed); }
        let prev_head = self.head.swap(task, Ordering::AcqRel);
        
        if prev_head.is_null() {
            // Queue was empty, set tail to the new task.
            // This CAS handles the race condition where a consumer might be dequeueing.
            let _ = self.tail.compare_exchange(ptr::null_mut(), task, Ordering::Release, Ordering::Relaxed);
        } else {
            // Queue wasn't empty, link the previous head to the new task.
            unsafe { (*prev_head).next.store(task, Ordering::Release); }
        }
    }

    /// Lock-free dequeue (Consumer-only).
    pub fn dequeue(&self) -> *mut Task {
        loop {
            let tail = self.tail.load(Ordering::Acquire);
            if tail.is_null() {
                return ptr::null_mut(); // Queue is empty
            }

            let next = unsafe { (*tail).next.load(Ordering::Acquire) };
            if next.is_null() {
                // Potential last item. We must carefully transition to empty.
                let head = self.head.load(Ordering::Acquire);
                if tail == head {
                    // It is the last item. Try to nullify the head to prevent new enqueues 
                    // from linking to it while we dequeue.
                    if self.head.compare_exchange(tail, ptr::null_mut(), Ordering::AcqRel, Ordering::Relaxed).is_ok() {
                        // Head is nullified, now safely nullify tail and return.
                        self.tail.store(ptr::null_mut(), Ordering::Release);
                        return tail;
                    }
                }
                // If tail != head but next is null, a producer is mid-enqueue.
                // We must spin briefly (lock-free property) until the link is established.
                core::hint::spin_loop();
                continue;
            }

            // Not the last item. Advance the tail.
            // Since this is MPSC (Single Consumer), we technically don't need a CAS here 
            // if we guarantee only one core dequeues from this specific runqueue.
            // However, we use a simple store for the single-consumer assumption.
            self.tail.store(next, Ordering::Release);
            return tail;
        }
    }
}

/// Per-core Lock-Free Scheduler.
pub struct Scheduler {
    pub runqueue: MpscQueue,
    pub current_task: AtomicPtr<Task>,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            runqueue: MpscQueue::new(),
            current_task: AtomicPtr::new(ptr::null_mut()),
        }
    }

    pub fn tick(&self) -> Option<(*mut TaskContext, *const TaskContext)> {
        let next_task = self.runqueue.dequeue();
        if next_task.is_null() { return None; }

        let old_task = self.current_task.swap(next_task, Ordering::AcqRel);
        if old_task.is_null() {
            unsafe { return Some((ptr::null_mut(), &(*next_task).context)); }
        }

        let old_state = unsafe { (*old_task).state.load(Ordering::Acquire) };
        if old_state == TaskState::Running as u32 {
            unsafe { (*old_task).state.store(TaskState::Ready as u32, Ordering::Release); }
            self.runqueue.enqueue(old_task);
        }

        unsafe { Some((&mut (*old_task).context, &(*next_task).context)) }
    }

    #[naked]
    pub unsafe extern "C" fn switch_to(old_context: *mut TaskContext, next_context: *const TaskContext) {
        core::arch::asm!(
            "test rdi, rdi",
            "jz 2f", // Skip saving if old_context is null (first boot)
            "mov [rdi + 0x00], r15", "mov [rdi + 0x08], r14",
            "mov [rdi + 0x10], r13", "mov [rdi + 0x18], r12",
            "mov [rdi + 0x20], rbx", "mov [rdi + 0x28], rbp",
            "rdpkru", "mov [rdi + 0x30], eax",
            "2:",
            "mov r15, [rsi + 0x00]", "mov r14, [rsi + 0x08]",
            "mov r13, [rsi + 0x10]", "mov r12, [rsi + 0x18]",
            "mov rbx, [rsi + 0x20]", "mov rbp, [rsi + 0x28]",
            "mov eax, [rsi + 0x30]", "xor edx, edx", "xor ecx, ecx", "wrpkru",
            "mov eax, [rsi + 0x34]", "mov [gs:0], eax",
            "push [rsi + 0x60]", "push [rsi + 0x58]", "push [rsi + 0x50]",
            "push [rsi + 0x48]", "push [rsi + 0x40]",
            "iretq",
            options(noreturn)
        );
    }
}

pub static SCHEDULERS: [Scheduler; 128] = [Scheduler::new(); 128];

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
    // Enqueue on the same core for cache locality (Simplified)
    let core_id = crate::core_local::CoreLocal::get().core_id;
    SCHEDULERS[core_id as usize].runqueue.enqueue(task_ptr);
}
