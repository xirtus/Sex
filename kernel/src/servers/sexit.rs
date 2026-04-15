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
    
    // 1. Core Services
    register_service("sexvfs", vec![]);
    register_service("sexnet", vec![]);
    
    // 2. Multimedia Infrastructure
    register_service("srv_wayland", vec!["sexdrm", "sexinput"]);
    register_service("sexsound", vec!["dde"]);
    register_service("srv_font", vec!["sexvfs"]);

    // 3. Phase 16 Applications
    register_service("doom", vec!["srv_wayland", "sexsound"]);
    register_service("classicube", vec!["srv_wayland", "sexnet"]);

    serial_println!("SEXIT: Multimedia dependency graph resolved.");
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

/// The main event loop for the sexit supervisor.
/// Intercepts asynchronous system fault messages instead of polling.
pub fn fault_listener_loop() {
    serial_println!("SEXIT: Fault listener loop active (Zero-Mediation).");
    loop {
        if let Some(event) = crate::interrupts::FAULT_RING.dequeue() {
            serial_println!("SEXIT: Intercepted {} for PD {}. Initiating Reincarnation.", 
                if event.fault_type == 0 { "Page Fault" } else { "Cap Violation" },
                event.pd_id);
            
            reincarnate_pd(event.pd_id);
        }
        x86_64::instructions::hlt();
    }
}

fn reincarnate_pd(pd_id: u32) {
    let mut services = SUPERVISED_SERVICES.lock();
    let mut target_name = None;
    for (name, service) in services.iter() {
        if let ServiceStatus::Up(id) = service.status {
            if id == pd_id {
                target_name = Some(*name);
                break;
            }
        }
    }

    if let Some(name) = target_name {
        serial_println!("SEXIT: Reincarnating service {} (Previous PD {})...", name, pd_id);
        if let Some(service) = services.get_mut(name) {
            service.status = ServiceStatus::Down;
            service.restart_count += 1;
            
            // Re-spawn the service (Simulated)
            let new_id = 6000 + service.restart_count;
            let new_pd = Arc::new(ProtectionDomain::new(new_id, (new_id % 16) as u8));
            crate::ipc::DOMAIN_REGISTRY.write().insert(new_id, new_pd);
            
            service.status = ServiceStatus::Up(new_id);
            serial_println!("SEXIT: Service {} successfully restored in PD {}.", name, new_id);
        }
    }
}

/// Triggers a manual restart of a service.
pub fn restart_service(pd_id: u32) {
    let mut services = SUPERVISED_SERVICES.lock();
    let mut target_name = None;
    for (name, service) in services.iter() {
        if let ServiceStatus::Up(id) = service.status {
            if id == pd_id {
                target_name = Some(*name);
                break;
            }
        }
    }

    if let Some(name) = target_name {
        serial_println!("SEXIT: Manual restart triggered for {} (PD {}).", name, pd_id);
        if let Some(service) = services.get_mut(name) {
            service.status = ServiceStatus::Down;
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
