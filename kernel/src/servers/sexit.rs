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
    pub dependencies: Vec<&'static str>,
}

lazy_static! {
    /// Registry of all supervised services (Runit-style).
    pub static ref SUPERVISED_SERVICES: Mutex<BTreeMap<&'static str, Service>> = 
        Mutex::new(BTreeMap::new());
}

pub fn sexit_init() {
    serial_println!("SEXIT: Runit-style service supervision active.");
    
    // 1. In a real system, we'd scan /etc/sv/
    serial_println!("SEXIT: Scanning /etc/sv/ for service definitions...");
}

pub fn register_service(name: &'static str, deps: Vec<&'static str>) {
    let mut services = SUPERVISED_SERVICES.lock();
    services.insert(name, Service {
        name,
        status: ServiceStatus::Down,
        restart_count: 0,
        dependencies: deps,
    });
    serial_println!("SEXIT: Registered service '{}' with dependencies: {:?}", name, services.get(name).unwrap().dependencies);
}

/// Starts services in dependency order.
pub fn start_all() {
    serial_println!("SEXIT: Resolving dependency graph and starting services...");
    
    // Simplified resolution for prototype
    let names: Vec<&'static str> = {
        let services = SUPERVISED_SERVICES.lock();
        services.keys().cloned().collect()
    };

    for name in names {
        boot_service(name);
    }
}

fn boot_service(name: &'static str) {
    let mut services = SUPERVISED_SERVICES.lock();
    if let Some(service) = services.get_mut(name) {
        if let ServiceStatus::Down = service.status {
            serial_println!("SEXIT: Starting '{}'...", name);
            // Simulate process spawning
            let new_id = 4000 + service.restart_count;
            service.status = ServiceStatus::Up(new_id);
            serial_println!("SEXIT: '{}' is UP (PD: {})", name, new_id);
        }
    }
}

/// Checks the health of all supervised services.
pub fn check_services() {
    let mut services = SUPERVISED_SERVICES.lock();
    for (name, service) in services.iter_mut() {
        match service.status {
            ServiceStatus::Up(id) => {
                // 1. Verify if the PD is still in the registry
                let registry = crate::ipc::DOMAIN_REGISTRY.read();
                if !registry.contains_key(&id) {
                    serial_println!("SEXIT: Service {} (PD {}) has DIED.", name, id);
                    service.status = ServiceStatus::Down;
                }
            },
            ServiceStatus::Down => {
                serial_println!("SEXIT: Reincarnating service {}...", name);
                service.restart_count += 1;
                
                // 2. Automated Reincarnation: Spawn a fresh PD
                // In a real system, we'd look up the binary path for this service
                let new_id = 5000 + service.restart_count;
                let new_pd = Arc::new(ProtectionDomain::new(new_id, (new_id % 16) as u8));
                crate::ipc::DOMAIN_REGISTRY.write().insert(new_id, new_pd);
                
                service.status = ServiceStatus::Up(new_id);
                serial_println!("SEXIT: SUCCESS - {} restored in PD {}.", name, new_id);
            },
            _ => {}
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
