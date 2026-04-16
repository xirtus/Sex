use crate::apic;
use crate::serial_println;
use x86_64::VirtAddr;
use core::arch::global_asm;

/// The address where we will place the AP trampoline code (must be below 1MB).
pub const TRAMPOLINE_ADDR: u64 = 0x8000;

/// The address where we store the kernel entry point for APs.
pub const AP_ENTRY_PTR: u64 = 0x500;

/// The address where we store the P4 physical address for APs.
pub const AP_P4_PTR: u64 = 0x508;

global_asm!(
    ".code16",
    ".global trampoline_start",
    "trampoline_start:",
    "    cli",
    "    cld",
    "    xor ax, ax",
    "    mov ds, ax",
    "    mov es, ax",
    "    mov ss, ax",
    "",
    "    # 1. Load 32-bit GDT",
    "    lgdt [gdt32_ptr - trampoline_start + 0x8000]",
    "",
    "    # 2. Enter Protected Mode",
    "    mov eax, cr0",
    "    or eax, 1",
    "    mov cr0, eax",
    "",
    "    # 3. Far jump to 32-bit protected mode",
    "    ljmp $0x08, $(protected_mode - trampoline_start + 0x8000)",
    "",
    ".code32",
    "protected_mode:",
    "    mov ax, 0x10",
    "    mov ds, ax",
    "    mov es, ax",
    "    mov ss, ax",
    "",
    "    # 4. Enable PAE",
    "    mov eax, cr4",
    "    or eax, 1 << 5",
    "    mov cr4, eax",
    "",
    "    # 5. Load P4 Page Table (BSP's CR3 passed via 0x508)",
    "    mov eax, [0x508]",
    "    mov cr3, eax",
    "",
    "    # 6. Enable Long Mode in EFER MSR",
    "    mov ecx, 0xC0000080",
    "    rdmsr",
    "    or eax, 1 << 8",
    "    wrmsr",
    "",
    "    # 7. Enable Paging and Protected Mode",
    "    mov eax, cr0",
    "    or eax, 1 << 31",
    "    mov cr0, eax",
    "",
    "    # 8. Load 64-bit GDT",
    "    lgdt [gdt64_ptr - trampoline_start + 0x8000]",
    "",
    "    # 9. Far jump to 64-bit long mode",
    "    ljmp $0x18, $(long_mode - trampoline_start + 0x8000)",
    "",
    ".code64",
    "long_mode:",
    "    # 10. Clear segment registers for long mode",
    "    mov ax, 0x00",
    "    mov ds, ax",
    "    mov es, ax",
    "    mov fs, ax",
    "    mov gs, ax",
    "    mov ss, ax",
    "",
    "    # 11. Jump to higher-half kernel entry point (stored at 0x500)",
    "    mov rax, [0x500]",
    "    jmp rax",
    "",
    ".align 8",
    "gdt32:",
    "    .quad 0x0000000000000000",
    "    .quad 0x00cf9a000000ffff # 32-bit code",
    "    .quad 0x00cf92000000ffff # 32-bit data",
    "gdt32_ptr:",
    "    .word . - gdt32 - 1",
    "    .long gdt32 - trampoline_start + 0x8000",
    "",
    "gdt64:",
    "    .quad 0x0000000000000000",
    "    .quad 0x0000000000000000",
    "    .quad 0x0000000000000000",
    "    .quad 0x00af9a000000ffff # 64-bit code",
    "gdt64_ptr:",
    "    .word . - gdt64 - 1",
    "    .quad gdt64 - trampoline_start + 0x8000",
    "",
    ".global trampoline_end",
    "trampoline_end:"
);

extern "C" {
    fn trampoline_start();
    fn trampoline_end();
}

pub fn boot_aps() {
    let processors = apic::PROCESSORS.lock();
    let ap_count = processors.iter().filter(|p| !p.is_bsp).count();
    
    if ap_count == 0 {
        serial_println!("SMP: No APs found. Running in single-core mode.");
        return;
    }

    serial_println!("SMP: Booting {} Application Processors...", ap_count);

    // 0. Copy trampoline to 0x8000
    let src = trampoline_start as *const u8;
    let dst = TRAMPOLINE_ADDR as *mut u8;
    let size = unsafe { (trampoline_end as usize) - (trampoline_start as usize) };
    unsafe {
        core::ptr::copy_nonoverlapping(src, dst, size);
    }

    // 1. Write the kernel entry point for APs
    unsafe {
        *(AP_ENTRY_PTR as *mut u64) = ap_kernel_entry as *const () as u64;
    }

    // 2. Write the P4 physical address for APs
    use x86_64::registers::control::Cr3;
    let (p4_frame, _) = Cr3::read();
    unsafe {
        *(AP_P4_PTR as *mut u64) = p4_frame.start_address().as_u64();
    }

    // 3. Send INIT IPI to all APs
    unsafe {
        // Broadcast INIT to all excluding self
        apic::send_ipi(0, 0, 0b101 | (0b11 << 18) | (1 << 14) | (1 << 15));
    }

    // 4. Wait 10ms (approximate)
    for _ in 0..10000000 { x86_64::instructions::nop(); }

    // 5. Send Startup IPI (SIPI) twice
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

    serial_println!("SMP: Core (LAPIC ID {}) online. Waiting for SAS sync...", lapic_id);

    // 2. Wait for BSP to publish SAS page tables (Phase 17 lock-free)
    while !crate::hal::PAGING_READY.load(core::sync::atomic::Ordering::Acquire) {
        core::hint::spin_loop();
    }
    let cr3_val = crate::hal::PAGING_CR3.load(core::sync::atomic::Ordering::Acquire);
    unsafe {
        use x86_64::registers::control::Cr3;
        use x86_64::structures::paging::{PhysFrame, PageTableFlags};
        use x86_64::PhysAddr;
        Cr3::write(PhysFrame::from_start_address(PhysAddr::new(cr3_val)).unwrap(), PageTableFlags::empty());
    }

    ap_main(lapic_id as u32);
}

pub fn ap_main(lapic_id: u32) -> ! {
    serial_println!("SMP: Core {} entering local scheduler loop", lapic_id);

    // 3. Initialize Core-Local storage
    unsafe {
        crate::core_local::CoreLocal::init(lapic_id);
    }

    // 4. Initialize HAL (GDT, IDT) for this AP
    // APs skip the SAS page-table setup part of hal::init in practice
    crate::hal::init(); 

    // 5. Local scheduler instance
    let mut local_scheduler = crate::scheduler::LocalScheduler::new(lapic_id);

    // 6. Enable Interrupts
    x86_64::instructions::interrupts::enable();
    
    loop {
        // Dequeue from this core’s PDX ring buffer
        if let Some(msg) = crate::ipc::dequeue_local() {
            local_scheduler.handle_pdx_message(msg);
        }

        // Tick local scheduler
        if let Some((old_ctx, next_ctx)) = local_scheduler.tick() {
            unsafe {
                crate::scheduler::Scheduler::switch_to(old_ctx, next_ctx);
            }
        }
        
        x86_64::instructions::hlt();
    }
}
