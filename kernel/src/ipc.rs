use crate::capability::{CapabilityData, ProtectionDomain};
use crate::pku::Pkru;
use crate::servers::sexnet::route_remote_ipc;
use alloc::sync::Arc;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};
use x86_64::VirtAddr;

pub const MAX_DOMAINS: usize = 1024;
pub const LOCAL_NODE_ID: u32 = 1;
const PDX_MESSAGE_TAG_MASK: u64 = 0xFFFF_0000_0000_0000;
const PDX_MESSAGE_TAG: u64 = 0x5349_0000_0000_0000;
const PDX_MESSAGE_KIND_SHIFT: u64 = 48;
const PDX_MESSAGE_KIND_SIGNAL: u64 = 0x01;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    RawCall(u64),
    Signal(u8),
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdxMessage {
    pub target_pd: u32,
    pub msg_type: MessageType,
}

struct DomainRegistry {
    domains: [AtomicPtr<ProtectionDomain>; MAX_DOMAINS],
}

impl DomainRegistry {
    const fn new() -> Self {
        Self {
            domains: [AtomicPtr::new(ptr::null_mut()); MAX_DOMAINS],
        }
    }

    pub fn get(&self, id: &u32) -> Option<Arc<ProtectionDomain>> {
        let idx = *id as usize % MAX_DOMAINS;
        let ptr = self.domains[idx].load(Ordering::Acquire);
        if ptr.is_null() {
            None
        } else {
            // Safety: We assume domains are never deallocated in this SASOS prototype.
            // In a real system, we'd use hazard pointers or Arc::from_raw with care.
            unsafe {
                let arc = Arc::from_raw(ptr);
                let cloned = arc.clone();
                let _ = Arc::into_raw(arc); // Keep original alive
                Some(cloned)
            }
        }
    }

    pub fn insert(&self, id: u32, pd: Arc<ProtectionDomain>) {
        let idx = id as usize % MAX_DOMAINS;
        let ptr = Arc::into_raw(pd) as *mut _;
        self.domains[idx].store(ptr, Ordering::Release);
    }

    pub fn contains_key(&self, id: &u32) -> bool {
        let idx = *id as usize % MAX_DOMAINS;
        !self.domains[idx].load(Ordering::Acquire).is_null()
    }

    pub fn len(&self) -> usize {
        self.domains.iter().filter(|p| !p.load(Ordering::Acquire).is_null()).count()
    }
}

static WAIT_FREE_REGISTRY: DomainRegistry = DomainRegistry::new();

// Compatibility wrapper for DOMAIN_REGISTRY (Simulated read/write)
pub struct RegistryWrapper;
impl RegistryWrapper {
    pub fn read(&self) -> &DomainRegistry { &WAIT_FREE_REGISTRY }
    pub fn write(&self) -> &DomainRegistry { &WAIT_FREE_REGISTRY }
}

pub static DOMAIN_REGISTRY: RegistryWrapper = RegistryWrapper;
use crate::amdahl::GLOBAL_AMDAHL;
use crate::sunni::GLOBAL_SUNNI;
use crate::latency_guard;

pub const fn encode_signal_message(signal: u8) -> u64 {
    PDX_MESSAGE_TAG | (PDX_MESSAGE_KIND_SIGNAL << PDX_MESSAGE_KIND_SHIFT) | signal as u64
}

pub fn decode_message(target_pd: u32, raw: u64) -> PdxMessage {
    if (raw & PDX_MESSAGE_TAG_MASK) == PDX_MESSAGE_TAG {
        let kind = (raw >> PDX_MESSAGE_KIND_SHIFT) & 0xFF;
        if kind == PDX_MESSAGE_KIND_SIGNAL {
            return PdxMessage {
                target_pd,
                msg_type: MessageType::Signal(raw as u8),
            };
        }
    }

    PdxMessage {
        target_pd,
        msg_type: MessageType::RawCall(raw),
    }
}

pub fn enqueue_message(target_pd: &ProtectionDomain, message: PdxMessage) -> Result<(), &'static str> {
    match message.msg_type {
        MessageType::Signal(signal) => target_pd.signal_ring.enqueue(signal),
        MessageType::RawCall(_) => Err("IPC: raw call cannot be enqueued"),
    }
}

pub fn route_signal(target_pd_id: u32, signal: u8) -> Result<(), &'static str> {
    let registry = DOMAIN_REGISTRY.read();
    let target_pd = registry
        .get(&target_pd_id)
        .ok_or("IPC: Target domain not found")?;

    enqueue_message(
        target_pd.as_ref(),
        PdxMessage {
            target_pd: target_pd_id,
            msg_type: MessageType::Signal(signal),
        },
    )
}

pub fn forward_interrupt_message(target_pd_id: u32, message: MessageType) -> Result<(), &'static str> {
    let registry = DOMAIN_REGISTRY.read();
    let target_pd = registry
        .get(&target_pd_id)
        .ok_or("IPC: Target domain not found")?;

    enqueue_message(
        target_pd.as_ref(),
        PdxMessage {
            target_pd: target_pd_id,
            msg_type: message,
        },
    )
}

