#![no_std]
#![no_main]

use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use core::panic::PanicInfo;
use sex_kernel::{init, serial_println};
use alloc::sync::Arc;
use sex_kernel::capability::ProtectionDomain;
use sex_kernel::ipc::DOMAIN_REGISTRY;
use sex_kernel::scheduler::{Task, TaskContext, TaskState, init_core};

const BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(bootloader_api::config::Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    sex_kernel::vga_println!("--------------------------------------------------");
    sex_kernel::vga_println!("Sex Microkernel v0.1 - SASOS Core Bootstrap");
    sex_kernel::vga_println!("--------------------------------------------------");

    // 1. Initialize HAL (GDT, IDT)
    sex_kernel::hal::init();
    
    // Disable legacy PIC
    unsafe {
        use pic8259::ChainedPics;
        let mut pics = ChainedPics::new(0x20, 0x28);
        pics.disable();
    }

    let phys_mem_offset = x86_64::VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    
    // 2. Initialize Memory (Sexting, Frame Allocator, Global VAS)
    let mapper = unsafe { sex_kernel::memory::init_sexting(phys_mem_offset) };
    let frame_allocator = unsafe {
        sex_kernel::memory::BitmapFrameAllocator::init(&boot_info.memory_regions, phys_mem_offset)
    };

    let global_vas_inst = sex_kernel::memory::GlobalVas {
        mapper,
        frame_allocator,
        phys_mem_offset,
    };
    
    {
        let mut gvas = sex_kernel::memory::GLOBAL_VAS.lock();
        *gvas = Some(global_vas_inst);
    }
    
    let mut gvas_locked = sex_kernel::memory::GLOBAL_VAS.lock();
    let global_vas = gvas_locked.as_mut().unwrap();

    // Initialize Heap
    sex_kernel::allocator::init_heap(&mut global_vas.mapper, &mut global_vas.frame_allocator)
        .expect("Heap initialization failed");

    // 3. Initialize Multi-Core & APIC
    if let Some(rsdp_addr) = boot_info.rsdp_addr.into_option() {
        sex_kernel::apic::init_apic(rsdp_addr, phys_mem_offset);
        unsafe {
            // Map Hardware IRQs
            sex_kernel::apic::map_irq(1, 0x21, 0, phys_mem_offset); // Keyboard
            sex_kernel::apic::map_irq(12, 0x2C, 0, phys_mem_offset); // Mouse
            sex_kernel::core_local::CoreLocal::init(0);
        }
        // sex_kernel::smp::boot_aps(); // Secondary cores
    }

    // 4. Initialize Protection Domains (PKU)
    if sex_kernel::pku::is_pku_supported() {
        unsafe { 
            sex_kernel::pku::enable_pku(); 
            sex_kernel::pku::Pkru::write(0xFFFF_FFFF);
        }
    }

    // 5. Initialize Scheduler
    init_core(0);

    // 6. Bootstrap Advanced Interaction Suite (Font, Wayland, AI)
    // Note: In a production system, these would be services managed by sexit.
    // For this bootstrap, we initialize the suite before jumping to PID 1.
    sex_kernel::bootstrap_advanced_services();

    // --- PHASE 3: SAS INITRD BOOTSTRAP ---
    serial_println!("--------------------------------------------------");
    serial_println!("Sex Microkernel: Bootstrapping SAS Ecosystem...");

    if let Some(ramdisk_addr) = boot_info.ramdisk_addr.into_option() {
        let ramdisk_len = boot_info.ramdisk_len;
        let ramdisk_vaddr = x86_64::VirtAddr::new(ramdisk_addr + phys_mem_offset.as_u64());
        
        sex_kernel::initrd::bootstrap_initrd(ramdisk_vaddr, ramdisk_len, global_vas)
            .expect("INITRD: Bootstrap failed");
    } else {
        serial_println!("INITRD: No ramdisk provided by bootloader!");
    }

    serial_println!("SAS Ecosystem ready. Handoff to SEXIT (PID 1).");
    serial_println!("--------------------------------------------------");

    // Start execution
    unsafe {
        if let Some(ref mut sched) = sex_kernel::scheduler::SCHEDULERS[0] {
            // Pick PID 1 (sexit) and enter Ring 3!
            sched.tick();
            
            if let Some(ref current_task_mutex) = sched.current_task {
                let current = current_task_mutex.lock();
                let next_ctx = &current.context;
                
                let mut dummy_ctx = TaskContext::new(0, 0, 
                    Arc::new(ProtectionDomain::new(0, 0)), false);
                
                sex_kernel::scheduler::Scheduler::switch_to(&mut dummy_ctx, next_ctx);
            } else {
                panic!("BOOT: SEXIT task not found in runqueue!");
            }
        }
    }

    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("KERNEL PANIC: {}", info);
    loop {
        x86_64::instructions::hlt();
    }
}
