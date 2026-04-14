use crate::serial_println;
use crate::capability::{MemoryCapData, CapabilityData, InterruptCapData};
use crate::ipc::DOMAIN_REGISTRY;
use x86_64::VirtAddr;
use alloc::sync::Arc;
use core::alloc::Layout;
use crate::capability::ProtectionDomain;

/// DDE-Sex: Device sexdrive Environment for the Sex Microkernel.
/// This module provides a shim for Linux/BSD kernel APIs, allowing 
/// unmodified or lightly patched sexdrives to run in isolated PDs.

pub struct DdeContext {
    pub sexdrive_pd_id: u32,
    pub name: &'static str,
}

impl DdeContext {
    pub fn new(id: u32, name: &'static str) -> Self {
        Self { sexdrive_pd_id: id, name }
    }
}

// --- Linux/BSD Emulation Types (lx_emul style) ---

#[repr(C)]
pub struct resource {
    pub start: u64,
    pub end: u64,
    pub name: *const u8,
    pub flags: u64,
}

#[repr(C)]
pub struct device {
    pub name: [u8; 32],
    pub release: Option<extern "C" fn(*mut device)>,
}

#[repr(C)]
pub struct pci_dev {
    pub dev: device,
    pub vendor: u16,
    pub device: u16,
    pub bus: u8,
    pub devfn: u8,
    pub resource: [resource; 6],
}

/// Linux-equivalent kmalloc (DDE Shim).
#[no_mangle]
pub extern "C" fn kmalloc(size: usize, _flags: i32) -> *mut u8 {
    dde_kmalloc(size)
}

/// Linux-equivalent kfree (DDE Shim).
#[no_mangle]
pub extern "C" fn kfree(ptr: *mut u8) {
    // In a real system, we'd need the size for dealloc, or use a size-aware allocator
}

/// Linux-equivalent request_irq (DDE Shim).
#[no_mangle]
pub extern "C" fn request_irq(irq: u32, handler: extern "C" fn(u64) -> u64, _flags: u64, _name: *const u8, _dev: *mut u8) -> i32 {
    match dde_request_irq(irq as u8, handler) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

// --- Concurrency Shims (Phase 2) ---

#[repr(C)]
pub struct spinlock_t {
    pub locked: core::sync::atomic::AtomicU32,
}

/// Linux-equivalent spin_lock_init (DDE Shim).
#[no_mangle]
pub extern "C" fn spin_lock_init(lock: *mut spinlock_t) {
    unsafe {
        (*lock).locked.store(0, core::sync::atomic::Ordering::SeqCst);
    }
}

/// Linux-equivalent spin_lock (DDE Shim).
/// Uses atomic CAS with a back-off yield to avoid deadlock.
#[no_mangle]
pub extern "C" fn _raw_spin_lock(lock: *mut spinlock_t) {
    let lock = unsafe { &*lock };
    let mut count = 0;
    while lock.locked.compare_exchange(0, 1, core::sync::atomic::Ordering::Acquire, core::sync::atomic::Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
        count += 1;
        if count > 1000 {
            // Call sys_yield (ID 6) instead of 'hlt' to avoid Ring 3 GPF
            crate::servers::sexc::sexc_syscall(6, 0, 0, 0); 
            count = 0;
        }
    }
}

/// Linux-equivalent spin_unlock (DDE Shim).
#[no_mangle]
pub extern "C" fn _raw_spin_unlock(lock: *mut spinlock_t) {
    let lock = unsafe { &*lock };
    lock.locked.store(0, core::sync::atomic::Ordering::Release);
}

/// Linux-equivalent spin_lock_irqsave (DDE Shim).
/// Note: In Ring 3, we cannot disable interrupts, so we rely on the 
/// back-off yield in the spinlock.
#[no_mangle]
pub extern "C" fn _raw_spin_lock_irqsave(lock: *mut spinlock_t) -> u64 {
    _raw_spin_lock(lock);
    0 // Return dummy flags
}

#[no_mangle]
pub extern "C" fn _raw_spin_unlock_irqrestore(lock: *mut spinlock_t, _flags: u64) {
    _raw_spin_unlock(lock);
}

// --- DMA Allocator (Phase 3 - Hardened) ---

use core::sync::atomic::AtomicU64;

static DMA_BUMP_PTR: AtomicU64 = AtomicU64::new(0x_D000_0000); // Dedicated DMA region

/// Linux-equivalent dma_alloc_coherent (DDE Shim).
/// Allocates physically contiguous, pinned memory for DMA using the kernel allocator.
#[no_mangle]
pub extern "C" fn dma_alloc_coherent(_dev: *mut device, size: usize, dma_handle: *mut u64, _flags: i32) -> *mut u8 {
    let num_frames = (size + 4095) / 4096;
    
    let mut gvas_lock = crate::memory::GLOBAL_VAS.lock();
    if let Some(ref mut gvas) = *gvas_lock {
        if let Some(frame) = gvas.frame_allocator.allocate_contiguous(num_frames) {
            let phys_addr = frame.start_address().as_u64();
            serial_println!("VAMPIRE: dma_alloc_coherent(size: {}) -> Phys: {:#x}", size, phys_addr);
            unsafe {
                *dma_handle = phys_addr;
            }
            // In SASOS, we return the virtual address (identity mapped for kernel buffers)
            return phys_addr as *mut u8;
        }
    }
    
    serial_println!("VAMPIRE: DMA allocation failed!");
    core::ptr::null_mut()
}

/// Linux-equivalent dma_free_coherent.
#[no_mangle]
pub extern "C" fn dma_free_coherent(_dev: *mut device, _size: usize, _vaddr: *mut u8, _dma_handle: u64) {
    serial_println!("VAMPIRE: dma_free_coherent()");
}

/// Linux-equivalent dma_map_single.
#[no_mangle]
pub extern "C" fn dma_map_single(_dev: *mut device, ptr: *mut u8, size: usize, _dir: i32) -> u64 {
    serial_println!("VAMPIRE: dma_map_single({:p}, size: {})", ptr, size);
    // In SASOS, virtual == physical for kernel-managed buffers often
    ptr as u64
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
    // In a real system, we'd call the sext to map the hardware range.
    Ok(VirtAddr::new(phys_addr))
}

/// Equivalent to Linux's request_irq().
/// Connects a hardware interrupt to the sexdrive's asynchronous ring buffer.
pub fn dde_request_irq(irq: u8, handler: extern "C" fn(u64) -> u64) -> Result<(), &'static str> {
    serial_println!("DDE: request_irq {} with handler at {:p}", irq, handler);
    
    // 1. Create an Interrupt Capability
    let _cap_data = CapabilityData::Interrupt(InterruptCapData { irq });
    
    // 2. Grant it to the sexdrive's PD (Self)
    // In a real DDE, we'd lookup the current PD.
    
    Ok(())
}

