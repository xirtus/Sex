use core::sync::atomic::{AtomicU64, Ordering};
use crate::serial_println;

/// Amdahl: Serial Fraction Profiler.
/// Measures the ratio of serial (locked) time to parallel (logic) time 
/// to predict the theoretical speedup limit of the nameserver.

pub struct AmdahlTracker {
    pub total_ipc_cycles: AtomicU64,
    pub serial_locked_cycles: AtomicU64,
}

impl AmdahlTracker {
    pub const fn new() -> Self {
        Self {
            total_ipc_cycles: AtomicU64::new(0),
            serial_locked_cycles: AtomicU64::new(0),
        }
    }

    /// Records an IPC event and the time spent in a serialized lock.
    pub fn record_event(&self, total: u64, locked: u64) {
        self.total_ipc_cycles.fetch_add(total, Ordering::Relaxed);
        self.serial_locked_cycles.fetch_add(locked, Ordering::Relaxed);
    }

    /// Calculates the serial fraction (s) and maximum possible speedup.
    pub fn report_analysis(&self) {
        let total = self.total_ipc_cycles.load(Ordering::Relaxed);
        let locked = self.serial_locked_cycles.load(Ordering::Relaxed);
        
        if total == 0 { return; }

        let s = (locked as f64) / (total as f64);
        let max_speedup = 1.0 / s;

        serial_println!("--- Amdahl Scalability Analysis ---");
        serial_println!("Serial Fraction (s): {:.4}", s);
        serial_println!("Maximum Speedup: {:.2}x (Amdahl's Law)", max_speedup);
        
        if s > 0.1 {
            serial_println!("WARNING: Serial fraction is high (>10%). IPC will not scale past 10 cores.");
        } else {
            serial_println!("OPTIMIZED: Serial fraction is low. System is ready for Hypercore scaling.");
        }
    }
}

pub static GLOBAL_AMDAHL: AmdahlTracker = AmdahlTracker::new();
