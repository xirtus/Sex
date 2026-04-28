use crate::capability::{CapabilityData, ProtectionDomain};
use core::ptr;
use core::sync::atomic::{AtomicPtr, AtomicU8, Ordering};
use x86_64::VirtAddr;

pub mod messages;
pub mod pagefault;
pub mod router;
pub mod state;
pub mod buffer;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BootPhase {
    Init = 0,
    RegistryBuild = 1,
    RegistryFrozen = 2,
    SchedulerArmed = 3,
    SchedulerRunning = 4,
    ScanoutEnabled = 5,
}

pub struct BootController {
    phase: BootPhase,
}

impl BootController {
    pub const fn new() -> Self {
        Self { phase: BootPhase::Init }
    }

    #[inline(always)]
    pub fn phase(&self) -> BootPhase {
        self.phase
    }

    #[inline(always)]
    pub fn advance(&mut self, next: BootPhase) {
        use BootPhase::*;

        let valid = match (self.phase, next) {
            (Init, RegistryBuild)
            | (RegistryBuild, RegistryFrozen)
            | (RegistryFrozen, SchedulerArmed)
            | (SchedulerArmed, SchedulerRunning)
            | (SchedulerRunning, ScanoutEnabled) => true,
            _ => false,
        };

        assert!(valid, "BOOT_PHASE_VIOLATION");
        self.phase = next;
        BOOT_PHASE.store(next as u8, Ordering::SeqCst);
    }

    #[inline(always)]
    pub fn require(&self, min: BootPhase) {
        assert!(self.phase as u8 >= min as u8, "BOOT_PHASE_GATED");
    }
}

pub static mut BOOT_CONTROLLER: BootController = BootController::new();

pub static BOOT_PHASE: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);

pub fn get_phase() -> BootPhase {
    unsafe { BOOT_CONTROLLER.phase() }
}

pub const MAX_DOMAINS: usize = 1024;
pub const LOCAL_NODE_ID: u32 = 1;

/// Capability graph node registry.
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
        // Enforce Phase 1: Only during RegistryBuild
        let phase = BOOT_PHASE.load(Ordering::SeqCst);
        if phase != BootPhase::RegistryBuild as u8 {
            crate::serial_println!(
                "registry.mutation.denied registry=DOMAIN_REGISTRY op=insert domain_id={} phase={}",
                id,
                phase
            );
        }
        assert!(phase == BootPhase::RegistryBuild as u8, "REGISTRY_MUTATION_VIOLATION");

        let idx = id as usize % MAX_DOMAINS;
        self.domains[idx].store(pd, Ordering::Release);
    }

    pub fn len(&self) -> usize {
        self.domains.iter().filter(|p| !p.load(Ordering::Acquire).is_null()).count()
    }
}

pub static DOMAIN_REGISTRY: DomainRegistry = DomainRegistry::new();

pub unsafe fn pdx_call(entry_point: VirtAddr, arg0: u64, arg1: u64, arg2: u64) -> u64 {
    let result: u64;
    core::arch::asm!(
        "mov rdi, {arg0_val}",
        "mov rsi, {arg1_val}",
        "mov rdx, {arg2_val}",
        "call {entry_val}",
        entry_val = in(reg) entry_point.as_u64(),
        arg0_val  = in(reg) arg0,
        arg1_val  = in(reg) arg1,
        arg2_val  = in(reg) arg2,
        inout("rax") 0u64 => result,
        out("rdx") _, out("rcx") _, out("rdi") _, out("rsi") _,
    );
    result
}

pub enum GraphEdge {
    SyncCall     { entry_point: VirtAddr },
    AsyncEnqueue { ring: *mut crate::ipc_ring::RingBuffer<crate::ipc::messages::MessageType, 256> },
    AsyncDequeue {
        ring: *mut crate::ipc_ring::RingBuffer<crate::ipc::messages::MessageType, 256>,
        source: DequeueSource
    },
}

