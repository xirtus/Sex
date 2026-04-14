use crate::apic;
use crate::serial_println;
use x86_64::VirtAddr;

/// The address where we will place the AP trampoline code (must be below 1MB).
pub const TRAMPOLINE_ADDR: u64 = 0x8000;

/// A simple AP trampoline in assembly.
/// In a real system, this would be a separate assembly file compiled to a binary blob.
/// Here we use a byte array for the demonstration.
pub const TRAMPOLINE_CODE: &[u8] = &[
    0xFA,                   // cli
    0xEB, 0xFE,             // jmp $ (Wait loop for demonstration)
    // Real implementation would include:
    // 1. lgdt with a temporary GDT
    // 2. set cr0 PE bit (Protected Mode)
    // 3. ljmp to 32-bit segment
    // 4. set cr4 PAE, EFER.LME, cr0 PG (Long Mode)
    // 5. ljmp to 64-bit kernel entry
];

pub fn boot_aps() {
    let processors = apic::PROCESSORS.lock();
    let ap_count = processors.iter().filter(|p| !p.is_bsp).count();
    
    if ap_count == 0 {
        serial_println!("SMP: No APs found. Running in single-core mode.");
        return;
    }

    serial_println!("SMP: Booting {} Application Processors...", ap_count);

    // 1. Prepare trampoline code at TRAMPOLINE_ADDR
    // SAFETY: We assume 0x8000 is available and mapped.
    unsafe {
        core::ptr::copy_nonoverlapping(
            TRAMPOLINE_CODE.as_ptr(),
            TRAMPOLINE_ADDR as *mut u8,
            TRAMPOLINE_CODE.len(),
        );
    }

    // 2. Send INIT IPI to all APs
    unsafe {
        // Delivery mode 0b101 = INIT
        // Destination shorthand 0b11 = All excluding self
        apic::send_ipi(0, 0, 0b101 | (0b11 << 10));
    }

    // 3. Wait 10ms (approximate)
    for _ in 0..1000000 { x86_64::instructions::nop(); }

    // 4. Send Startup IPI (SIPI)
    // The vector 0x08 corresponds to address 0x08000 (vector * 4096)
    unsafe {
        apic::broadcast_sipi(0x08);
    }

    serial_println!("SMP: SIPI sent to all APs.");
}
