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

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    serial_println!("--------------------------------------------------");
    serial_println!("SASOS Microkernel v0.1 - Hello from global VAS");
    
    // Initialize the kernel (HAL, etc.)
    init();
    
    serial_println!("Boot successful. System ready.");
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
