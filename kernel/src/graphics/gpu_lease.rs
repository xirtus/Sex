use crate::capability::DisplayHardwareLease;
use spin::Mutex;
use core::sync::atomic::{AtomicBool, Ordering};

pub static PRIMARY_GPU_LEASE: Mutex<Option<DisplayHardwareLease>> = Mutex::new(None);
static ALREADY_CLAIMED: AtomicBool = AtomicBool::new(false);
static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn select_primary_gpu() -> Option<DisplayHardwareLease> {
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        // Return None if already initialized to prevent re-selection paths
        return None;
    }

    let devices = crate::drivers::pci::enumerate_display_devices();

    // SINGLE PASS deterministic selection (No fallback logic, no hotplug)
    let mut intel = None;
    let mut amd = None;
    let mut virtio = None;
    let mut fallback = None;

    for d in devices.iter() {
        match d.vendor_id {
            0x8086 => { if intel.is_none() { intel = Some(d); } }
            0x1002 => { if amd.is_none() { amd = Some(d); } }
            0x1af4 => { if virtio.is_none() { virtio = Some(d); } }
            _ => { if fallback.is_none() { fallback = Some(d); } }
        }
    }

    let chosen = intel.or(amd).or(virtio).or(fallback)?;

    Some(DisplayHardwareLease {
        domain: 0,
        bus: chosen.bus,
        dev: chosen.dev,
        func: chosen.func,
        vendor_id: chosen.vendor_id,
        device_id: chosen.device_id,
    })
}

pub fn claim_primary_for_pd1() -> DisplayHardwareLease {
    if ALREADY_CLAIMED.swap(true, Ordering::SeqCst) {
        panic!("FATAL: DisplayHardwareLease already claimed");
    }
    PRIMARY_GPU_LEASE.lock().take().expect("FATAL: PRIMARY_GPU_LEASE is None (Ownership Violation)")
}
