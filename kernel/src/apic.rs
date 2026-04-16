use acpi::{AcpiHandler, AcpiTables, PhysicalMapping};
use core::ptr::NonNull;
use x86_64::{PhysAddr, VirtAddr};
use crate::serial_println;
use alloc::vec::Vec;
use conquer_once::spin::Mutex;
use lazy_static::lazy_static;

#[derive(Clone, Copy)]
pub struct SexAcpiHandler {
    pub physical_memory_offset: VirtAddr,
}

impl AcpiHandler for SexAcpiHandler {
    unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> PhysicalMapping<Self, T> {
        let virt_addr = self.physical_memory_offset + physical_address as u64;
        PhysicalMapping::new(
            physical_address,
            NonNull::new(virt_addr.as_mut_ptr()).unwrap(),
            size,
            size,
            self.clone(),
        )
    }

    fn unmap_physical_region<T>(_mapping: &PhysicalMapping<Self, T>) {
        // No-op
    }
}

pub struct ProcessorInfo {
    pub id: u32,
    pub local_apic_id: u8,
    pub is_bsp: bool,
}

pub struct IoApicInfo {
    pub id: u8,
    pub address: u32,
    pub global_system_interrupt_base: u32,
}

lazy_static! {
    pub static ref PROCESSORS: Mutex<Vec<ProcessorInfo>> = Mutex::new(Vec::new());
    pub static ref IO_APICS: Mutex<Vec<IoApicInfo>> = Mutex::new(Vec::new());
}

pub static LAPIC_ADDR: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

pub fn init_apic(rsdp_addr: u64, physical_memory_offset: VirtAddr) {
    let handler = SexAcpiHandler { physical_memory_offset };
    let tables = unsafe { AcpiTables::from_rsdp(handler, rsdp_addr as usize).expect("ACPI: Failed to parse tables") };

    let platform_info = tables.platform_info().expect("ACPI: Failed to get platform info");
    
    if let acpi::InterruptModel::Apic(apic_info) = platform_info.interrupt_model {
        let lapic_virt = physical_memory_offset + apic_info.local_apic_address;
        LAPIC_ADDR.store(lapic_virt.as_u64(), core::sync::atomic::Ordering::Release);
        serial_println!("APIC: Found LAPIC at {:#x}", apic_info.local_apic_address);
        
        unsafe {
            init_local_apic(lapic_virt);
        }

        let mut io_apics = IO_APICS.lock();
        for io_apic in apic_info.io_apics.iter() {
            serial_println!("APIC: Found I/O APIC {} at {:#x}", io_apic.id, io_apic.address);
            io_apics.push(IoApicInfo {
                id: io_apic.id,
                address: io_apic.address,
                global_system_interrupt_base: io_apic.global_system_interrupt_base,
            });
        }

        let mut processors = PROCESSORS.lock();
        if let Some(proc_info) = platform_info.processor_info {
            for proc in proc_info.application_processors {
                processors.push(ProcessorInfo {
                    id: proc.processor_uid,
                    local_apic_id: proc.local_apic_id,
                    is_bsp: false,
                });
            }
            processors.push(ProcessorInfo {
                id: proc_info.boot_processor.processor_uid,
                local_apic_id: proc_info.boot_processor.local_apic_id,
                is_bsp: true,
            });
        }
    }
}

unsafe fn init_local_apic(lapic_virt: VirtAddr) {
    let lapic_ptr = lapic_virt.as_u64() as *mut u32;
    let svr_reg = lapic_ptr.offset(0x0F0 / 4);
    svr_reg.write_volatile(svr_reg.read_volatile() | 0x100 | 0xFF);
}

/// Maps an IRQ to a vector on a specific I/O APIC.
pub unsafe fn map_irq(irq: u8, vector: u8, dest_lapic_id: u8, physical_memory_offset: VirtAddr) {
    let io_apics = IO_APICS.lock();
    
    // Find the correct I/O APIC based on the Global System Interrupt (GSI) base
    // For simplicity, we assume IRQ maps 1:1 to GSI
    let gsi = irq as u32;
    let io_apic = io_apics.iter()
        .find(|io| gsi >= io.global_system_interrupt_base && gsi < io.global_system_interrupt_base + 24)
        .or_else(|| io_apics.first()) // Fallback to first
        .expect("APIC: No suitable I/O APIC found");

    let io_apic_virt = physical_memory_offset + io_apic.address as u64;
    let io_apic_ptr = io_apic_virt.as_u64() as *mut u32;

    let reg_sel = io_apic_ptr;
    let reg_win = io_apic_ptr.offset(4 / 4);

    // Redirection table entry for this IRQ (starts at 0x10, 2 registers per IRQ)
    let relative_irq = gsi - io_apic.global_system_interrupt_base;
    let low_index = 0x10 + relative_irq * 2;
    let high_index = low_index + 1;

    // Write low part: vector, delivery mode (000 = fixed), dest mode (0 = physical), polarity/trigger (0=active high, 0=edge)
    reg_sel.write_volatile(low_index);
    reg_win.write_volatile(vector as u32);

    // Write high part: destination (LAPIC ID)
    reg_sel.write_volatile(high_index);
    reg_win.write_volatile((dest_lapic_id as u32) << 24);
    
    serial_println!("APIC: Mapped GSI {} (IOAPIC {}) to Vector {} (Dest LAPIC {})", 
        gsi, io_apic.id, vector, dest_lapic_id);
}

pub unsafe fn send_ipi(lapic_id: u8, vector: u8, delivery_mode: u32) {
    let lapic_vaddr = LAPIC_ADDR.load(core::sync::atomic::Ordering::Acquire);
    if lapic_vaddr == 0 { return; }
    let lapic_ptr = lapic_vaddr as *mut u32;
    let icr_high = lapic_ptr.offset(0x310 / 4);
    let icr_low = lapic_ptr.offset(0x300 / 4);
    while (icr_low.read_volatile() & (1 << 12)) != 0 {}
    icr_high.write_volatile((lapic_id as u32) << 24);
    let cmd = (delivery_mode << 8) | (vector as u32);
    icr_low.write_volatile(cmd);
}

pub unsafe fn broadcast_sipi(vector: u8) {
    let lapic_vaddr = LAPIC_ADDR.load(core::sync::atomic::Ordering::Acquire);
    if lapic_vaddr == 0 { return; }
    let lapic_ptr = lapic_vaddr as *mut u32;
    let icr_low = lapic_ptr.offset(0x300 / 4);
    while (icr_low.read_volatile() & (1 << 12)) != 0 {}
    let cmd = (0b11 << 18) | (0b110 << 8) | (vector as u32);
    icr_low.write_volatile(cmd);
}
