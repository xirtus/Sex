use crate::serial_println;
use x86_64::VirtAddr;

pub static mut SEXDISPLAY_PD_ID: u32 = 0;
const SLOT_USB_SEXINPUT: u64 = 9;

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
    let mut sexdrive_id = 0;
    let mut silkshell_id = 0;
    let mut sexinput_id = 0;
    let mut sexusb_id = 0;
    let mut silkbar_id = 0;
    let mut linen_id = 0;
    let mut sexstore_id = 0;
    let mut quil_id = 0;
    let mut sexbell_id = 0;
    let mut sexfiles_id = 0;
    let mut spindle_id = 0;

    // Fixed Spawn Order (Deterministic IDs)
    let module_paths = ["sexdisplay", "sexdrive", "silk-shell", "sexinput", "sexusb", "silkbar", "linen", "sexstore", "quil", "sexbell", "sexfiles", "spindle"];
    for (i, target) in module_paths.iter().enumerate() {
        let domain_id = (i + 1) as u8;
        for module in modules.modules() {
            let path = module.path();
            if path.contains(target) {
                serial_println!("[bootgraph.pd.spawn.begin] pd={}", target);
                match pdx_spawn(path, domain_id) {
                    Ok(id) => {
                        serial_println!("✓ Spawned PD {}: {} (Domain {})", id, path, domain_id);
                        serial_println!(
                            "[bootgraph.pd.spawn.ok] pd={} id={} pkey={}",
                            target, id, domain_id
                        );
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
                        } else if domain_id == 2 {
                            sexdrive_id = id;
                        } else if domain_id == 3 {
                            silkshell_id = id;
                        } else if domain_id == 4 {
                            sexinput_id = id;
                        } else if domain_id == 5 {
                            sexusb_id = id;
                        } else if domain_id == 6 {
                            silkbar_id = id;
                        } else if domain_id == 7 {
                            linen_id = id;
                        } else if domain_id == 8 {
                            sexstore_id = id;
                            serial_println!("[kernel.sexstore.spawn] id={}", id);
                        } else if domain_id == 9 {
                            quil_id = id;
                            serial_println!("[kernel.spawn.quil] id={} path={}", id, path);
                        } else if domain_id == 10 {
                            sexbell_id = id;
                            serial_println!("[kernel.spawn.sexbell] id={} path={}", id, path);
                        } else if domain_id == 11 {
                            sexfiles_id = id;
                            serial_println!("[kernel.spawn.sexfiles] id={} path={}", id, path);
                        } else if domain_id == 12 {
                            spindle_id = id;
                            serial_println!("[kernel.spawn.spindle] id={} path={}", id, path);
                        }
                    }
                    Err(e) => {
                        serial_println!("!! Spawn Error {}: {}", path, e);
                        serial_println!("[bootgraph.pd.spawn.err] pd={} reason=spawn_failed", target);
                    }
                }
                break;
            }
        }
    }

    // Grant Phase 25 well-known capabilities
    serial_println!("[bootgraph.phase25.begin]");
    if sexdisp_id != 0 && silkshell_id != 0 {
        use crate::ipc::DOMAIN_REGISTRY;
        use crate::capability::CapabilityData;

        if let Some(pd) = DOMAIN_REGISTRY.get(silkshell_id) {
            pd.grant_capability(sex_pdx::SLOT_DISPLAY, CapabilityData::Domain(sexdisp_id));
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_DISPLAY target={} ok=1]",
                silkshell_id, sexdisp_id
            );
            serial_println!(
                "[bootgraph.cap.grant] from=kernel to={} slot=SLOT_DISPLAY target={} ok=1",
                silkshell_id, sexdisp_id
            );
            pd.grant_capability(sex_pdx::SLOT_SHELL,   CapabilityData::Domain(silkshell_id));
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_SHELL target={} ok=1]",
                silkshell_id, silkshell_id
            );
            serial_println!(
                "[bootgraph.cap.grant] from=kernel to={} slot=SLOT_SHELL target={} ok=1",
                silkshell_id, silkshell_id
            );
            // Stage 2B: silk-shell can send workspace IPC to SilkBar
            pd.grant_capability(sex_pdx::SLOT_SILKBAR, CapabilityData::Domain(silkbar_id));
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_SILKBAR target={} ok=1 optional=1]",
                silkshell_id, silkbar_id
            );
            serial_println!(
                "[bootgraph.cap.grant] from=kernel to={} slot=SLOT_SILKBAR target={} ok=1 optional=1",
                silkshell_id, silkbar_id
            );
            if sexstore_id != 0 {
                pd.grant_capability(sex_pdx::SLOT_SEXSTORE, CapabilityData::Domain(sexstore_id));
                serial_println!("[kernel.sexstore.cap] shell={} store={}", silkshell_id, sexstore_id);
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_SEXSTORE target={} ok=1 optional=1]",
                    silkshell_id, sexstore_id
                );
                serial_println!(
                    "[bootgraph.cap.grant] from=kernel to={} slot=SLOT_SEXSTORE target={} ok=1 optional=1",
                    silkshell_id, sexstore_id
                );
            } else {
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_SEXSTORE target=missing ok=0 optional=1 reason=sexstore_absent]",
                    silkshell_id
                );
                serial_println!(
                    "[bootgraph.cap.grant] from=kernel to={} slot=SLOT_SEXSTORE target=missing ok=0 optional=1 reason=missing_target",
                    silkshell_id
                );
            }
            // Bell read-cap: silk-shell can call OP_BELL_LIST
            if sexbell_id != 0 {
                pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(sexbell_id));
                serial_println!("[kernel.sexbell.cap.shell] shell→bell slot=12");
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_BELL target={} ok=1 optional=1]",
                    silkshell_id, sexbell_id
                );
                serial_println!(
                    "[bootgraph.cap.grant] from=kernel to={} slot=SLOT_BELL target={} ok=1 optional=1",
                    silkshell_id, sexbell_id
                );
            } else {
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_BELL target=missing ok=0 optional=1 reason=sexbell_absent]",
                    silkshell_id
                );
                serial_println!(
                    "[bootgraph.cap.grant] from=kernel to={} slot=SLOT_BELL target=missing ok=0 optional=1 reason=missing_target",
                    silkshell_id
                );
            }
            serial_println!("✓ Phase 25: Capabilities granted to silk-shell");
        } else {
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_DISPLAY target={} ok=0 optional=1 reason=pd_missing]",
                silkshell_id, sexdisp_id
            );
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_SHELL target={} ok=0 optional=1 reason=pd_missing]",
                silkshell_id, silkshell_id
            );
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_SILKBAR target={} ok=0 optional=1 reason=pd_missing]",
                silkshell_id, silkbar_id
            );
            if sexstore_id != 0 {
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_SEXSTORE target={} ok=0 optional=1 reason=pd_missing]",
                    silkshell_id, sexstore_id
                );
            }
            if sexbell_id != 0 {
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_BELL target={} ok=0 optional=1 reason=pd_missing]",
                    silkshell_id, sexbell_id
                );
            }
        }

        if sexinput_id != 0 {
            if let Some(pd) = DOMAIN_REGISTRY.get(sexinput_id) {
                // Static Binding: SLOT_INPUT -> Kernel INPUT_RING
                pd.grant_capability(sex_pdx::SLOT_INPUT, CapabilityData::InputRing);
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_INPUT target=input_ring ok=1]",
                    sexinput_id
                );
                serial_println!(
                    "[bootgraph.cap.grant] from=kernel to={} slot=SLOT_INPUT target=input_ring ok=1",
                    sexinput_id
                );
                // Grant access to silk-shell for event forwarding
                pd.grant_capability(sex_pdx::SLOT_SHELL, CapabilityData::Domain(silkshell_id));
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_SHELL target={} ok=1]",
                    sexinput_id, silkshell_id
                );
                serial_println!(
                    "[bootgraph.cap.grant] from=kernel to={} slot=SLOT_SHELL target={} ok=1",
                    sexinput_id, silkshell_id
                );
                serial_println!("✓ Phase 25: Capabilities granted to sexinput");
            } else {
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_INPUT target=input_ring ok=0 optional=1 reason=pd_missing]",
                    sexinput_id
                );
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_SHELL target={} ok=0 optional=1 reason=pd_missing]",
                    sexinput_id, silkshell_id
                );
            }
        } else {
            serial_println!(
                "[bootgraph.cap.grant from=kernel to=missing slot=SLOT_INPUT target=input_ring ok=0 optional=1 reason=sexinput_absent]"
            );
            serial_println!(
                "[bootgraph.cap.grant from=kernel to=missing slot=SLOT_SHELL target={} ok=0 optional=1 reason=sexinput_absent]",
                silkshell_id
            );
        }

        // Minimal USB input route: allow sexusb to send decoded mouse reports
        // directly to sexinput over one dedicated domain slot.
        if sexusb_id != 0 && sexinput_id != 0 {
            if let Some(pd) = DOMAIN_REGISTRY.get(sexusb_id) {
                pd.grant_capability(SLOT_USB_SEXINPUT, CapabilityData::Domain(sexinput_id));
                serial_println!("✓ cap.route: sexusb->sexinput slot {}", SLOT_USB_SEXINPUT);
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot={} target={} ok=1 optional=1]",
                    sexusb_id, SLOT_USB_SEXINPUT, sexinput_id
                );
                serial_println!(
                    "[bootgraph.cap.grant] from=kernel to={} slot={} target={} ok=1 optional=1",
                    sexusb_id, SLOT_USB_SEXINPUT, sexinput_id
                );
            } else {
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot={} target={} ok=0 optional=1 reason=pd_missing]",
                    sexusb_id, SLOT_USB_SEXINPUT, sexinput_id
                );
            }
        } else {
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot={} target={} ok=0 optional=1 reason=sexusb_or_sexinput_absent]",
                sexusb_id, SLOT_USB_SEXINPUT, sexinput_id
            );
        }

    } else {
        serial_println!(
            "[bootgraph.cap.grant from=kernel to={} slot=SLOT_DISPLAY target={} ok=0 optional=1 reason=sexdisplay_or_silkshell_absent]",
            silkshell_id, sexdisp_id
        );
    }

    // Hardware discovery and driver lease routing.
    // Includes XHCI discovery + lease to sexusb (slot SLOT_USB_HOST) when present.
    if sexdrive_id != 0 && sexdisp_id != 0 {
        crate::devmgr::init(sexdrive_id, sexdisp_id, sexusb_id);
    }

    // SilkBar delivery path: grant display capability independently of silk-shell.
    // Otherwise SilkBar updates are silently blocked whenever silk-shell is absent.
    if sexdisp_id != 0 && silkbar_id != 0 {
        use crate::ipc::DOMAIN_REGISTRY;
        use crate::capability::CapabilityData;
        if let Some(pd) = DOMAIN_REGISTRY.get(silkbar_id) {
            pd.grant_capability(sex_pdx::SLOT_DISPLAY, CapabilityData::Domain(sexdisp_id));
            serial_println!("✓ SilkBar v8: Capability SLOT_DISPLAY granted");
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_DISPLAY target={} ok=1]",
                silkbar_id, sexdisp_id
            );
            // Bell polling cap: SilkBar needs SLOT_BELL for OP_BELL_LIST.
            // This is a read-only LIST capability — SilkBar has no NOTIFY/CLOSE/ACTION.
            // Bell server-side allowlist (BELL_LIST_ALLOWLIST) provides second gate.
            if sexbell_id != 0 {
                pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(sexbell_id));
                serial_println!("[kernel.sexbell.cap.silkbar] silkbar→bell slot=12");
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_BELL target={} ok=1 optional=1]",
                    silkbar_id, sexbell_id
                );
            } else {
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_BELL target=missing ok=0 optional=1 reason=sexbell_absent]",
                    silkbar_id
                );
            }
        } else {
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_DISPLAY target={} ok=0 optional=1 reason=pd_missing]",
                silkbar_id, sexdisp_id
            );
        }
    } else {
        serial_println!(
            "[bootgraph.cap.grant from=kernel to={} slot=SLOT_DISPLAY target={} ok=0 optional=1 reason=sexdisplay_or_silkbar_absent]",
            silkbar_id, sexdisp_id
        );
    }

    // Linen delivery path: grant display capability for placeholder surface.
    if linen_id != 0 && sexdisp_id != 0 {
        use crate::ipc::DOMAIN_REGISTRY;
        use crate::capability::CapabilityData;
        if let Some(pd) = DOMAIN_REGISTRY.get(linen_id) {
            pd.grant_capability(sex_pdx::SLOT_DISPLAY, CapabilityData::Domain(sexdisp_id));
            serial_println!("✓ Phase 25: Capability SLOT_DISPLAY granted to linen");
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_DISPLAY target={} ok=1]",
                linen_id, sexdisp_id
            );
            if sexfiles_id != 0 {
                pd.grant_capability(sex_pdx::SLOT_STORAGE, CapabilityData::Domain(sexfiles_id));
                serial_println!(
                    "[kernel.cap.storage.linen] slot={} target_pd={}",
                    sex_pdx::SLOT_STORAGE,
                    sexfiles_id
                );
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_STORAGE target={} ok=1 optional=1]",
                    linen_id, sexfiles_id
                );
            } else {
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_STORAGE target=missing ok=0 optional=1 reason=sexfiles_absent]",
                    linen_id
                );
            }
        } else {
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_DISPLAY target={} ok=0 optional=1 reason=pd_missing]",
                linen_id, sexdisp_id
            );
        }
        
        if silkshell_id != 0 {
            if let Some(pd) = DOMAIN_REGISTRY.get(silkshell_id) {
                pd.grant_capability(sex_pdx::SLOT_LINEN, CapabilityData::Domain(linen_id));
                serial_println!("[kernel.cap.linen.route] shell->linen slot={}", sex_pdx::SLOT_LINEN);
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_LINEN target={} ok=1 optional=1]",
                    silkshell_id, linen_id
                );
            } else {
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_LINEN target={} ok=0 optional=1 reason=pd_missing]",
                    silkshell_id, linen_id
                );
            }
        } else {
            serial_println!(
                "[bootgraph.cap.grant from=kernel to=missing slot=SLOT_LINEN target={} ok=0 optional=1 reason=silkshell_absent]",
                linen_id
            );
        }
    } else {
        serial_println!(
            "[bootgraph.cap.grant from=kernel to={} slot=SLOT_DISPLAY target={} ok=0 optional=1 reason=linen_or_sexdisplay_absent]",
            linen_id, sexdisp_id
        );
    }

    // Quil route: grant silk-shell capability to ping Quil (no display caps).
    if quil_id != 0 && silkshell_id != 0 {
        use crate::ipc::DOMAIN_REGISTRY;
        use crate::capability::CapabilityData;
        if let Some(pd) = DOMAIN_REGISTRY.get(silkshell_id) {
            pd.grant_capability(sex_pdx::SLOT_QUIL, CapabilityData::Domain(quil_id));
            serial_println!("[kernel.cap.quil.route] shell->quil slot={}", sex_pdx::SLOT_QUIL);
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_QUIL target={} ok=1 optional=1]",
                silkshell_id, quil_id
            );
        } else {
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_QUIL target={} ok=0 optional=1 reason=pd_missing]",
                silkshell_id, quil_id
            );
        }
        
        if sexdisp_id != 0 {
            if let Some(pd) = DOMAIN_REGISTRY.get(quil_id) {
                pd.grant_capability(sex_pdx::SLOT_DISPLAY, CapabilityData::Domain(sexdisp_id));
                serial_println!("✓ Capability SLOT_DISPLAY granted to quil");
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_DISPLAY target={} ok=1]",
                    quil_id, sexdisp_id
                );
                if sexfiles_id != 0 {
                    pd.grant_capability(sex_pdx::SLOT_STORAGE, CapabilityData::Domain(sexfiles_id));
                    serial_println!("[kernel.cap.storage.quil] quil->sexfiles slot={}", sex_pdx::SLOT_STORAGE);
                    serial_println!(
                        "[bootgraph.cap.grant from=kernel to={} slot=SLOT_STORAGE target={} ok=1 optional=1]",
                        quil_id, sexfiles_id
                    );
                } else {
                    serial_println!(
                        "[bootgraph.cap.grant from=kernel to={} slot=SLOT_STORAGE target=missing ok=0 optional=1 reason=sexfiles_absent]",
                        quil_id
                    );
                }
            } else {
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_DISPLAY target={} ok=0 optional=1 reason=pd_missing]",
                    quil_id, sexdisp_id
                );
            }
        } else {
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_DISPLAY target={} ok=0 optional=1 reason=sexdisplay_absent]",
                quil_id, sexdisp_id
            );
        }
    } else {
        serial_println!(
            "[bootgraph.cap.grant from=kernel to={} slot=SLOT_QUIL target={} ok=0 optional=1 reason=quil_or_silkshell_absent]",
            silkshell_id, quil_id
        );
    }

    // Bell self-cap: grant SLOT_BELL to sexbell for listen (no external caps).
    if sexbell_id != 0 {
        use crate::ipc::DOMAIN_REGISTRY;
        use crate::capability::CapabilityData;
        if let Some(pd) = DOMAIN_REGISTRY.get(sexbell_id) {
            pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(sexbell_id));
            serial_println!("[kernel.sexbell.cap] self slot={}", sex_pdx::SLOT_BELL);
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_BELL target={} ok=1 optional=1]",
                sexbell_id, sexbell_id
            );
        } else {
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_BELL target={} ok=0 optional=1 reason=pd_missing]",
                sexbell_id, sexbell_id
            );
        }
    } else {
        serial_println!(
            "[bootgraph.cap.grant from=kernel to=missing slot=SLOT_BELL target=missing ok=0 optional=1 reason=sexbell_absent]"
        );
    }

    // Spindle capability grants: terminal console bridges (PD 12).
    if spindle_id != 0 {
        use crate::ipc::DOMAIN_REGISTRY;
        use crate::capability::CapabilityData;
        if let Some(pd) = DOMAIN_REGISTRY.get(spindle_id) {
            if sexfiles_id != 0 {
                pd.grant_capability(sex_pdx::SLOT_STORAGE, CapabilityData::Domain(sexfiles_id));
                serial_println!("[kernel.cap.storage.spindle] spindle->sexfiles slot={}", sex_pdx::SLOT_STORAGE);
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_STORAGE target={} ok=1 optional=1]",
                    spindle_id, sexfiles_id
                );
            } else {
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_STORAGE target=missing ok=0 optional=1 reason=sexfiles_absent]",
                    spindle_id
                );
            }
            if sexbell_id != 0 {
                pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(sexbell_id));
                serial_println!("[kernel.cap.bell.spindle] spindle->sexbell slot={}", sex_pdx::SLOT_BELL);
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_BELL target={} ok=1 optional=1]",
                    spindle_id, sexbell_id
                );
            } else {
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_BELL target=missing ok=0 optional=1 reason=sexbell_absent]",
                    spindle_id
                );
            }
            if linen_id != 0 {
                pd.grant_capability(sex_pdx::SLOT_LINEN, CapabilityData::Domain(linen_id));
                serial_println!("[kernel.cap.linen.spindle] spindle->linen slot={}", sex_pdx::SLOT_LINEN);
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_LINEN target={} ok=1 optional=1]",
                    spindle_id, linen_id
                );
            } else {
                serial_println!(
                    "[bootgraph.cap.grant from=kernel to={} slot=SLOT_LINEN target=missing ok=0 optional=1 reason=linen_absent]",
                    spindle_id
                );
            }
            serial_println!("[kernel.cap.spindle] storage={} bell={} linen={}",
                sexfiles_id != 0, sexbell_id != 0, linen_id != 0);
        } else {
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_STORAGE target={} ok=0 optional=1 reason=pd_missing]",
                spindle_id, sexfiles_id
            );
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_BELL target={} ok=0 optional=1 reason=pd_missing]",
                spindle_id, sexbell_id
            );
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_LINEN target={} ok=0 optional=1 reason=pd_missing]",
                spindle_id, linen_id
            );
        }
    } else {
        serial_println!(
            "[bootgraph.cap.grant from=kernel to=missing slot=SLOT_STORAGE target={} ok=0 optional=1 reason=spindle_absent]",
            sexfiles_id
        );
    }

    // SexFiles → SexDrive block/DMA route: grant SLOT_BLOCK so DiskFS
    // can send DmaCall messages to the block device server.
    if sexfiles_id != 0 && sexdrive_id != 0 {
        use crate::ipc::DOMAIN_REGISTRY;
        use crate::capability::CapabilityData;
        if let Some(pd) = DOMAIN_REGISTRY.get(sexfiles_id) {
            pd.grant_capability(sex_pdx::SLOT_BLOCK, CapabilityData::Domain(sexdrive_id));
            serial_println!("[kernel.cap.block] sexfiles->sexdrive slot={}", sex_pdx::SLOT_BLOCK);
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_BLOCK target={} ok=1 optional=1]",
                sexfiles_id, sexdrive_id
            );
        } else {
            serial_println!(
                "[bootgraph.cap.grant from=kernel to={} slot=SLOT_BLOCK target={} ok=0 optional=1 reason=pd_missing]",
                sexfiles_id, sexdrive_id
            );
        }
    } else {
        serial_println!(
            "[bootgraph.cap.grant from=kernel to={} slot=SLOT_BLOCK target={} ok=0 optional=1 reason=sexfiles_or_sexdrive_absent]",
            sexfiles_id, sexdrive_id
        );
    }
    serial_println!("[bootgraph.phase25.complete]");

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
                let pd_name = match pd_id {
                    1 => "sexdisplay",
                    2 => "sexdrive",
                    3 => "silk-shell",
                    4 => "sexinput",
                    5 => "sexusb",
                    6 => "silkbar",
                    7 => "linen",
                    8 => "sexstore",
                    9 => "quil",
                    10 => "sexbell",
                    11 => "sexfiles",
                    12 => "spindle",
                    _ => "unknown",
                };
                let entry = unsafe { (*task_ptr).context.rip };
                serial_println!(
                    "[bootgraph.boot.handoff] target={} id={} entry={:#x}",
                    pd_name, pd_id, entry
                );
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
