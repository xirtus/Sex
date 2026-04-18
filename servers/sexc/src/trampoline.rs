use libsys::messages::MessageType;
use core::sync::atomic::{AtomicBool, Ordering};

// Global signal state (will become per-PD later)
pub static SIGNAL_STATE: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy)]
pub struct SigAction {
    pub sa_handler: usize, // C function pointer
    pub sa_flags: u64,
}

pub fn start_signal_trampoline() {
    // Register the background trampoline with the PDX system
    // (using the existing safe_pdx_register API in your tree)
    // For Phase 24 we stub the actual listen loop — it will be expanded in Phase 25
    SIGNAL_STATE.store(true, Ordering::Release);
    // In final version this will spawn a real background thread that calls safe_pdx_listen
    // and dispatches MessageType::Signal without kernel stack surgery
}

pub fn sexc_trampoline_entry() {
    // Placeholder — the real loop will live here once PDX listen is wired
    // This is the "trampoline" that receives MessageType::Signal via PDX ring
}
