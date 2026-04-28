use crate::serial_println;
use x86_64::VirtAddr;
use core::sync::atomic::{AtomicU8, Ordering};
use crate::capability::ProtectionDomain;

pub static mut SEXDISPLAY_PD_ID: u32 = 0;
static NEXT_PKEY: AtomicU8 = AtomicU8::new(4);

const SEXDISPLAY_PD_ID_CONST: u32 = 1;
const LINEN_PD_ID_CONST:      u32 = 2;
const SILK_SHELL_PD_ID_CONST: u32 = 3;

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
    if pd_id == SEXDISPLAY_PD_ID_CONST {
        (*pd.cap_table).insert_at(SLOT_DISPLAY as u32, CapabilityData::Domain(SEXDISPLAY_PD_ID_CONST));
    } else if pd_id == LINEN_PD_ID_CONST {
        (*pd.cap_table).insert_at(SLOT_STORAGE as u32, CapabilityData::Domain(LINEN_PD_ID_CONST));
        (*pd.cap_table).insert_at(SLOT_DISPLAY as u32, CapabilityData::Domain(SEXDISPLAY_PD_ID_CONST));
    } else if pd_id == SILK_SHELL_PD_ID_CONST {
        (*pd.cap_table).insert_at(SLOT_DISPLAY as u32, CapabilityData::Domain(SEXDISPLAY_PD_ID_CONST));
        (*pd.cap_table).insert_at(SLOT_SHELL as u32,   CapabilityData::Domain(SILK_SHELL_PD_ID_CONST));
    } else {
        (*pd.cap_table).insert_at(SLOT_STORAGE as u32, CapabilityData::Domain(LINEN_PD_ID_CONST));
        (*pd.cap_table).insert_at(SLOT_DISPLAY as u32, CapabilityData::Domain(SEXDISPLAY_PD_ID_CONST));
        (*pd.cap_table).insert_at(SLOT_SHELL as u32,   CapabilityData::Domain(SILK_SHELL_PD_ID_CONST));
    }
}

