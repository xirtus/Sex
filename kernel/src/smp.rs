use crate::apic;
use crate::serial_println;
use x86_64::VirtAddr;

/// The address where we will place the AP trampoline code (must be below 1MB).
pub const TRAMPOLINE_ADDR: u64 = 0x8000;

/// The address where we store the kernel entry point for APs.
pub const AP_ENTRY_PTR: u64 = 0x500;

/// The address of the P4 page table (BSP's table).
pub const P4_TABLE_ADDR: u64 = 0x1000;

pub fn boot_aps() {
    let processors = apic::PROCESSORS.lock();
    let ap_count = processors.iter().filter(|p| !p.is_bsp).count();
    
    if ap_count == 0 {
        serial_println!("SMP: No APs found. Running in single-core mode.");
        return;
    }

    serial_println!("SMP: Booting {} Application Processors...", ap_count);

    // 1. Write the kernel entry point for APs to a fixed location
    unsafe {
        *(AP_ENTRY_PTR as *mut u64) = ap_kernel_entry as *const () as u64;
    }

    // 2. Send INIT IPI to all APs
    unsafe {
        // Delivery mode 0b101 = INIT, Level 1, Assert 1, Shorthand 0b11 (All excluding self)
        apic::send_ipi(0, 0, 0b101 | (0b11 << 10) | (1 << 14) | (1 << 15));
    }

    // 3. Wait 10ms (approximate)
    for _ in 0..10000000 { x86_64::instructions::nop(); }

    // 4. Send Startup IPI (SIPI) twice
    for _ in 0..2 {
        unsafe {
            apic::broadcast_sipi(0x08); // Vector 0x08 -> 0x8000
        }
        for _ in 0..1000000 { x86_64::instructions::nop(); }
    }

    serial_println!("SMP: SIPI sequence completed.");
}

/// The entry point for Application Processors in 64-bit Long Mode.
pub extern "C" fn ap_kernel_entry() -> ! {
    // 1. Identify current core
    let lapic_id = unsafe { 
        let lapic_virt = VirtAddr::new(apic::LAPIC_ADDR.load(core::sync::atomic::Ordering::Acquire));
        let lapic_ptr = lapic_virt.as_u64() as *const u32;
        (lapic_ptr.offset(0x020 / 4).read_volatile() >> 24) as u8
    };

    serial_println!("SMP: Core (LAPIC ID {}) online.", lapic_id);

    // 2. Initialize Core-Local storage
    // In a real system, we'd find the core_id from LAPIC_ID mapping
    let core_id = lapic_id as usize; 
    unsafe {
        crate::core_local::CoreLocal::init(core_id as u32);
    }

    // 3. Enable Interrupts and enter the scheduler loop
    x86_64::instructions::interrupts::enable();
    
    serial_println!("SMP: Core {} entering scheduler loop.", core_id);
    
    loop {
        x86_64::instructions::hlt();
    }
}
