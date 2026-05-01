use crate::serial_println;
use x86_64::VirtAddr;

pub static mut SEXDISPLAY_PD_ID: u32 = 0;

pub fn init() {
    // Advance boot phase to allow registry insertion
    unsafe { crate::ipc::BOOT_CONTROLLER.advance(crate::ipc::BootPhase::RegistryBuild); }

    // Phase 1.5: Populate PRIMARY_GPU_LEASE exactly once
    {
        use crate::graphics::gpu_lease::{select_primary_gpu, PRIMARY_GPU_LEASE};
        if let Some(lease) = select_primary_gpu() {
            *PRIMARY_GPU_LEASE.lock() = Some(lease);
            serial_println!("init: PRIMARY_GPU_LEASE populated");
        }
    }

    let modules_res = crate::MODULE_REQUEST.response();
    if modules_res.is_none() {
        panic!("FATAL: Limine returned no modules. Check limine.cfg and ISO layout.");
    }
    let modules = modules_res.unwrap();
    serial_println!("init: Found {} Limine modules", modules.modules().len());

    let mut sexdisp_id = 0;
    let mut silkshell_id = 0;
    let mut sexinput_id = 0;
    let mut silkbar_id = 0;
    let mut linen_id = 0;

    // Fixed Spawn Order (Deterministic IDs)
    let module_paths = ["sexdisplay", "sexdrive", "silk-shell", "sexinput", "silkbar", "linen"];
    for (i, target) in module_paths.iter().enumerate() {
        let domain_id = (i + 1) as u8;
        for module in modules.modules() {
            let path = module.path();
            if path.contains(target) {
                match pdx_spawn(path, domain_id) {
                    Ok(id) => {
                        serial_println!("✓ Spawned PD {}: {} (Domain {})", id, path, domain_id);
                        if domain_id == 1 { 
                            sexdisp_id = id; 
                            unsafe { SEXDISPLAY_PD_ID = id; }
                            
                            use crate::graphics::gpu_lease::claim_primary_for_pd1;
                            let lease = claim_primary_for_pd1();
                            
                            use crate::ipc::DOMAIN_REGISTRY;
                            let pd_ptr = DOMAIN_REGISTRY.get(id).expect("PD1 not in registry");
                            let main_task_ptr = (*pd_ptr).main_task.load(core::sync::atomic::Ordering::Acquire);
                            if !main_task_ptr.is_null() {
                                let main_task = unsafe { &mut *main_task_ptr };
                                main_task.ext_init = Some(crate::scheduler::InitArg { display_lease: lease });
                            }
                        } else if domain_id == 3 {
                            silkshell_id = id;
                        } else if domain_id == 4 {
                            sexinput_id = id;
                        } else if domain_id == 5 {
                            silkbar_id = id;
                        } else if domain_id == 6 {
                            linen_id = id;
                        }
                    }
                    Err(e) => {
                        serial_println!("!! Spawn Error {}: {}", path, e);
                    }
                }
                break;
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
            // Stage 2B: silk-shell can send workspace IPC to SilkBar
            pd.grant_capability(sex_pdx::SLOT_SILKBAR, CapabilityData::Domain(silkbar_id));
            serial_println!("✓ Phase 25: Capabilities granted to silk-shell");
        }

        if sexinput_id != 0 {
            if let Some(pd) = DOMAIN_REGISTRY.get(sexinput_id) {
                // Static Binding: SLOT_INPUT -> Kernel INPUT_RING
                pd.grant_capability(sex_pdx::SLOT_INPUT, CapabilityData::InputRing);
                // Grant access to silk-shell for event forwarding
                pd.grant_capability(sex_pdx::SLOT_SHELL, CapabilityData::Domain(silkshell_id));
                serial_println!("✓ Phase 25: Capabilities granted to sexinput");
            }
        }

    }

    // SilkBar delivery path: grant display capability independently of silk-shell.
    // Otherwise SilkBar updates are silently blocked whenever silk-shell is absent.
    if sexdisp_id != 0 && silkbar_id != 0 {
        use crate::ipc::DOMAIN_REGISTRY;
        use crate::capability::CapabilityData;
        if let Some(pd) = DOMAIN_REGISTRY.get(silkbar_id) {
            pd.grant_capability(sex_pdx::SLOT_DISPLAY, CapabilityData::Domain(sexdisp_id));
            serial_println!("✓ SilkBar v8: Capability SLOT_DISPLAY granted");
        }
    }

    // Linen delivery path: grant display capability for placeholder surface.
    if linen_id != 0 && sexdisp_id != 0 {
        use crate::ipc::DOMAIN_REGISTRY;
        use crate::capability::CapabilityData;
        if let Some(pd) = DOMAIN_REGISTRY.get(linen_id) {
            pd.grant_capability(sex_pdx::SLOT_DISPLAY, CapabilityData::Domain(sexdisp_id));
            serial_println!("✓ Phase 25: Capability SLOT_DISPLAY granted to linen");
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
                    // Use manual page-table walk that handles huge pages (2MiB, 1GiB).
                    // The old mapper.update_flags(Page<Size4KiB>) silently returns
                    // Err(ParentEntryHugePage) when the framebuffer is mapped with huge pages,
                    // leaving USER_ACCESSIBLE unset and causing #GP from ring 3.
                    let pkey = sexdisp_id as u8; // domain_id == pkey for sexdisplay
                    let start = fb_addr & !0xFFF;
                    let end = ((fb_addr + fb_size + 4095) & !0xFFF);
                    for va in (start..end).step_by(4096) {
                        unsafe { crate::pku::set_page_user_accessible(va, pkey); }
                    }
                    serial_println!("init: FB remapped USER_ACCESSIBLE ({:#x}, {} bytes) key={}",
                        fb_addr, fb_size, pkey);
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

    // Enqueue all registered PD tasks onto scheduler runqueue
    for pd_id in 1..crate::ipc::MAX_DOMAINS as u32 {
        if let Some(pd) = crate::ipc::DOMAIN_REGISTRY.get(pd_id) {
            let task_ptr = pd.main_task.load(core::sync::atomic::Ordering::Acquire);
            if !task_ptr.is_null() {
                unsafe { (*task_ptr).state.store(crate::scheduler::STATE_READY, core::sync::atomic::Ordering::Release); }
                crate::scheduler::SCHEDULERS[0].runqueue.push(task_ptr);
                serial_println!("scheduler.enqueue pd_id={}", pd_id);
            }
        }
    }

    unsafe {
        crate::ipc::BOOT_CONTROLLER.advance(crate::ipc::BootPhase::RegistryFrozen);
        crate::ipc::BOOT_CONTROLLER.advance(crate::ipc::BootPhase::SchedulerArmed);
        crate::ipc::BOOT_CONTROLLER.advance(crate::ipc::BootPhase::SchedulerRunning);
    }

    serial_println!("init: Ready for Scheduler.");
}

fn pdx_spawn(name: &str, domain_id: u8) -> Result<u32, &'static str> {
    use crate::pd::create::create_protection_domain;
    create_protection_domain(name, None, domain_id)
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

    // crate::core_local::CoreLocal::get().set_pd(pd_id); // Deprecated, jump_to_userland is dead code

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
