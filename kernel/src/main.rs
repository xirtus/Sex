#![no_std]
#![no_main]

use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use core::panic::PanicInfo;
use sasos_kernel::{init, serial_println};

const BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(bootloader_api::config::Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    serial_println!("--------------------------------------------------");
    serial_println!("SASOS Microkernel v0.1 - Hello from global VAS");
    
    // Initialize the kernel (HAL, etc.)
    init();
    
    serial_println!("Boot successful. Initializing Phase 1: Memory...");

    let phys_mem_offset = x86_64::VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    
    // Initialize Paging
    let mut mapper = unsafe { sasos_kernel::memory::init_paging(phys_mem_offset) };

    // Initialize Frame Allocator
    let mut frame_allocator = unsafe {
        sasos_kernel::memory::BootInfoFrameAllocator::init(&boot_info.memory_regions)
    };

    serial_println!("Memory: Paging and Frame Allocator initialized.");

    // Initialize Phase 1.2: Protection Domains (PKU)
    if sasos_kernel::pku::is_pku_supported() {
        serial_println!("PKU: Hardware support detected.");
        unsafe { sasos_kernel::pku::enable_pku(); }
        
        // Basic PKRU test
        let initial_pkru = sasos_kernel::pku::Pkru::read();
        serial_println!("PKU: Initial PKRU value: {:#010x}", initial_pkru);
        
        // Disable write for key 1
        sasos_kernel::pku::Pkru::set_permissions(1, false, true);
        let updated_pkru = sasos_kernel::pku::Pkru::read();
        serial_println!("PKU: Updated PKRU value (Key 1 write disabled): {:#010x}", updated_pkru);
        
        // Restore permissions
        unsafe { sasos_kernel::pku::Pkru::write(initial_pkru); }
        serial_println!("PKU: PKRU restored.");
    } else {
        serial_println!("PKU: Hardware support NOT detected. System will run without hardware-accelerated protection domains.");
    }

    // Test virtual-to-physical translation
    use x86_64::structures::paging::Mapper;
    let addresses = [
        // the identity-mapped vga buffer
        0xb8000,
        // some code page
        0x201008,
        // some stack page
        0x0100_0020_1a10,
        // virtual address mapped to physical address 0
        phys_mem_offset.as_u64(),
    ];

    for &address in &addresses {
        let virt = x86_64::VirtAddr::new(address);
        let phys = mapper.translate_addr(virt);
        serial_println!("{:?} -> {:?}", virt, phys);
    }

    serial_println!("SASOS: System ready (Phase 1.1).");
    serial_println!("--------------------------------------------------");

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