pub enum DequeueSource {
    MessageRing,
    InputRing,
}

pub fn resolve_edge(cap_data: CapabilityData, caller_pd_id: u32, is_listen: bool) -> Result<GraphEdge, u64> {
    match cap_data {
        CapabilityData::IPC(data) => {
            if is_listen { return Err(sex_pdx::ERR_CAP_INVALID); }
            Ok(GraphEdge::SyncCall { entry_point: data.entry_point })
        },
        CapabilityData::Domain(id) => {
            if is_listen { return Err(sex_pdx::ERR_CAP_INVALID); }
            let pd = DOMAIN_REGISTRY.get(id).ok_or(sex_pdx::ERR_CAP_INVALID)?;
            Ok(GraphEdge::AsyncEnqueue { ring: pd.message_ring })
        },
        CapabilityData::MessageQueue => {
            if !is_listen { return Err(sex_pdx::ERR_CAP_INVALID); }
            let pd = DOMAIN_REGISTRY.get(caller_pd_id).ok_or(sex_pdx::ERR_CAP_INVALID)?;
            Ok(GraphEdge::AsyncDequeue { ring: pd.message_ring, source: DequeueSource::MessageRing })
        },
        CapabilityData::InputRing => {
            if !is_listen { return Err(sex_pdx::ERR_CAP_INVALID); }
            Ok(GraphEdge::AsyncDequeue {
                ring: &crate::interrupts::INPUT_RING as *const _ as *mut _,
                source: DequeueSource::InputRing
            })
        },
        _ => Err(sex_pdx::ERR_CAP_INVALID),
    }
}

pub fn traverse_edge(edge: GraphEdge, opcode: u64, arg0: u64, arg1: u64, arg2: u64, caller_pd: u32) -> Result<u64, u64> {
    match edge {
        GraphEdge::SyncCall { entry_point } =>
            unsafe { Ok(pdx_call(entry_point, arg0, arg1, arg2)) },
        GraphEdge::AsyncEnqueue { ring } => {
            let msg = crate::ipc::messages::MessageType::IpcCall {
                func_id: opcode, arg0, arg1, arg2, caller_pd,
            };
            unsafe { (*ring).enqueue(msg).map(|_| 0u64).map_err(|_| sex_pdx::ERR_SERVICE_NOT_READY) }
        },
        GraphEdge::AsyncDequeue { ring: _, source: _ } => {
            Err(sex_pdx::ERR_CAP_INVALID)
        }
    }
}

pub fn safe_pdx_call(cap_id: u32, opcode: u64, arg0: u64, arg1: u64, arg2: u64) -> Result<u64, u64> {
    let current_pd = crate::core_local::CoreLocal::get().current_pd_ref();
    let caller_pd_id = current_pd.id;
    let cap = unsafe { (*current_pd.cap_table).find(cap_id) }.ok_or(sex_pdx::ERR_CAP_INVALID)?;
    let edge = resolve_edge(cap.data, caller_pd_id, false)?;
    traverse_edge(edge, opcode, arg0, arg1, arg2, caller_pd_id)
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

pub fn pdx_spawn_pd(elf_addr: u64, elf_len: u64) -> Result<u32, u64> {
    crate::pd::create::spawn_from_elf(elf_addr, elf_len)
        .map_err(|_| sex_pdx::ERR_CAP_INVALID)
}

pub fn route_signal(target_pd_id: u32, signal: u8) -> Result<(), &'static str> {
    let target_pd = DOMAIN_REGISTRY.get(target_pd_id)
        .ok_or("IPC: Target not found")?;

    unsafe { (*target_pd.signal_ring).enqueue(signal) }
}

pub fn dequeue_local() -> Option<crate::ipc::messages::Message> {
    let core_local = crate::core_local::CoreLocal::get();
    core_local.pdx_ring.dequeue()
}
