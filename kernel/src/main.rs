#![no_std]
#![no_main]

extern crate alloc;

use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use core::panic::PanicInfo;
use sex_kernel::serial_println;
use alloc::sync::Arc;
use sex_kernel::capability::ProtectionDomain;
use sex_kernel::ipc::DOMAIN_REGISTRY;
use sex_kernel::scheduler::{Task, TaskContext, TaskState};
use sex_kernel::pd::create::create_protection_domain;
use x86_64::VirtAddr;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(bootloader_api::config::Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // 1. Initial Serial/VGA setup
    // sex_kernel::serial::init(); // Handled by lazy_static
    sex_kernel::vga::_print(format_args!("Sex Microkernel v1.0.0 Loading...\n"));

    // 2. Hardware Abstraction Layer (GDT, IDT, PIC)
    sex_kernel::hal::init();

    // 3. Memory & Virtual Address Space
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    let mut mapper = unsafe { sex_kernel::memory::init_sexting(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        sex_kernel::memory::BitmapFrameAllocator::init(&boot_info.memory_regions, phys_mem_offset)
    };

    // Phase 14: Bootstrap the Global Virtual Address Space
    let mut global_vas_inst = sex_kernel::memory::GlobalVas {
        mapper,
        frame_allocator,
        phys_mem_offset,
    };

    // 4. Global Allocator Initialization
    sex_kernel::allocator::init_heap(&mut global_vas_inst.mapper, &mut global_vas_inst.frame_allocator)
        .expect("Heap initialization failed");

    {
        let mut gvas = sex_kernel::memory::GLOBAL_VAS.lock();
        *gvas = Some(global_vas_inst);
    }

    // 5. Symmetric Multi-Processing (SMP)
    sex_kernel::apic::init_apic(boot_info.rsdp_addr.into_option().unwrap(), phys_mem_offset);
    sex_kernel::smp::boot_aps();

    // 6. Spawn Core System Domains (Isolation Level 1)
    serial_println!("pd: Spawning core services...");
    
    unsafe {
        // Root Domain (Slot 0)
        let root_pd = alloc::boxed::Box::into_raw(alloc::boxed::Box::new(sex_kernel::capability::ProtectionDomain::new(0, 0)));
        DOMAIN_REGISTRY.insert(0, root_pd);
    }

    let _sext = create_protection_domain("/servers/sext/bin/sext\0", None).expect("sext lost");
    
    // 7. Advanced Interaction Suite (Capability Engine)
    sex_kernel::bootstrap_advanced_services();

    let _sexinput = create_protection_domain("/servers/sexinput/bin/sexinput\0", None).expect("sexinput lost");
    let _sexnet = create_protection_domain("/servers/sexnet/bin/sexnet\0", None).expect("sexnet lost");

    serial_println!("Sex SASOS: Production Ready (Phase 16).");

    // 8. Yield to Scheduler
    if let Some((_, next_ctx)) = sex_kernel::scheduler::SCHEDULERS[0].tick() {
        unsafe {
            sex_kernel::scheduler::Scheduler::switch_to(core::ptr::null_mut(), next_ctx);
        }
    } else {
        panic!("Main loop failed to enqueue!");
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