/// A "Safe" PDX call that validates a capability before switching domains.
pub fn safe_pdx_call(caller: &ProtectionDomain, cap_id: u32, arg0: u64) -> Result<u64, &'static str> {
    let start_cycles = x86_64::instructions::port::Port::<u32>::new(0).read() as u64;

    // 1. Look up the capability in the caller's table
    let cap = caller.cap_table.find(cap_id)
        .ok_or("IPC: Capability not found")?;

    match cap.data {
        CapabilityData::IPC(ipc_data) => {
            if ipc_data.node_id != LOCAL_NODE_ID {
                return Ok(route_remote_ipc(ipc_data.node_id, ipc_data.target_pd_id, arg0));
            }

            let start_lock = x86_64::instructions::port::Port::<u32>::new(0).read() as u64;
            let registry = DOMAIN_REGISTRY.read();
            let end_lock = x86_64::instructions::port::Port::<u32>::new(0).read() as u64;

            let target_pd = registry.get(&ipc_data.target_pd_id)
                .ok_or("IPC: Target domain not found")?;

            let message = decode_message(ipc_data.target_pd_id, arg0);
            if !matches!(message.msg_type, MessageType::RawCall(_)) {
                enqueue_message(target_pd.as_ref(), message)?;
                latency_guard::verify_latency("IPC_FAST_PATH", start_cycles, false);
                GLOBAL_AMDAHL.record_event(0, end_lock - start_lock);
                GLOBAL_SUNNI.update_scale(100, 50);
                return Ok(0);
            }

            let target_mask = target_pd.current_pkru_mask.load(Ordering::SeqCst);

            let result = unsafe {
                pdx_call_with_mask(target_mask, ipc_data.entry_point, arg0)
            };

            // Record performance data & Enforce Latency Guard
            latency_guard::verify_latency("IPC_FAST_PATH", start_cycles, false);
            GLOBAL_AMDAHL.record_event(0, end_lock - start_lock); 
            GLOBAL_SUNNI.update_scale(100, 50);

            Ok(result)
        },

        // Handle Node capabilities (Unified sexvfs interface)
        CapabilityData::Node(node_data) => {
            if node_data.node_id != LOCAL_NODE_ID {
                // Route to sexnet Stack for remote invocation
                return Ok(route_remote_ipc(node_data.node_id, node_data.sexdrive_pd_id, arg0));
            }

            let registry = DOMAIN_REGISTRY.read();
            let target_pd = registry.get(&node_data.sexdrive_pd_id)
                .ok_or("IPC: sexdrive domain not found")?;

            // We'll need a way to look up the entry point for the sexdrive's read/write 
            // operations. For this prototype, we'll assume a standard sexdrive entry point.
            // In a real system, the sexdrive would register its entry points in the capability.
            let sexdrive_entry = VirtAddr::new(0x2000_0000); // Placeholder sexdrive entry
            
            let target_mask = target_pd.current_pkru_mask.load(Ordering::SeqCst);
            
            let result = unsafe {
                pdx_call_with_mask(target_mask, sexdrive_entry, arg0)
            };

            latency_guard::verify_latency("STORAGE_IO_PATH", start_cycles, true);
            Ok(result)
        },
        // Handle Distributed Proxies (SASOS Memory Fabric)
        CapabilityData::RemoteProxy(remote_data) => {
            // IPCtax Mandate: Verify distributed consensus before RDMA dispatch
            if !crate::servers::sexnode::GLOBAL_CONSENSUS.is_valid(&remote_data) {
                return Err("IPC: Remote capability is from a dead epoch (Node failure).");
            }

            serial_println!("IPC: Triggering Distributed Proxy for Node {}.", remote_data.node_id);
            Ok(route_remote_ipc(remote_data.node_id, remote_data.local_id, arg0))
        },
        _ => Err("IPC: Capability does not support PDX calls"),
    }
}

/// A "Fused" PDX call that bypasses capability validation for hot-paths.
/// This fulfills the "Domain Fusion" requirement in IPCtax.txt.
pub fn fused_pdx_call(target: &ProtectionDomain, entry_point: VirtAddr, arg0: u64) -> u64 {
    // No capability check; assumes caller is already "fused" with target (has its PKU key enabled).
    let target_mask = target.current_pkru_mask.load(Ordering::SeqCst);
    unsafe {
        pdx_call_with_mask(target_mask, entry_point, arg0)
    }
}

/// The hardware-accelerated PDX primitive with explicit mask.
pub unsafe fn pdx_call_with_mask(target_pkru: u32, entry_point: VirtAddr, arg0: u64) -> u64 {
    let old_pkru = Pkru::read();
    
    let result: u64;

    core::arch::asm!(
        "mov {tmp_pkru}, eax", 
        "mov eax, {target_pkru_val}",
        "xor edx, edx",
        "xor ecx, ecx",
        "wrpkru",              
        "mov rdi, {arg0_val}", 
        "call {entry_val}",    
        "mov {res_val}, rax",  
        "mov eax, {tmp_pkru}",
        "xor edx, edx",
        "xor ecx, ecx",
        "wrpkru",              
        target_pkru_val = in(reg) target_pkru,
        entry_val = in(reg) entry_point.as_u64(),
        arg0_val = in(reg) arg0,
        tmp_pkru = out(reg) _,
        res_val = out(reg) result,
        in("eax") old_pkru,
        out("rdx") _,
        out("rcx") _,
        out("rdi") _,
        out("rax") _,
    );

    result
}
