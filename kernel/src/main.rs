#![no_std]
#![no_main]

use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use core::panic::PanicInfo;
use sex_kernel::{init, serial_println};
use alloc::sync::Arc;
use sex_kernel::capability::ProtectionDomain;
use sex_kernel::ipc::DOMAIN_REGISTRY;
use sex_kernel::scheduler::{Task, TaskContext, TaskState};

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
    let mut frame_allocator = unsafe {
        sex_kernel::memory::BitmapFrameAllocator::init(&boot_info.memory_regions, phys_mem_offset)
    };

    // Phase 14: Bootstrap Lock-Free Buddy Allocator
    let total_pages = 0x100000; // 4 GiB
    let metadata_size = total_pages * core::mem::size_of::<sex_kernel::memory::allocator::PageMetadata>();
    let metadata_pages = (metadata_size + 4095) / 4096;
    let metadata_phys = frame_allocator.allocate_contiguous(metadata_pages).expect("OOM for metadata");
    let metadata_vaddr = phys_mem_offset.as_u64() + metadata_phys.start_address().as_u64();

    unsafe {
        sex_kernel::memory::allocator::GLOBAL_ALLOCATOR.init_from_mmap(
            0, total_pages as u64 * 4096, metadata_vaddr
        );
    }

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

            // Phase 13.2.1: Register PD 0 (Kernel/Root)
            let root_pd = alloc::boxed::Box::into_raw(alloc::boxed::Box::new(sex_kernel::capability::ProtectionDomain::new(0, 0)));
            sex_kernel::ipc::DOMAIN_REGISTRY.insert(0, root_pd);
            sex_kernel::core_local::CoreLocal::get().set_pd(0);
            unsafe { sex_kernel::capabilities::engine::CapEngine::grant_initial_rights(&*root_pd); }
            }

    }

    // 4. Initialize Protection Domains (PKU)
    if sex_kernel::pku::is_pku_supported() {
        unsafe { 
            sex_kernel::pku::enable_pku(); 
            sex_kernel::pku::Pkru::write(0xFFFF_FFFF);
        }
    }

    // 5. Initialize Scheduler
    // Note: SCHEDULERS is already initialized as a static array

    // 6. Bootstrap Advanced Interaction Suite (Simulated)
    sex_kernel::bootstrap_advanced_services();

    // --- PHASE 3: SAS INITRD BOOTSTRAP ---
    serial_println!("--------------------------------------------------");
    serial_println!("Sex Microkernel: Bootstrapping SAS Ecosystem...");

    if let Some(ramdisk_addr) = boot_info.ramdisk_addr.into_option() {
        let ramdisk_len = boot_info.ramdisk_len;
        let ramdisk_vaddr = x86_64::VirtAddr::new(ramdisk_addr + phys_mem_offset.as_u64());
        
        sex_kernel::initrd::bootstrap_initrd(ramdisk_vaddr, ramdisk_len, global_vas)
            .expect("INITRD: Bootstrap failed");
    }

    serial_println!("SAS Ecosystem ready. Handoff to system servers.");
    serial_println!("--------------------------------------------------");

    // Phase 8: Root shell and system servers bootstrap
    sex_kernel::init::init();

    // Phase 16: Performance Benchmarking
    sex_kernel::benchmark::run_maturity_benchmarks();

    // Start execution on first core
    let sched = &sex_kernel::scheduler::SCHEDULERS[0];
    if let Some((_old_ctx, next_ctx)) = sched.tick() {
        unsafe {
            sex_kernel::scheduler::Scheduler::switch_to(core::ptr::null_mut(), next_ctx);
        }
    } else {
        panic!("BOOT: No tasks found in runqueue!");
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
