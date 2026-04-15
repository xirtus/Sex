use crate::capability::{CapabilityData, ProtectionDomain};
use crate::pku::Pkru;
use alloc::sync::Arc;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};
use x86_64::VirtAddr;

pub mod messages;
pub mod pagefault;
pub mod router;

pub const MAX_DOMAINS: usize = 1024;
pub const LOCAL_NODE_ID: u32 = 1;

/// A Lock-Free, Wait-Free Domain Registry for SASOS.
/// IPCtax-Compliant: Sharded Atomic Array (No Locks).
pub struct DomainRegistry {
    pub domains: [AtomicPtr<ProtectionDomain>; MAX_DOMAINS],
}

impl DomainRegistry {
    pub const fn new() -> Self {
        const INIT: AtomicPtr<ProtectionDomain> = AtomicPtr::new(ptr::null_mut());
        Self {
            domains: [INIT; MAX_DOMAINS],
        }
    }

    pub fn get(&self, id: u32) -> Option<Arc<ProtectionDomain>> {
        let idx = id as usize % MAX_DOMAINS;
        let ptr = self.domains[idx].load(Ordering::Acquire);
        if ptr.is_null() {
            None
        } else {
            // Safety: We assume domains are never deallocated in this SASOS prototype.
            unsafe {
                let arc = Arc::from_raw(ptr);
                let cloned = arc.clone();
                let _ = Arc::into_raw(arc); // Re-increment refcount for registry
                Some(cloned)
            }
        }
    }

    pub fn insert(&self, id: u32, pd: Arc<ProtectionDomain>) {
        let idx = id as usize % MAX_DOMAINS;
        let ptr = Arc::into_raw(pd) as *mut _;
        let old = self.domains[idx].swap(ptr, Ordering::AcqRel);
        if !old.is_null() {
            // In a real system, we'd defer the deletion of 'old' via Epochs
        }
    }

    pub fn len(&self) -> usize {
        self.domains.iter().filter(|p| !p.load(Ordering::Acquire).is_null()).count()
    }
}

pub static DOMAIN_REGISTRY: DomainRegistry = DomainRegistry::new();

/// The hardware-accelerated PDX primitive with explicit mask.
/// This fulfills the "Domain Fusion" requirement in IPCtax.txt.
pub unsafe fn pdx_call_with_mask(target_pkru: u32, entry_point: VirtAddr, arg0: u64) -> u64 {
    let old_pkru = Pkru::read();
    let result: u64;

    core::arch::asm!(
        "mov {tmp_pkru}, eax", 
        "mov eax, {target_pkru_val}",
        "xor edx, edx", "xor ecx, ecx",
        "wrpkru",              
        "mov rdi, {arg0_val}", 
        "call {entry_val}",    
        "mov {res_val}, rax",  
        "mov eax, {tmp_pkru}",
        "xor edx, edx", "xor ecx, ecx",
        "wrpkru",              
        target_pkru_val = in(reg) target_pkru,
        entry_val = in(reg) entry_point.as_u64(),
        arg0_val = in(reg) arg0,
        tmp_pkru = out(reg) _,
        res_val = out(reg) result,
        in("eax") old_pkru,
        out("rdx") _, out("rcx") _, out("rdi") _, out("rax") _,
    );

    result
}

pub fn safe_pdx_call(target: &ProtectionDomain, cap_id: u32, arg0: u64) -> Result<u64, &'static str> {
    // 1. Validate Capability (Lock-Free)
    let cap = target.cap_table.find(cap_id).ok_or("IPC: Invalid cap")?;
    
    // 2. Perform Switch
    let target_mask = target.current_pkru_mask.load(Ordering::Acquire);
    let entry = match cap.data {
        CapabilityData::IPC(data) => data.entry_point,
        _ => return Err("IPC: Not a PDX capability"),
    };

    unsafe {
        Ok(pdx_call_with_mask(target_mask, entry, arg0))
    }
}

pub fn route_signal(target_pd_id: u32, signal: u8) -> Result<(), &'static str> {
    let target_pd = DOMAIN_REGISTRY.get(target_pd_id)
        .ok_or("IPC: Target not found")?;

    target_pd.signal_ring.enqueue(signal)
}