pub fn init() {
    // Phase 1.5: Populate PRIMARY_GPU_LEASE exactly once
    {
        use crate::graphics::gpu_lease::{select_primary_gpu, PRIMARY_GPU_LEASE};
        if let Some(lease) = select_primary_gpu() {
            *PRIMARY_GPU_LEASE.lock() = Some(lease);
            serial_println!("init: PRIMARY_GPU_LEASE populated");
        }
    }

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
                        
                        // Phase 1: GPU Lease assignment
                        use crate::graphics::gpu_lease::claim_primary_for_pd1;
                        let lease = claim_primary_for_pd1();
                        
                        use crate::ipc::DOMAIN_REGISTRY;
                        let pd_ptr = DOMAIN_REGISTRY.get(id).expect("PD1 not in registry");
                        let main_task_ptr = (*pd_ptr).main_task.load(core::sync::atomic::Ordering::Acquire);
                        if !main_task_ptr.is_null() {
                            let main_task = unsafe { &mut *main_task_ptr };
                            main_task.ext_init = Some(crate::scheduler::InitArg { display_lease: lease });
                        }
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
                let fb_addr = fb.address() as u64;
                let fb_size = fb.pitch * fb.height;

                // Remap FB pages USER_ACCESSIBLE — Ring-3 sexdisplay can't write without this.
                {
                    use x86_64::structures::paging::{
                        Mapper, Page, PageTableFlags, OffsetPageTable, PageTable, Size4KiB,
                    };
                    use x86_64::registers::control::Cr3;
                    const HHDM: u64 = 0xffff800000000000;
                    let (cr3_frame, _) = Cr3::read();
                    let pml4 = unsafe {
                        &mut *((cr3_frame.start_address().as_u64() + HHDM) as *mut PageTable)
                    };
                    let mut mapper = unsafe { OffsetPageTable::new(pml4, VirtAddr::new(HHDM)) };
                    let flags = PageTableFlags::PRESENT
                        | PageTableFlags::WRITABLE
                        | PageTableFlags::USER_ACCESSIBLE;
                    let start = Page::<Size4KiB>::containing_address(VirtAddr::new(fb_addr));
                    let end = Page::<Size4KiB>::containing_address(
                        VirtAddr::new(fb_addr + fb_size - 1)
                    );
                    for page in Page::range_inclusive(start, end) {
                        unsafe {
                            if let Ok(tlb) = mapper.update_flags(page, flags) {
                                tlb.flush();
                            }
                        }
                    }
                    serial_println!("init: FB remapped USER_ACCESSIBLE ({:#x}, {} pages)",
                        fb_addr, (fb_size as usize + 4095) / 4096);
                }

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
    serial_println!("[DEBUG] pdx_spawn: pushing task {} to SCHEDULERS[0] at {:p}", pd_id, &crate::scheduler::SCHEDULERS[0].runqueue);
    serial_println!("[DEBUG] pdx_spawn: queue len before push = {}", crate::scheduler::SCHEDULERS[0].runqueue.bottom.load(core::sync::atomic::Ordering::SeqCst) - crate::scheduler::SCHEDULERS[0].runqueue.top.load(core::sync::atomic::Ordering::SeqCst));
    crate::scheduler::SCHEDULERS[0].runqueue.push(task_ptr);
    serial_println!("[DEBUG] pdx_spawn: queue len after push = {}", crate::scheduler::SCHEDULERS[0].runqueue.bottom.load(core::sync::atomic::Ordering::SeqCst) - crate::scheduler::SCHEDULERS[0].runqueue.top.load(core::sync::atomic::Ordering::SeqCst));
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

    unsafe { grant_standard_capabilities(&mut *pd_ptr, pd_id); }

    // Phase 25 IPCPKU_MAP.md — explicit dynamic capability grants for every spawned PD.
    // Overwrites static constants from grant_standard_capabilities with live IDs.
    unsafe {
        use crate::capability::CapabilityData;
        let disp_id = SEXDISPLAY_PD_ID; // 0 when sexdisplay itself is being spawned (harmless)
        if disp_id != 0 {
            (*pd_ptr).grant_capability(sex_pdx::SLOT_DISPLAY, CapabilityData::Domain(disp_id));
        }
        (*pd_ptr).grant_capability(sex_pdx::SLOT_SHELL, CapabilityData::Domain(SILK_SHELL_PD_ID_CONST));
    }

    if name.contains("linen") {
        serial_println!("[kernel] Linen PD{} — granted SLOT_DISPLAY={} SLOT_SHELL={}",
                        pd_id, sex_pdx::SLOT_DISPLAY, sex_pdx::SLOT_SHELL);
    }
    serial_println!("[kernel] PD {} ({}) — capability graph wired under MPK lock", pd_id, name);

    DOMAIN_REGISTRY.insert(pd_id, pd_ptr);
    
    serial_println!("PDX: Registered {} (PKEY {})", name, pku_key);

    Ok(pd_id)
}

pub unsafe fn jump_to_userland(pd_id: u32, entry: u64, pkru: u32, pku_key: u8) -> ! {
    use crate::gdt;
    let selectors = gdt::get_selectors();
    
    // User Code Segment (0x28) with RPL 3 = 0x2B
    let user_cs = (selectors.user_cs.0 | 3) as u64;
    // User Data Segment (0x20) with RPL 3 = 0x23
    let user_ss = (selectors.user_ss.0 | 3) as u64;
    
    let rflags: u64 = 0x3202;
    let stack_top = 0x_7000_0000_0000 + (pku_key as u64 * 0x100_0000) + (64 * 1024);

    crate::core_local::CoreLocal::get().set_pd(pd_id);

    core::arch::asm!(
        "xor eax, eax", "xor ecx, ecx", "xor edx, edx", "wrpkru", // God Mode
        "push {ss}",
        "push {rsp_val}",
        "push {rflags}",
        "push {cs}",
        "push {rip}",
        "mov eax, {target_pkru:e}", "xor ecx, ecx", "xor edx, edx", "wrpkru", // Isolation
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
