use crate::serial_println;
use crate::capability::ProtectionDomain;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use spin::Mutex;
use lazy_static::lazy_static;

/// Sexit: Simple Service Supervisor for the Sex Microkernel.
/// A PDX-driven, isolated manager that avoids the monolithic complexity of systemd.

pub enum ServiceStatus {
    Down,
    Up(u32), // PD ID
    Restarting,
}

pub struct Service {
    pub name: &'static str,
    pub status: ServiceStatus,
    pub restart_count: u32,
}

lazy_static! {
    /// Registry of all supervised services.
    pub static ref SUPERVISED_SERVICES: Mutex<BTreeMap<&'static str, Service>> = 
        Mutex::new(BTreeMap::new());
}

pub fn sexit_init() {
    serial_println!("SEXIT: Simple service supervision active.");
}

pub fn start_service(name: &'static str, pd_id: u32) {
    let mut services = SUPERVISED_SERVICES.lock();
    services.insert(name, Service {
        name,
        status: ServiceStatus::Up(pd_id),
        restart_count: 0,
    });
    serial_println!("SEXIT: Supervising service: {} (PD: {})", name, pd_id);
}

pub fn check_services() {
    let mut services = SUPERVISED_SERVICES.lock();
    for (name, service) in services.iter_mut() {
        match service.status {
            ServiceStatus::Up(id) => {
                // In a real system, we'd check if the PD is still alive
                // serial_println!("SEXIT: {} is healthy.", name);
            },
            _ => {
                serial_println!("SEXIT: Service {} is DOWN. Restarting...", name);
                service.restart_count += 1;
                // Trigger PD restart...
            }
        }
    }
}

/// The sexit entry point for PDX calls.
pub extern "C" fn sexit_entry(arg: u64) -> u64 {
    // 1: Status, 2: Start, 3: Stop
    let cmd = (arg >> 32) as u32;
    serial_println!("SEXIT: Received command {}", cmd);
    0
}
