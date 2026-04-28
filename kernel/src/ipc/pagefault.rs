use crate::ipc::messages::MessageType;
use crate::ipc::safe_pdx_call;

/// Route a #PF to the sext demand pager (user-space fault resolver, slot 2).
///
/// The kernel is a transition arbiter only — NOT a memory authority.
/// It traps the fault, forwards via sex-pdx, and resumes. It does NOT allocate
/// frames, does NOT map memory, does NOT own GLOBAL_VAS.
///
/// Canonical fault flow:
///   CPU #PF exception
///     → kernel trap (this function — ONLY ring-0 component)
///     → sex-pdx dispatch: safe_pdx_call(slot=2, ...)
///     → sext domain (ring-3): frame allocator
///     → GLOBAL_VAS::map_pku_range()
///     → PKU key assignment (enforcement layer — see ARCHITECTURE.md §0)
///     → resume faulting domain
///
/// Frame allocation and GLOBAL_VAS mapping are sext's exclusive responsibility.
pub fn forward_page_fault(fault_addr: u64, error_code: u32, pd_id: u64) -> Result<(), u64> {
    let msg = MessageType::PageFault {
        fault_addr,
        error_code,
        pd_id,
        lent_cap: 0,
    };
    safe_pdx_call(2, 0, &msg as *const _ as u64, 0, 0)?;
    Ok(())
}
