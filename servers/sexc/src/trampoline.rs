use core::sync::atomic::{AtomicPtr, Ordering};
use alloc::sync::Arc;
use crate::ipc_ring::RingBuffer;
use crate::xipc::messages::MessageType;

/// Per-PD Signal State as specified in IPCtax §4.2.
pub struct SignalState {
    pub handlers: [AtomicPtr<()>; 32],
    pub flags: [u64; 32],
    pub sigmask: u64,
    pub altstack: *mut u8,
}

impl SignalState {
    pub fn new() -> Self {
        const ATOMIC_NULL: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());
        Self {
            handlers: [ATOMIC_NULL; 32],
            flags: [0; 32],
            sigmask: 0,
            altstack: core::ptr::null_mut(),
        }
    }
}

/// Trampoline entry point. Blocks until a signal message arrives.
pub extern "C" fn trampoline_entry(control_ring_ptr: *mut RingBuffer<MessageType, 256>) {
    let ring = unsafe { &*control_ring_ptr };
    loop {
        // BLOCK (FLSCHED park/unpark) until a message arrives
        while ring.is_empty() {
            crate::scheduler::park_current_thread();
        }

        if let Ok(msg) = ring.dequeue() {
            match msg {
                MessageType::Signal { signo, sender_capability_id } => {
                    dispatch_signal(signo, sender_capability_id);
                }
                _ => {}
            }
        }
    }
}

/// Dispatches the signal to the registered C handler.
fn dispatch_signal(signo: u32, sender_cap_id: u64) {
    let pd = crate::core_local::CoreLocal::get().current_pd_ref();
    let sexc_state_lock = pd.sexc_state.lock();
    let sexc_state = sexc_state_lock.as_ref().expect("sexc: No state for signal dispatch");
    let state = &sexc_state.signal_state;
    
    let signo_idx = (signo as usize) % 32;
    let handler_ptr = state.handlers[signo_idx].load(Ordering::Acquire);
    
    if handler_ptr.is_null() {
        // Default actions for critical signals
        match signo {
            2 | 9 | 15 => { // SIGINT, SIGKILL, SIGTERM
                crate::servers::sexc::sys_exit(0);
            }
            _ => {}
        }
        return;
    }

    let flags = state.flags[signo_idx];
    
    // Support for SA_RESETHAND
    if flags & 0x04000000 != 0 {
        state.handlers[signo_idx].store(core::ptr::null_mut(), Ordering::Release);
    }

    // Support for SA_NODEFER (Simplified mask management)
    if flags & 0x40000000 == 0 {
        // Mask signal during execution
    }

    unsafe {
        // Invoke C handler on the trampoline's dedicated stack context
        // Prototype assumes the current thread is already on the trampoline stack.
        let handler = core::mem::transmute::<*mut (), extern "C" fn(i32, *mut (), *mut ())>(handler_ptr);
        handler(signo as i32, core::ptr::null_mut(), core::ptr::null_mut());
    }
}

pub fn register_sigaction(signo: i32, handler: usize, flags: u64) {
    let pd = crate::core_local::CoreLocal::get().current_pd_ref();
    let sexc_state_lock = pd.sexc_state.lock();
    if let Some(ref sexc) = *sexc_state_lock {
        let idx = (signo as usize) % 32;
        sexc.signal_state.handlers[idx].store(handler as *mut (), Ordering::Release);
        sexc.signal_state.flags[idx] = flags;
    }
}
