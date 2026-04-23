use crate::serial_println;
use x86_64::VirtAddr;
use core::sync::atomic::{AtomicU8, Ordering};

pub static mut SEXDISPLAY_PD_ID: u32 = 0;
static NEXT_PKEY: AtomicU8 = AtomicU8::new(2); 

pub fn init() {
    let mut sexdisp_id = 0; 

    let modules_res = crate::MODULE_REQUEST.response();
    if modules_res.is_none() {
        panic!("FATAL: Limine returned no modules. Check limine.cfg and ISO layout.");
    }
    let modules = modules_res.unwrap();
    serial_println!("init: Found {} Limine modules", modules.modules().len());

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
                    }
                }
                Err(e) => {
                    serial_println!("!! Spawn Error {}: {}", path, e);
                }
            }
        }
    }

    unsafe {
        use crate::ipc::DOMAIN_REGISTRY;
        use crate::capability::CapabilityData;
        for i in 1..=4 {
            if let Some(pd) = DOMAIN_REGISTRY.get(i) {
                if sexdisp_id != 0 {
                    (*pd.cap_table).insert_at(5, CapabilityData::Domain(sexdisp_id));
                }
            }
        }
    }

    serial_println!("init: Revoking kernel write access...");
    // Preemption-safe: Scheduler handles PKRU switch
    // unsafe { crate::pku::wrpkru(0b1100); } 

    serial_println!("init: Ready for Scheduler.");
}

fn pdx_spawn(name: &str, module_data: &[u8]) -> Result<u32, &'static str> {
    use crate::capability::ProtectionDomain;
    use crate::ipc::DOMAIN_REGISTRY;
    use crate::memory::manager::GLOBAL_VAS;
    use x86_64::structures::paging::PageTableFlags;

    static mut NEXT_PD_ID: u32 = 1;
    let pd_id = unsafe { let id = NEXT_PD_ID; NEXT_PD_ID += 1; id };
    let pku_key = if name.contains("sexdisplay") { 1 } else { NEXT_PKEY.fetch_add(1, Ordering::SeqCst) };

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
    
    // Register Task in Scheduler
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
    serial_println!("PDX: Registered {} (PKEY {})", name, pku_key);

    Ok(pd_id)
}

pub unsafe fn jump_to_userland(pd_id: u32, entry: u64, pkru: u32, pku_key: u8) -> ! {
    use crate::gdt;
    let selectors = gdt::get_selectors();
    let user_cs = selectors.user_code_selector.0 as u64 | 3;
    let user_ss = selectors.user_data_selector.0 as u64 | 3;
    let rflags: u64 = 0x202;
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
        rsp_val = in(reg) stack_top & !0xFu64,
        rflags  = in(reg) rflags,
        cs      = in(reg) user_cs,
        rip     = in(reg) entry,
        target_pkru = in(reg) pkru,
        options(noreturn)
    );
}
