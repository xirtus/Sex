use core::sync::atomic::{AtomicU64, Ordering};

/// POSIX Signal ABI constants
pub const NSIG: usize = 64;
pub const SA_RESTART: u64 = 0x10000000;
pub const SA_SIGINFO: u64 = 0x00000004;
pub const SA_RESETHAND: u64 = 0x80000000;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SigAction {
    pub handler: u64,
    pub flags: u64,
    pub mask: u64,
}

/// Per-PD lock-free signal state.
pub struct SignalState {
    pub handlers: [AtomicU64; NSIG],
    pub flags: [AtomicU64; NSIG],
    pub masks: [AtomicU64; NSIG],
}

impl SignalState {
    pub const fn new() -> Self {
        const INIT_U64: AtomicU64 = AtomicU64::new(0);
        Self {
            handlers: [INIT_U64; NSIG],
            flags: [INIT_U64; NSIG],
            masks: [INIT_U64; NSIG],
        }
    }

    pub fn set_action(&self, signum: usize, action: SigAction) {
        if signum >= NSIG { return; }
        self.handlers[signum].store(action.handler, Ordering::Release);
        self.flags[signum].store(action.flags, Ordering::Release);
        self.masks[signum].store(action.mask, Ordering::Release);
    }

    pub fn get_action(&self, signum: usize) -> Option<SigAction> {
        if signum >= NSIG { return None; }
        let handler = self.handlers[signum].load(Ordering::Acquire);
        if handler == 0 { return None; }
        Some(SigAction {
            handler,
            flags: self.flags[signum].load(Ordering::Acquire),
            mask: self.masks[signum].load(Ordering::Acquire),
        })
    }
}

/// The trampoline dispatch logic.
/// Executed on the dedicated trampoline stack in user-space.
#[no_mangle]
pub extern "C" fn sexc_trampoline_dispatch(signum: i32, handler: u64) {
    // 1. Invoke the actual user handler
    let handler_fn: extern "C" fn(i32) = unsafe { core::mem::transmute(handler) };
    handler_fn(signum);

    // 2. Return to sexc loop via PDX sigreturn (syscall 15)
    unsafe {
        core::arch::asm!("syscall", in("rax") 15, in("rdi") signum);
    }
}
