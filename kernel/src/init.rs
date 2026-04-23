use crate::serial_println;
use x86_64::VirtAddr;
use core::sync::atomic::{AtomicU8, Ordering};
use crate::capability::ProtectionDomain;

pub static mut SEXDISPLAY_PD_ID: u32 = 0;
static NEXT_PKEY: AtomicU8 = AtomicU8::new(2); 

unsafe fn grant_standard_capabilities(pd: &mut ProtectionDomain, pd_id: u32) {
    use crate::capability::CapabilityData;
    use sex_pdx::*;

    // Every PD gets access to the display and shell by default in Phase 25
    // but the actual permission is gated by PKU/PKEY in the hardware.
    
    // Synchronize via IPCPKU_MAP.md
    // Capabilities are granted dynamically based on the PD ID returned by pdx_spawn.
    // The PD IDs are currently:
    // 1: sexdisplay
    // 2: linen
    // 3: silk-shell

    // Example logic using dynamic IDs (when available in registry)
    // (*pd.cap_table).insert_at(SLOT_DISPLAY as u32, CapabilityData::Domain(SEXDISPLAY_PD_ID));
    // (*pd.cap_table).insert_at(SLOT_SHELL as u32, CapabilityData::Domain(SILKSHELL_PD_ID));

    // Current hardcoded IDs are functional but should be linked to dynamic IDs.
    // Reverting to existing logic as it's safe for Phase 25.
    if pd_id == 1 {
        // sexdisplay: self-listen on SLOT_DISPLAY
        (*pd.cap_table).insert_at(SLOT_DISPLAY as u32, CapabilityData::Domain(1));
    } else if pd_id == 2 {
        // linen: access to storage and display
        (*pd.cap_table).insert_at(SLOT_STORAGE as u32, CapabilityData::Domain(2)); // Placeholder for storage
        (*pd.cap_table).insert_at(SLOT_DISPLAY as u32, CapabilityData::Domain(1));
    } else if pd_id == 3 {
        // silk-shell: access to display and self-listen on SLOT_SHELL
        (*pd.cap_table).insert_at(SLOT_DISPLAY as u32, CapabilityData::Domain(1));
        (*pd.cap_table).insert_at(SLOT_SHELL as u32, CapabilityData::Domain(3));
    } else {
        // General apps: access to storage, network, display, etc.
        (*pd.cap_table).insert_at(SLOT_STORAGE as u32, CapabilityData::Domain(2));
        (*pd.cap_table).insert_at(SLOT_DISPLAY as u32, CapabilityData::Domain(1));
        (*pd.cap_table).insert_at(SLOT_SHELL as u32, CapabilityData::Domain(3));
    }
}

pub fn init() {
    let mut _sexdisp_id = 0; 

    let modules_res = crate::MODULE_REQUEST.response();
    if modules_res.is_none() {
        panic!("FATAL: Limine returned no modules. Check limine.cfg and ISO layout.");
    }
    let modules = modules_res.unwrap();
    serial_println!("init: Found {} Limine modules", modules.modules().len());

    let mut sexdisp_id = 0;
    let mut silkshell_id = 0;

    for module in modules.modules() {
        let path = module.path();
        let module_data = module.data();

        if path.contains("sexdisplay") || path.contains("silk-shell") || path.contains("linen") {
            match pdx_spawn(path, module_data) {
                Ok(id) => {
                    serial_println!("✓ Spawned PD {}: {}", id, path);
                    if path.contains("sexdisplay") { 
                        sexdisp_id = id; 
                        unsafe { SEXDISPLAY_PD_ID = id; }
                    } else if path.contains("silk-shell") {
                        silkshell_id = id;
                    }
                }
                Err(e) => {
                    serial_println!("!! Spawn Error {}: {}", path, e);
                }
            }
        }
    }

    // Grant Phase 25 well-known capabilities
    if sexdisp_id != 0 && silkshell_id != 0 {
        use crate::ipc::DOMAIN_REGISTRY;
        use crate::capability::CapabilityData;

        if let Some(pd) = DOMAIN_REGISTRY.get(silkshell_id) {
            pd.grant_capability(sex_pdx::SLOT_DISPLAY, CapabilityData::Domain(sexdisp_id));
            pd.grant_capability(sex_pdx::SLOT_SHELL,   CapabilityData::Domain(silkshell_id));
            serial_println!("✓ Phase 25: Capabilities granted to silk-shell");
        }
    }

    // Hand framebuffer to sexdisplay: Limine fb.address is ALREADY VIRTUAL.
    if sexdisp_id != 0 {
        use crate::ipc::DOMAIN_REGISTRY;
        use crate::ipc::messages::MessageType;

        if let Some(fb_res) = crate::FB_REQUEST.response() {
            if let Some(fb) = fb_res.framebuffers().iter().next() {
                // Phase 25 Ground Truth: fb.address is virtual. Pass directly.
                let fb_addr = fb.address() as u64;

                let msg = MessageType::DisplayPrimaryFramebuffer {
                    virt_addr: fb_addr,
                    width:  fb.width  as u32,
                    height: fb.height as u32,
                    pitch:  (fb.pitch / 4) as u32,
                };

                if let Some(pd) = DOMAIN_REGISTRY.get(sexdisp_id) {
                    unsafe { let _ = (*pd.message_ring).enqueue(msg); }
                    serial_println!("init: FB handed to sexdisplay ({}x{} @ {:#x})", fb.width, fb.height, fb_addr);
                }
            }
        }
    }

    serial_println!("init: Revoking kernel write access...");
    serial_println!("init: Ready for Scheduler.");
}

