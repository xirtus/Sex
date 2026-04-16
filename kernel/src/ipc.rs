use crate::capability::{CapabilityData, ProtectionDomain};
use crate::pku::Pkru;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};
use x86_64::VirtAddr;

pub mod messages;
pub mod pagefault;
pub mod router;

pub const MAX_DOMAINS: usize = 1024;
pub const LOCAL_NODE_ID: u32 = 1;

/// A Lock-Free, Wait-Free Domain Registry for SASOS.
/// IPCtax-Compliant: Sharded Atomic Array (No Locks, No Arc).
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

    pub fn get(&self, id: u32) -> Option<&'static ProtectionDomain> {
        let idx = id as usize % MAX_DOMAINS;
        let ptr = self.domains[idx].load(Ordering::Acquire);
        if ptr.is_null() {
            None
        } else {
            unsafe { Some(&*ptr) }
        }
    }

    pub fn insert(&self, id: u32, pd: *mut ProtectionDomain) {
        let idx = id as usize % MAX_DOMAINS;
        let old = self.domains[idx].swap(pd, Ordering::AcqRel);
        if !old.is_null() {
            // RCU reclamation omitted
        }
    }

    pub fn len(&self) -> usize {
        self.domains.iter().filter(|p| !p.load(Ordering::Acquire).is_null()).count()
    }
}

pub static DOMAIN_REGISTRY: DomainRegistry = DomainRegistry::new();

/// The hardware-accelerated PDX primitive with explicit mask.
pub unsafe fn pdx_call_with_mask(target_pkru: u32, entry_point: VirtAddr, arg0: u64) -> u64 {
    let old_pkru = Pkru::read();
    let result: u64;

    core::arch::asm!(
        "mov {tmp_pkru:e}, eax", 
        "mov eax, {target_pkru_val:e}",
        "xor edx, edx", "xor ecx, ecx",
        "wrpkru",              
        "mov rdi, {arg0_val}", 
        "call {entry_val}",    
        "mov {tmp_pkru:e}, eax",
        "xor edx, edx", "xor ecx, ecx",
        "wrpkru",              
        target_pkru_val = in(reg) target_pkru,
        entry_val = in(reg) entry_point.as_u64(),
        arg0_val = in(reg) arg0,
        tmp_pkru = out(reg) _,
        inout("rax") 0u64 => result,
        out("rdx") _, out("rcx") _, out("rdi") _,
    );

    result
}

pub fn safe_pdx_call(cap_id: u32, arg0: u64) -> Result<u64, &'static str> {
    let current_pd = crate::core_local::CoreLocal::get().current_pd_ref();
    let cap = unsafe { (*current_pd.cap_table).find(cap_id).ok_or("IPC: Invalid cap")? };
    
    match cap.data {
        CapabilityData::IPC(data) => {
            let target_pd = DOMAIN_REGISTRY.get(data.target_pd_id).ok_or("IPC: Target lost")?;
            let target_mask = target_pd.current_pkru_mask.load(Ordering::Acquire);
            unsafe {
                Ok(pdx_call_with_mask(target_mask, data.entry_point, arg0))
            }
        },
        CapabilityData::Domain(target_pd_id) => {
            let target_pd = DOMAIN_REGISTRY.get(target_pd_id).ok_or("IPC: Target lost")?;
            let target_mask = target_pd.current_pkru_mask.load(Ordering::Acquire);
            let entry = VirtAddr::new(0x_4000_0000);
            unsafe {
                Ok(pdx_call_with_mask(target_mask, entry, arg0))
            }
        },
        _ => Err("IPC: Not a PDX capability"),
    }
}

pub fn resolve_phys_pdx(cap_id: u32) -> u64 {
    let current_pd = crate::core_local::CoreLocal::get().current_pd_ref();
    if let Some(cap) = unsafe { (*current_pd.cap_table).find(cap_id) } {
        match cap.data {
            CapabilityData::MemLend(data) => return data.base,
            CapabilityData::DMA(data) => return data.phys_addr,
            _ => (),
        }
    }
    0
}

pub fn route_signal(target_pd_id: u32, signal: u8) -> Result<(), &'static str> {
    let target_pd = DOMAIN_REGISTRY.get(target_pd_id)
        .ok_or("IPC: Target not found")?;

    unsafe { (*target_pd.signal_ring).enqueue(signal) }
}