// --- User-Space Buffer Cache (Phase 4) ---

use alloc::collections::BTreeMap;
use spin::RwLock;

pub struct CachedPage {
    pub data: *mut u8,
    pub is_dirty: bool,
}

lazy_static::lazy_static! {
    /// Localized Buffer Cache for vampired filesystems.
    pub static ref BUFFER_CACHE: RwLock<BTreeMap<u64, CachedPage>> = RwLock::new(BTreeMap::new());
}

/// Linux-equivalent find_get_page (DDE Shim).
#[no_mangle]
pub extern "C" fn find_get_page(index: u64) -> *mut u8 {
    let cache = BUFFER_CACHE.read();
    if let Some(page) = cache.get(&index) {
        return page.data;
    }
    
    // Fallback: Allocate new page from PMM
    core::ptr::null_mut()
}

/// Linux-equivalent mark_page_dirty.
#[no_mangle]
pub extern "C" fn mark_page_dirty(page_ptr: *mut u8) {
    let mut cache = BUFFER_CACHE.write();
    // In a real system, we'd find the index from the pointer
    serial_println!("VAMPIRE: mark_page_dirty({:p})", page_ptr);
}

/// Linux-equivalent set_page_dirty.
#[no_mangle]
pub extern "C" fn set_page_dirty(page_ptr: *mut u8) -> i32 {
    mark_page_dirty(page_ptr);
    1
}

#[repr(C)]
pub struct bio_vec {
    pub bv_page: *mut u8,
    pub bv_len: u32,
    pub bv_offset: u32,
}

#[repr(C)]
pub struct bio {
    pub bi_vcnt: u16,
    pub bi_io_vec: *mut bio_vec,
    pub bi_iter_sector: u64,
}

/// Linux-equivalent submit_bio (Vampire I/O Bridge).
/// Translates Linux Block I/O requests into SexOS Descriptor Rings.
#[no_mangle]
pub unsafe extern "C" fn submit_bio(bio_ptr: *mut bio) {
    let bio = &*bio_ptr;
    serial_println!("VAMPIRE: submit_bio() -> Translating {} bio_vecs to sDDF Descriptors.", bio.bi_vcnt);
    
    let vecs = core::slice::from_raw_parts(bio.bi_io_vec, bio.bi_vcnt as usize);
    
    for (i, v) in vecs.iter().enumerate() {
        serial_println!("VAMPIRE:   [{}] Phys: {:p}, Len: {}, Offset: {}", 
            i, v.bv_page, v.bv_len, v.bv_offset);
        
        // 1. Create SexOS IoDescriptor
        let desc = crate::servers::storage::IoDescriptor {
            lba: bio.bi_iter_sector + (i as u64 * 8), // Simplified LBA mapping
            count: (v.bv_len / 512) as u32,
            buffer_phys: v.bv_page as u64 + v.bv_offset as u64,
            op: 0, // Assume Read for now
        };

        // 2. Enqueue to Storage Server (Conceptual)
        serial_println!("VAMPIRE:   Enqueued Descriptor: LBA={}, Phys={:#x}", desc.lba, desc.buffer_phys);
    }
}

