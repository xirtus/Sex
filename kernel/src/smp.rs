use crate::serial_println;
use limine::request::MpResponse;
use limine::mp::MpInfo;

pub fn boot_aps(smp: &MpResponse) {
    let cpus = smp.cpus();
    let ap_count = cpus.len() - 1;
    
    if ap_count == 0 {
        serial_println!("SMP: No APs found. Running in single-core mode.");
        return;
    }

    serial_println!("SMP: Booting {} Application Processors via Limine...", ap_count);

    for cpu in cpus {
        if cpu.lapic_id == smp.bsp_lapic_id {
            continue;
        }

        // Set the entry point for the AP via bootstrap method
        cpu.bootstrap(limine_ap_entry, 0);
    }

    serial_println!("SMP: All APs signaled to boot.");
}

/// The entry point for Application Processors via Limine SMP protocol.
extern "C" fn limine_ap_entry(info: &MpInfo) -> ! {
    let lapic_id = info.lapic_id;

    serial_println!("SMP: Core (LAPIC ID {}) online via Limine. Waiting for SAS sync...", lapic_id);

    // 1. Wait for BSP to publish SAS page tables
    while !crate::hal::HAL.is_paging_ready() {
        core::hint::spin_loop();
    }
    let cr3_val = crate::hal::HAL.get_paging_cr3();
    unsafe {
        use x86_64::registers::control::{Cr3, Cr3Flags};
        use x86_64::structures::paging::PhysFrame;
        use x86_64::PhysAddr;
        Cr3::write(PhysFrame::from_start_address(PhysAddr::new(cr3_val)).unwrap(), Cr3Flags::empty());
    }

    ap_main(lapic_id);
}

pub fn ap_main(lapic_id: u32) -> ! {
    serial_println!("SMP: Core {} entering local scheduler loop", lapic_id);

    // 2. Initialize Core-Local storage
    unsafe {
        crate::core_local::CoreLocal::init(lapic_id);
    }

    // 2.1 Enable PKU on this core
    if crate::pku::is_pku_supported() {
        unsafe { crate::pku::enable_pku(); }
    }

    // 3. Initialize HAL (GDT, IDT) for this AP
    crate::hal::init(); 

    // 4. Local scheduler instance
    let mut local_scheduler = crate::scheduler::LocalScheduler::new(lapic_id);

    // 5. Enable Interrupts
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
