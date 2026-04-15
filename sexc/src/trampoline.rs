use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use core::mem::transmute;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use lazy_static::lazy_static;

pub const SA_RESTART: u64 = 0x1000_0000;
pub const SA_RESETHAND: u64 = 0x8000_0000;
pub const SA_SIGINFO: u64 = 0x0000_0004;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SigInfo {
    pub signo: i32,
    pub sender_pid: u32,
    pub code: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct UContext {
    pub pd_id: u32,
    pub delivery_count: u64,
    pub flags: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SigAction {
    pub handler: usize,
    pub flags: u64,
}

impl SigAction {
    pub const fn is_empty(self) -> bool {
        self.handler == 0
    }
}

struct PdState {
    actions: Mutex<BTreeMap<u8, SigAction>>,
    delivery_count: AtomicU64,
    worker_started: AtomicBool,
    #[cfg(feature = "std")]
    queue: Mutex<VecDeque<u8>>,
    #[cfg(feature = "std")]
    parked: Condvar,
}

impl PdState {
    fn new() -> Self {
        Self {
            actions: Mutex::new(BTreeMap::new()),
            delivery_count: AtomicU64::new(0),
            worker_started: AtomicBool::new(false),
            #[cfg(feature = "std")]
            queue: Mutex::new(VecDeque::new()),
            #[cfg(feature = "std")]
            parked: Condvar::new(),
        }
    }
}

#[cfg(feature = "std")]
use std::sync::{Condvar, Mutex, RwLock};
#[cfg(not(feature = "std"))]
use spin::{Mutex, RwLock};

lazy_static! {
    static ref PD_STATES: RwLock<BTreeMap<u32, Arc<PdState>>> = RwLock::new(BTreeMap::new());
}

fn pd_state(pd_id: u32) -> Arc<PdState> {
    if let Some(state) = read_states().get(&pd_id) {
        return Arc::clone(state);
    }

    let mut states = write_states();
    Arc::clone(states.entry(pd_id).or_insert_with(|| Arc::new(PdState::new())))
}

pub fn init_pd(pd_id: u32) {
    let state = pd_state(pd_id);
    #[cfg(feature = "std")]
    {
        if state
            .worker_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let worker_state = Arc::clone(&state);
            std::thread::Builder::new()
                .name(format!("sexc-trampoline-{pd_id}"))
                .spawn(move || trampoline_loop(pd_id, worker_state))
                .expect("failed to spawn signal trampoline");
        }
    }
}

pub fn register_sigaction(pd_id: u32, signo: u8, action: SigAction) {
    let state = pd_state(pd_id);
    action_guard(&state).insert(signo, action);
}

pub fn clear_sigaction(pd_id: u32, signo: u8) {
    let state = pd_state(pd_id);
    action_guard(&state).remove(&signo);
}

pub fn notify_signal(pd_id: u32, signo: u8) {
    #[cfg(feature = "std")]
    {
        let state = pd_state(pd_id);
        state.queue.lock().unwrap_or_else(recover).push_back(signo);
        state.parked.notify_one();
    }
}

pub fn dispatch_signal(pd_id: u32, signo: u8) -> bool {
    let state = pd_state(pd_id);
    let action = {
        let actions = action_guard(&state);
        actions.get(&signo).copied()
    };

    let Some(action) = action else {
        return false;
    };

    let delivery_count = state.delivery_count.fetch_add(1, Ordering::SeqCst) + 1;
    let mut siginfo = SigInfo {
        signo: signo as i32,
        sender_pid: pd_id,
        code: 0,
    };
    let mut ucontext = UContext {
        pd_id,
        delivery_count,
        flags: action.flags,
    };

    unsafe {
        if action.flags & SA_SIGINFO != 0 {
            let func: extern "C" fn(i32, *mut SigInfo, *mut UContext) = transmute(action.handler);
            func(signo as i32, &mut siginfo, &mut ucontext);
        } else {
            let func: extern "C" fn(i32) = transmute(action.handler);
            func(signo as i32);
        }
    }

    if action.flags & SA_RESETHAND != 0 {
        clear_sigaction(pd_id, signo);
    }

    true
}

#[cfg(not(feature = "std"))]
pub fn pump_pending<const N: usize>(pd_id: u32, ring: &crate::ipc_ring::RingBuffer<u8, N>) -> usize {
    let mut dispatched = 0;
    while let Some(signal) = ring.dequeue() {
        if dispatch_signal(pd_id, signal) {
            dispatched += 1;
        }
    }
    dispatched
}

#[cfg(feature = "std")]
fn trampoline_loop(pd_id: u32, state: Arc<PdState>) -> ! {
    loop {
        let signal = {
            let mut queue = state.queue.lock().unwrap_or_else(recover);
            loop {
                if let Some(signal) = queue.pop_front() {
                    break signal;
                }
                queue = state.parked.wait(queue).unwrap_or_else(recover);
            }
        };

        let _ = dispatch_signal(pd_id, signal);
    }
}

#[cfg(feature = "std")]
fn recover<T>(err: std::sync::PoisonError<T>) -> T {
    err.into_inner()
}

#[cfg(feature = "std")]
fn action_guard<'a>(state: &'a Arc<PdState>) -> std::sync::MutexGuard<'a, BTreeMap<u8, SigAction>> {
    state.actions.lock().unwrap_or_else(recover)
}

#[cfg(not(feature = "std"))]
fn action_guard<'a>(state: &'a Arc<PdState>) -> spin::MutexGuard<'a, BTreeMap<u8, SigAction>> {
    state.actions.lock()
}

#[cfg(feature = "std")]
fn read_states<'a>() -> std::sync::RwLockReadGuard<'a, BTreeMap<u32, Arc<PdState>>> {
    PD_STATES.read().unwrap_or_else(recover)
}

#[cfg(not(feature = "std"))]
fn read_states<'a>() -> spin::RwLockReadGuard<'a, BTreeMap<u32, Arc<PdState>>> {
    PD_STATES.read()
}

#[cfg(feature = "std")]
fn write_states<'a>() -> std::sync::RwLockWriteGuard<'a, BTreeMap<u32, Arc<PdState>>> {
    PD_STATES.write().unwrap_or_else(recover)
}

#[cfg(not(feature = "std"))]
fn write_states<'a>() -> spin::RwLockWriteGuard<'a, BTreeMap<u32, Arc<PdState>>> {
    PD_STATES.write()
}