/// Linux-equivalent ioctl (Vampire Bridge).
/// Routes DRM and Input IOCTLs to SexOS native servers.
#[no_mangle]
pub extern "C" fn drm_ioctl(fd: i32, request: u64, arg: *mut u8) -> i32 {
    serial_println!("VAMPIRE: drm_ioctl(fd: {}, req: {:#x})", fd, request);
    
    match request {
        0xC0106400 => { // DRM_IOCTL_VERSION
            serial_println!("VAMPIRE: Handing DRM_VERSION to lifted driver.");
            0
        },
        0x40086444 => { // DRM_IOCTL_MODE_CREATE_DUMB
            serial_println!("VAMPIRE: Redirecting Dumb Buffer creation to srv_drm.");
            0
        },
        _ => 0,
    }
}

/// Linux-equivalent read for input devices (Vampire Bridge).
#[no_mangle]
pub extern "C" fn input_read(fd: i32, buffer: *mut u8, count: usize) -> isize {
    serial_println!("VAMPIRE: input_read(fd: {}) -> Fetching from sexinput ring.", fd);
    // 1. Dequeue from sexinput::event_queue
    // 2. Copy into provided buffer
    0
}

// --- PCI Support (Real Implementation) ---

pub struct PciDevice {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_id: u8,
    pub subclass_id: u8,
}

impl PciDevice {
    pub fn read_u32(&self, offset: u8) -> u32 {
        unsafe { pci_config_read(self.bus, self.dev, self.func, offset) }
    }

    pub fn write_u32(&self, offset: u8, value: u32) {
        unsafe { pci_config_write(self.bus, self.dev, self.func, offset, value) }
    }
}

/// Reads a 32-bit value from the PCI configuration space.
pub unsafe fn pci_config_read(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    let address = ((bus as u32) << 16) | ((slot as u32) << 11) |
                  ((func as u32) << 8) | (offset as u32 & 0xFC) | 0x8000_0000;
    
    // I/O ports for PCI configuration space
    x86_64::instructions::port::Port::new(0xCF8).write(address);
    x86_64::instructions::port::Port::new(0xCFC).read()
}

/// Writes a 32-bit value to the PCI configuration space.
pub unsafe fn pci_config_write(bus: u8, slot: u8, func: u8, offset: u8, value: u32) {
    let address = ((bus as u32) << 16) | ((slot as u32) << 11) |
                  ((func as u32) << 8) | (offset as u32 & 0xFC) | 0x8000_0000;
    
    x86_64::instructions::port::Port::new(0xCF8).write(address);
    x86_64::instructions::port::Port::new(0xCFC).write(value);
}

pub fn dde_pci_enumerate() -> alloc::vec::Vec<PciDevice> {
    let mut devices = alloc::vec::Vec::new();
    for bus in 0..256 {
        for slot in 0..32 {
            for func in 0..8 {
                let vendor_id = unsafe { pci_config_read(bus as u8, slot as u8, func as u8, 0) } as u16;
                if vendor_id != 0xFFFF {
                    let device_id = (unsafe { pci_config_read(bus as u8, slot as u8, func as u8, 0) } >> 16) as u16;
                    let class_rev = unsafe { pci_config_read(bus as u8, slot as u8, func as u8, 8) };
                    let class_id = (class_rev >> 24) as u8;
                    let subclass_id = (class_rev >> 16) as u8;

                    devices.push(PciDevice {
                        bus: bus as u8,
                        dev: slot as u8,
                        func: func as u8,
                        vendor_id,
                        device_id,
                        class_id,
                        subclass_id,
                    });
                }
            }
        }
    }
    devices
}

pub fn dde_pci_register_sexdrive(pd: &ProtectionDomain, vendor: u16, device: u16) -> Result<PciDevice, &'static str> {
    serial_println!("DDE: Searching for PCI device {:#x}:{:#x} for PD {}...", vendor, device, pd.id);
    
    // 1. Enumerate and find device
    let devices = dde_pci_enumerate();
    let dev_info = devices.into_iter().find(|d| d.vendor_id == vendor && d.device_id == device)
        .ok_or("DDE: Device not found")?;

    // 2. Grant PCI Capability to the PD
    let cap_id = pd.grant(CapabilityData::Pci(PciCapData {
        bus: dev_info.bus,
        dev: dev_info.dev,
        func: dev_info.func,
        vendor_id: vendor,
        device_id: device,
    }));

    serial_println!("DDE: Granted PCI Capability ID {} for device {:#x}:{:#x}", cap_id, vendor, device);
    
    Ok(dev_info)
}
