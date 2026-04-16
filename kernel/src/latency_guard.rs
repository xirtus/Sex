use crate::amdahl::GLOBAL_AMDAHL;
use crate::serial_println;

/// The Latency Guard: Enforces IPCtax.txt mandates.
/// Measures every PDX call and audits drivers that exceed their budget.

pub const FAST_PATH_BUDGET: u64 = 500;
pub const IO_PATH_BUDGET: u64 = 2500;

pub fn verify_latency(driver_name: &str, start_cycles: u64, is_io: bool) {
    let end_cycles = unsafe { x86_64::instructions::port::Port::<u32>::new(0).read() as u64 };
    let duration = end_cycles - start_cycles;
    let budget = if is_io { IO_PATH_BUDGET } else { FAST_PATH_BUDGET };

    if duration > budget {
        serial_println!("IPCTAX VIOLATION: [{}] exceeded budget ({} > {})", 
            driver_name, duration, budget);
        // Log to Amdahl for serial fraction analysis
        GLOBAL_AMDAHL.record_event(duration, duration - budget);
    }
}
