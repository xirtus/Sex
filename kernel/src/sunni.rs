use core::sync::atomic::{AtomicU64, Ordering};
use crate::serial_println;

/// Sunni: Memory-Bounded Throughput Simulator.
/// Evaluates how the nameserver scales as the number of capabilities 
/// and Protection Domains (problem size) increases to fill RAM.

pub struct SunniTracker {
    pub problem_size: AtomicU64, // Number of active capabilities
    pub latency_nanos: AtomicU64,
}

impl SunniTracker {
    pub const fn new() -> Self {
        Self {
            problem_size: AtomicU64::new(0),
            latency_nanos: AtomicU64::new(0),
        }
    }

    /// Records the current scaling state.
    pub fn update_scale(&self, size: u64, latency: u64) {
        self.problem_size.store(size, Ordering::Relaxed);
        self.latency_nanos.store(latency, Ordering::Relaxed);
    }

    /// Reports Sun-Ni scalability.
    pub fn report_analysis(&self) {
        let n = self.problem_size.load(Ordering::Relaxed);
        let l = self.latency_nanos.load(Ordering::Relaxed);

        serial_println!("--- Sun-Ni Scaling Analysis ---");
        serial_println!("Active Capabilities: {}", n);
        serial_println!("Avg Lookup Latency: {} ns", l);

        // Predict memory wall impact
        if l > 1000 {
            serial_println!("WARNING: Memory Wall detected. Nameserver is cache-starved.");
            serial_println!("SUGGESTION: Enable NUMA sharding for capability tables.");
        } else {
            serial_println!("SCALABLE: Problem size is efficiently balanced with memory bandwidth.");
        }
    }
}

pub static GLOBAL_SUNNI: SunniTracker = SunniTracker::new();