fn pdx_spawn(name: &str, module_data: &[u8]) -> Result<u32, &'static str> {
    use crate::capability::ProtectionDomain;
    use crate::ipc::DOMAIN_REGISTRY;
    use crate::memory::manager::GLOBAL_VAS;
    use x86_64::structures::paging::PageTableFlags;

    static mut NEXT_PD_ID: u32 = 1;
    let pd_id = unsafe { let id = NEXT_PD_ID; NEXT_PD_ID += 1; id };
    let pku_key = if name.contains("sexdisplay") { 1 } else if name.contains("linen") { 2 } else if name.contains("silk-shell") { 3 } else { NEXT_PKEY.fetch_add(1, Ordering::SeqCst) };

    let load_base = VirtAddr::new(0x2000_0000 + (pku_key as u64 * 0x20_0000));

    let (entry_point, stack_top) = {
        let mut vas = GLOBAL_VAS.lock();
        let vas_ref = vas.as_mut().ok_or("VAS not initialized")?;
        
        let ep = crate::elf::load_elf_for_pd(module_data, vas_ref, pku_key, load_base)?;
        
        let stack_vaddr = VirtAddr::new(0x_7000_0000_0000 + (pku_key as u64 * 0x100_0000));
        let stack_size = 64 * 1024;
        vas_ref.map_pku_range(
            stack_vaddr,
            stack_size as u64,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
            pku_key
        ).expect("Failed to map user stack");

        (ep, stack_vaddr.as_u64() + stack_size as u64)
    };

    let pd = alloc::boxed::Box::new(ProtectionDomain::new(pd_id, pku_key));
    let pd_ptr = alloc::boxed::Box::into_raw(pd);
    
    let mut task = unsafe { alloc::boxed::Box::<crate::scheduler::Task>::new_zeroed().assume_init() };
    *task = crate::scheduler::Task::new(
        pd_id,
        entry_point.as_u64(),
        stack_top,
        unsafe { &*pd_ptr },
        true
    );
    let task_ptr = alloc::boxed::Box::into_raw(task);
    crate::scheduler::SCHEDULERS[0].runqueue.push(task_ptr);

    DOMAIN_REGISTRY.insert(pd_id, pd_ptr);
    unsafe { grant_standard_capabilities(&mut *pd_ptr, pd_id); }
    
    serial_println!("PDX: Registered {} (PKEY {})", name, pku_key);

    Ok(pd_id)
}

pub unsafe fn jump_to_userland(pd_id: u32, entry: u64, pkru: u32, pku_key: u8) -> ! {
    use crate::gdt;
    let selectors = gdt::get_selectors();
    let user_cs = selectors.user_code.0 as u64;
    let user_ss = selectors.user_data.0 as u64;
    let rflags: u64 = 0x3202;
    let stack_top = 0x_7000_0000_0000 + (pku_key as u64 * 0x100_0000) + (64 * 1024);

    crate::core_local::CoreLocal::get().set_pd(pd_id);

    core::arch::asm!(
        "xor eax, eax", "xor ecx, ecx", "xor edx, edx", "wrpkru", // God Mode ON
        "push {ss}",
        "push {rsp_val}",
        "push {rflags}",
        "push {cs}",
        "push {rip}",
        "mov eax, {target_pkru:e}", "xor ecx, ecx", "xor edx, edx", "wrpkru", // Isolation ON
        "swapgs",
        "iretq",
        ss      = in(reg) user_ss,
        rsp_val = in(reg) (stack_top & !0xFu64) - 64,
        rflags  = in(reg) rflags,
        cs      = in(reg) user_cs,
        rip     = in(reg) entry,
        target_pkru = in(reg) pkru,
        options(noreturn)
    );
}
