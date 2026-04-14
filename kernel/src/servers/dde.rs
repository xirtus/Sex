use crate::serial_println;
use crate::capability::{MemoryCapData, CapabilityData, InterruptCapData};
use crate::ipc::DOMAIN_REGISTRY;
use x86_64::VirtAddr;
use alloc::sync::Arc;
use core::alloc::Layout;

/// DDE-Sex: Device Driver Environment for the Sex Microkernel.
/// This module provides a shim for Linux/BSD kernel APIs, allowing 
/// unmodified or lightly patched drivers to run in isolated PDs.

pub struct DdeContext {
    pub driver_pd_id: u32,
    pub name: &'static str,
}

impl DdeContext {
    pub fn new(id: u32, name: &'static str) -> Self {
        Self { driver_pd_id: id, name }
    }
}

// --- Linux/BSD Shim APIs ---

/// Equivalent to Linux's kmalloc().
/// Allocates memory from the Global SAS heap.
pub fn dde_kmalloc(size: usize) -> *mut u8 {
    unsafe {
        let layout = Layout::from_size_align_unchecked(size, 16);
        let ptr = alloc::alloc::alloc(layout);
        serial_println!("DDE [{}]: kmalloc({}) -> {:p}", "DRIVER", size, ptr);
        ptr
    }
}

/// Equivalent to Linux's kfree().
pub fn dde_kfree(ptr: *mut u8, size: usize) {
    unsafe {
        let layout = Layout::from_size_align_unchecked(size, 16);
        alloc::alloc::dealloc(ptr, layout);
    }
}

/// Equivalent to Linux's ioremap().
/// Maps a physical MMIO range into the Global VAS and grants a capability.
pub fn dde_ioremap(phys_addr: u64, size: u64) -> Result<VirtAddr, &'static str> {
    serial_println!("DDE: ioremap physical {:#x} (size: {})", phys_addr, size);
    
    // In a SASOS, MMIO is often identity-mapped or mapped at a fixed offset.
    // For now, we return the virtual address directly (assuming 1:1 for hardware).
    // In a real system, this would call the Pager to map the hardware range.
    Ok(VirtAddr::new(phys_addr))
}

/// Equivalent to Linux's request_irq().
/// Connects a hardware interrupt to the driver's asynchronous ring buffer.
pub fn dde_request_irq(irq: u8, handler: extern "C" fn(u64) -> u64) -> Result<(), &'static str> {
    serial_println!("DDE: request_irq {} with handler at {:p}", irq, handler);
    
    // 1. Create an Interrupt Capability
    let cap_data = CapabilityData::Interrupt(InterruptCapData { irq });
    
    // 2. Grant it to the driver's PD (Self)
    // In a real DDE, we'd lookup the current PD.
    
    Ok(())
}

/// A "Dummy" symbol generator for DDE-Sex.
/// Used during linking to satisfy unresolved Linux kernel symbols.
#[no_mangle]
pub extern "C" fn dde_dummy_symbol() {
    serial_println!("DDE: Triggered unhandled dummy symbol!");
}

// --- PCI Support (Platform Session) ---

pub struct PciDevice {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
    pub vendor_id: u16,
    pub device_id: u16,
}

pub fn dde_pci_register_driver(vendor: u16, device: u16) -> Option<PciDevice> {
    serial_println!("DDE: Registering driver for PCI {:#x}:{:#x}", vendor, device);
    // Simulate finding the device
    Some(PciDevice {
        bus: 0, dev: 1, func: 0,
        vendor_id: vendor,
        device_id: device,
    })
}
