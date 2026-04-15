use core::sync::atomic::{AtomicU64, Ordering};
use libsys::pdx::{pdx_listen, pdx_reply};
use libsys::messages::MessageType;

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

#[repr(C)]
pub struct SigInfo {
    pub signum: i32,
    pub code: i32,
    pub value: u64,
}

#[repr(C)]
pub struct UContext {
    pub stack: [u64; 2],
    pub mcontext: [u64; 32], // Simplified registers
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

pub static SIGNAL_STATE: SignalState = SignalState::new();

/// The background trampoline thread entry point.
/// IPCtax: Blocks with FLSCHED::park() on the control ring.
use libsys::sched::park_on_ring;

#[no_mangle]
pub extern "C" fn sexc_trampoline_entry() -> ! {
    loop {
        // 1. Wait-free park until signal message arrives
        park_on_ring();

        // 2. Poll control ring via PDX library
        let req = pdx_listen(0);
        let msg = unsafe { *(req.arg0 as *const MessageType) };

        if let MessageType::Signal(signum) = msg {
            dispatch_signal(signum as usize);
        }
        
        // 3. Signal handled, reply to kernel/sender
        pdx_reply(req.caller_pd, 0);
    }
}

fn dispatch_signal(signum: usize) {
    if let Some(action) = SIGNAL_STATE.get_action(signum) {
        // ABI: Construct siginfo and ucontext on dedicated stack
        let info = SigInfo { signum: signum as i32, code: 0, value: 0 };
        let ctx = UContext { stack: [0; 2], mcontext: [0; 32] };

        // Invoke handler
        if action.flags & SA_SIGINFO != 0 {
            let handler_fn: extern "C" fn(i32, *const SigInfo, *const UContext) = 
                unsafe { core::mem::transmute(action.handler) };
            handler_fn(signum as i32, &info, &ctx);
        } else {
            let handler_fn: extern "C" fn(i32) = unsafe { core::mem::transmute(action.handler) };
            handler_fn(signum as i32);
        }

        // Support SA_RESETHAND
        if action.flags & SA_RESETHAND != 0 {
            SIGNAL_STATE.handlers[signum].store(0, Ordering::Release);
        }

        // PDX Sigreturn (Syscall 15)
        unsafe { core::arch::asm!("syscall", in("rax") 15, in("rdi") signum); }
    }
}
