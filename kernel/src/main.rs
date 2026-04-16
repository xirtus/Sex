#![no_std]
#![no_main]

extern crate alloc;

use limine::request::{FramebufferRequest, HhdmRequest, MemoryMapRequest, RsdpRequest};
use limine::BaseRevision;
use core::panic::PanicInfo;
use sex_kernel::serial_println;
use sex_kernel::ipc::DOMAIN_REGISTRY;
use sex_kernel::pd::create::create_protection_domain;
use x86_64::VirtAddr;

// 1. Limine Protocol Requirements
#[used]
#[link_section = ".requests"]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[link_section = ".requests_start"]
static REQ_START: limine::request::RequestsStartMarker = limine::request::RequestsStartMarker::new();

#[used]
#[link_section = ".requests_end"]
static REQ_END: limine::request::RequestsEndMarker = limine::request::RequestsEndMarker::new();

// 2. Bootloader Information Requests
#[used]
#[link_section = ".requests"]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[link_section = ".requests"]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[link_section = ".requests"]
static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[used]
#[link_section = ".requests"]
static RSDP_REQUEST: RsdpRequest = RsdpRequest::new();

/// The absolute entry point for the SEX microkernel (Pure Rust).
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Ensure the bootloader is compatible with our version of the protocol
    if !BASE_REVISION.is_supported() {
        loop { x86_64::instructions::hlt(); }
    }

    // 1. Capture early bootloader responses
    let hhdm = HHDM_REQUEST.get_response().unwrap();
    let phys_mem_offset = VirtAddr::new(hhdm.offset());
    
    let mmap = MEMORY_MAP_REQUEST.get_response().unwrap();
    let rsdp = RSDP_REQUEST.get_response().unwrap();

    // 2. Initialize Core HAL (GDT, IDT, PIC)
    sex_kernel::hal::init();

    // 3. Setup Virtual Memory & Page Mapping (SASOS)
    let mapper = unsafe { sex_kernel::memory::init_sexting(phys_mem_offset) };
    let frame_allocator = unsafe {
        sex_kernel::memory::BitmapFrameAllocator::init(mmap.entries(), phys_mem_offset)
    };

    // Phase 14: Bootstrap the Global Virtual Address Space
    let mut global_vas_inst = sex_kernel::memory::GlobalVas {
        mapper,
        frame_allocator,
        phys_mem_offset,
    };

    // 4. Global Allocator Initialization (Heap)
    sex_kernel::allocator::init_heap(&mut global_vas_inst.mapper, &mut global_vas_inst.frame_allocator)
        .expect("Heap initialization failed");

    {
        let mut gvas = sex_kernel::memory::GLOBAL_VAS.lock();
        *gvas = Some(global_vas_inst);
    }

    // 5. Symmetric Multi-Processing (SMP) - Deliverable 2
    sex_kernel::apic::init_apic(rsdp.address().as_ptr() as u64, phys_mem_offset);
    sex_kernel::smp::boot_aps();

    // 6. Spawn Core System Domains (Isolation Level 1)
    serial_println!("Sex SASOS: Production Ready (Phase 16).");
    serial_println!("pd: Spawning core services...");
    
    // Root Domain (Slot 0)
    let root_pd = alloc::boxed::Box::into_raw(alloc::boxed::Box::new(sex_kernel::capability::ProtectionDomain::new(0, 0)));
    DOMAIN_REGISTRY.insert(0, root_pd);

    let _sext = create_protection_domain("/servers/sext/bin/sext\0", None).expect("sext lost");
    
    // 7. Advanced Interaction Suite (Capability Engine)
    sex_kernel::bootstrap_advanced_services();

    let _sexinput = create_protection_domain("/servers/sexinput/bin/sexinput\0", None).expect("sexinput lost");
    let _sexnet = create_protection_domain("/servers/sexnet/bin/sexnet\0", None).expect("sexnet lost");

    // 8. Yield to Scheduler (BSP context)
    if let Some((_, next_ctx)) = sex_kernel::scheduler::SCHEDULERS[0].tick() {
        unsafe {
            sex_kernel::scheduler::Scheduler::switch_to(core::ptr::null_mut(), next_ctx);
        }
    } else {
        panic!("Main loop failed to enqueue scheduler!");
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
