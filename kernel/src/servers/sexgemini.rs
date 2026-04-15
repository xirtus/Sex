use crate::serial_println;
use crate::interrupts::{FAULT_RING, SystemFaultEvent};
use crate::capability::{ProtectionDomain, CapabilityData, PciCapData};
use crate::ipc::DOMAIN_REGISTRY;
use alloc::sync::Arc;
use alloc::string::String;

/// sex-gemini: The Autonomous Self-Repair Daemon for SexOS.
/// This service runs with Key 0 (Root SAS) and heals the system in real-time.

pub struct SexGemini {
    pub name: &'static str,
    pub pku_key: u8, // Should be 0 for full SAS access
}

impl SexGemini {
    pub fn new() -> Self {
        Self {
            name: "sex-gemini",
            pku_key: 0,
        }
    }

    /// The main repair loop. Intercepts faults from the Global SAS and remediates.
    pub fn run_repair_loop(&self) {
        serial_println!("sex-gemini: Autonomous Self-Repair active (Key 0).");
        
        loop {
            if let Some(event) = FAULT_RING.dequeue() {
                serial_println!("sex-gemini: Intercepted Fault for PD {} at {:#x}", 
                    event.pd_id, event.fault_addr);
                
                // 1. Diagnostic Analysis (RIP & Resource)
                let fix = self.diagnose_and_heal(event);
                
                // 2. Execution of remediation
                if let Some(remediation) = fix {
                    self.apply_remediation(event.pd_id, remediation);
                }
            }
            
            // Wait for more system telemetry
            x86_64::instructions::hlt();
        }
    }

    /// Analyzes the fault and determines the required capability patch.
    fn diagnose_and_heal(&self, event: SystemFaultEvent) -> Option<CapabilityData> {
        // SAS Heuristic: If fault is in the MMIO range, the PD likely lacks a BAR capability.
        if event.fault_addr >= 0x_A000_0000_0000 && event.fault_addr < 0x_B000_0000_0000 {
            serial_println!("sex-gemini: [HEALING] Missing MMIO capability detected.");
            
            // In a production system, we'd lookup the PCI database
            return Some(CapabilityData::Pci(PciCapData {
                bus: 0, dev: 0, func: 0, vendor_id: 0x8086, device_id: 0x100E // e1000 example
            }));
        }
        
        None
    }

    /// Applies the patch to the PD and signals sexit to unpark the task.
    fn apply_remediation(&self, pd_id: u32, cap: CapabilityData) {
        let registry = DOMAIN_REGISTRY.read();
        if let Some(pd) = registry.get(&pd_id) {
            // 1. Dynamically update the Capability Engine
            let cap_id = pd.grant(cap);
            serial_println!("sex-gemini: [REPAIRED] PD {} granted Cap ID {}.", pd_id, cap_id);
            
            // 2. Signal sexit to unpark and retry the instruction (RIP)
            // In SASOS, we simply unblock the task and it retries the load/store.
            unsafe {
                if let Some(ref mut sched) = crate::scheduler::SCHEDULERS[0] {
                    sched.unblock(pd_id);
                }
            }
            
            serial_println!("sex-gemini: [RESUMED] Task {} unparked. Faulting instruction will retry.", pd_id);
        }
    }
}

pub extern "C" fn sexgemini_entry(arg: u64) -> u64 {
    serial_println!("sex-gemini PDX: Received supervisor request {:#x}", arg);
    0
}
