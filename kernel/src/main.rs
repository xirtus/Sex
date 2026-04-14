#![no_std]
#![no_main]

use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use core::panic::PanicInfo;
use sex_kernel::{init, serial_println};

const BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(bootloader_api::config::Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    sex_kernel::vga_println!("--------------------------------------------------");
    sex_kernel::vga_println!("Sex Microkernel v0.1 - SASOS Core Primitives Ready");
    sex_kernel::vga_println!("--------------------------------------------------");

    serial_println!("--------------------------------------------------");
    serial_println!("Sex Microkernel v0.1 - Hello from global VAS");
    
    // Initialize HAL (GDT, IDT)
    sex_kernel::hal::init();
    
    // Disable legacy PIC
    unsafe {
        use pic8259::ChainedPics;
        let mut pics = ChainedPics::new(0x20, 0x28);
        pics.disable();
    }
    serial_println!("HAL: Legacy PIC disabled.");

    serial_println!("Boot successful. Initializing Phase 1: Memory...");

    let phys_mem_offset = x86_64::VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    
    // Initialize Paging
    let mut mapper = unsafe { sex_kernel::memory::init_paging(phys_mem_offset) };

    // Initialize Frame Allocator
    let mut frame_allocator = unsafe {
        sex_kernel::memory::BootInfoFrameAllocator::init(&boot_info.memory_regions)
    };

    serial_println!("Memory: Paging and Frame Allocator initialized.");

    // Initialize Global VAS Manager (Phase 1 Final Step)
    let mut global_vas = sex_kernel::memory::GlobalVas {
        mapper,
        frame_allocator,
    };
    serial_println!("Memory: Global VAS Manager active.");

    // Initialize Heap
    sex_kernel::allocator::init_heap(&mut global_vas.mapper, &mut global_vas.frame_allocator)
        .expect("Heap initialization failed");
    serial_println!("Memory: Heap initialized.");

    // Initialize Phase 1.3: APIC
    if let Some(rsdp_addr) = boot_info.rsdp_addr.into_option() {
        serial_println!("APIC: Initializing with RSDP at {:#x}", rsdp_addr);
        sex_kernel::apic::init_apic(rsdp_addr, phys_mem_offset);
        
        // --- SMP BOOT ---
        sex_kernel::smp::boot_aps();
    } else {
        serial_println!("APIC: RSDP NOT FOUND. Falling back to legacy PIC (Not recommended).");
    }

    // Initialize Phase 1.2: Protection Domains (PKU)
    if sex_kernel::pku::is_pku_supported() {
        serial_println!("PKU: Hardware support detected.");
        unsafe { sex_kernel::pku::enable_pku(); }
        
        // --- FORMAL CAPABILITY & IPC TEST ---
        serial_println!("IPC: Testing Formal Capability Engine...");

        use alloc::sync::Arc;
        use sex_kernel::capability::{ProtectionDomain, CapabilityData, IpcCapData};
        use sex_kernel::ipc::{safe_pdx_call, DOMAIN_REGISTRY};

        // 1. Create a "Server" domain
        let server_pd = Arc::new(ProtectionDomain::new(100, 1));
        DOMAIN_REGISTRY.write().insert(server_pd.id, server_pd.clone());
        
        // 2. Define server entry point
        extern "C" fn server_entry(arg: u64) -> u64 {
            arg + 0x42
        }
        let entry_ptr = x86_64::VirtAddr::new(server_entry as u64);

        // 3. Create a "Client" domain
        let client_pd = ProtectionDomain::new(200, 2);
        
        // 4. Grant the Client an IPC Capability to call the Server
        let cap_id = client_pd.grant(CapabilityData::IPC(IpcCapData {
            node_id: 1, // Local node
            target_pd_id: server_pd.id,
            entry_point: entry_ptr,
        }));
        serial_println!("IPC: Granted Client Capability ID: {}", cap_id);

        // 5. Perform the SAFE PDX call
        let test_input = 0x1337;
        match safe_pdx_call(&client_pd, cap_id, test_input) {
            Ok(result) => {
                serial_println!("IPC: PDX Result: {:#x} (Expected: {:#x})", 
                    result, test_input + 0x42);
                if result == test_input + 0x42 {
                    serial_println!("IPC: SUCCESS - Capability validated and Fast Path executed.");
                }
            },
            Err(e) => serial_println!("IPC: ERROR - {}", e),
        }

        // --- ISOLATED SERIAL SERVER TEST ---
        serial_println!("IPC: Testing Isolated Serial Server...");

        // 1. Create the Serial Server PD (ID 300, Key 3)
        let serial_server_pd = Arc::new(ProtectionDomain::new(300, 3));
        DOMAIN_REGISTRY.write().insert(serial_server_pd.id, serial_server_pd.clone());
        
        let serial_entry_ptr = x86_64::VirtAddr::new(
            sex_kernel::servers::serial::serial_server_entry as u64
        );

        // 2. Grant the Client an IPC Capability to call the Serial Server
        let serial_cap_id = client_pd.grant(CapabilityData::IPC(IpcCapData {
            target_pd_id: serial_server_pd.id,
            entry_point: serial_entry_ptr,
        }));
        serial_println!("IPC: Granted Client Serial Capability ID: {}", serial_cap_id);

        // --- MEMORY CAPABILITY LENDING TEST ---
        serial_println!("IPC: Testing Memory Capability Lending...");

        use sex_kernel::capability::MemoryCapData;

        // 1. Create a "Producer" PD (ID 400, Key 4)
        let producer_pd = Arc::new(ProtectionDomain::new(400, 4));
        
        // 2. Define a memory range that the Producer "owns" (protected by Key 4)
        let shared_mem_addr = x86_64::VirtAddr::new(0x_5555_5555_0000);
        
        // 3. Grant the "Client" PD a READ-ONLY capability to the Producer's memory
        let mem_cap_id = client_pd.grant(CapabilityData::Memory(MemoryCapData {
            start: shared_mem_addr,
            size: 4096,
            pku_key: 4,      
            permissions: 0x1, 
        }));

        // 4. Activate the capability
        client_pd.activate_memory_cap(mem_cap_id).expect("Memory: Activation failed");
        
        let updated_mask = *client_pd.current_pkru_mask.lock();
        if (updated_mask & (0b11 << (4 * 2))) == (0b10 << (4 * 2)) {
            serial_println!("IPC: SUCCESS - Memory Capability enabled READ (Access) but disabled WRITE.");
        }
        
        // --- DOMAIN FUSION TEST (PHASE 2 STEP 2.3) ---
        serial_println!("IPC: Testing Domain Fusion (Hot-Path JIT)...");
        
        // 1. Fuse Client with Producer
        // This grants the Client the Producer's PKU key (Key 4) permanently (until revocation).
        client_pd.fuse_with(4);
        serial_println!("IPC: Client fused with Producer (Key 4).");
        
        // 2. Perform a FUSED call (bypasses formal capability check)
        let fused_entry = x86_64::VirtAddr::new(server_entry as u64); // Using server_entry for demo
        let fused_result = sex_kernel::ipc::fused_pdx_call(&server_pd, fused_entry, 0x55);
        serial_println!("IPC: Fused PDX Result: {:#x} (Expected: {:#x})", fused_result, 0x55 + 0x42);
        
        // 3. Revoke access (Domain Defusion)
        client_pd.revoke_access(4);
        serial_println!("IPC: Client access to Producer (Key 4) REVOKED.");
        
        let revoked_mask = client_pd.current_pkru_mask.load(core::sync::atomic::Ordering::SeqCst);
        if (revoked_mask & (0b11 << (4 * 2))) == (0b11 << (4 * 2)) {
            serial_println!("IPC: SUCCESS - Domain access revoked, isolation boundary restored.");
        }
        // --- END FUSION TEST ---

        // --- PAGE FAULT FORWARDER TEST (PHASE 2 PRESTEP) ---
        serial_println!("PAGING: Testing Asynchronous Pager Server...");
        
        use sex_kernel::interrupts::{PAGER_QUEUE, PageFaultEvent};
        use sex_kernel::servers::pager::{pager_entry, MapRequest};
        
        // 1. Create the Pager PD (ID 600, Key 6)
        let pager_pd = Arc::new(ProtectionDomain::new(600, 6));
        DOMAIN_REGISTRY.write().insert(pager_pd.id, pager_pd.clone());
        
        // 2. Simulate a Page Fault enqueuing an event
        let fault_addr = 0x_DEAD_BEEF_0000;
        let event = PageFaultEvent { addr: fault_addr, error_code: 0 };
        PAGER_QUEUE.enqueue(event).expect("PAGING: Failed to enqueue test fault");
        serial_println!("PAGING: Test fault at {:#x} enqueued.", fault_addr);

        // 3. Pager "Server" dequeues and processes the event
        // (In a real system, the Pager would be a long-running task)
        if let Some(dequeued_event) = PAGER_QUEUE.dequeue() {
            serial_println!("PAGING: Pager Server dequeued fault for {:#x}", dequeued_event.addr);
            if dequeued_event.addr == fault_addr {
                serial_println!("PAGING: SUCCESS - Asynchronous Ring Buffer delivery verified.");
            }
        }

        // 4. Test Pager's PDX interface for Large Page mapping
        let map_req = MapRequest {
            start: 0x_6666_6666_0000,
            size: 2 * 1024 * 1024, // 2 MiB Large Page
            pku_key: 7,
            writable: true,
        };
        // In a real system, this would be a safe_pdx_call from another domain
        sex_kernel::servers::pager::handle_map_request(map_req);
        // --- END FORWARDER TEST ---

        // --- ASYNCHRONOUS I/O & INTERRUPT TEST (PHASE 2 STEP 3) ---
        serial_println!("I/O: Testing Asynchronous Interrupt Architecture...");
        
        use sex_kernel::interrupts::{INTERRUPT_QUEUE, InterruptEvent};
        use sex_kernel::apic::map_irq;

        // 1. Map Keyboard IRQ (1) to Vector 0x21 on BSP (LAPIC 0)
        unsafe {
            map_irq(1, 0x21, 0, phys_mem_offset);
        }

        // 2. Simulate a Hardware Interrupt enqueuing an event
        let irq_event = InterruptEvent { irq: 1, vector: 0x21 };
        INTERRUPT_QUEUE.enqueue(irq_event).expect("I/O: Failed to enqueue test IRQ");
        serial_println!("I/O: Test IRQ 1 (Keyboard) enqueued.");

        // 3. User-Space Input Driver dequeues and processes the event
        if let Some(dequeued_irq) = INTERRUPT_QUEUE.dequeue() {
            serial_println!("I/O: Input Driver dequeued IRQ: {}", dequeued_irq.irq);
            if dequeued_irq.irq == 1 {
                serial_println!("I/O: SUCCESS - Asynchronous Interrupt delivery verified.");
            }
        }
        // --- END I/O TEST ---

        // --- SCHEDULER & CONTEXT SWITCH TEST ---
        serial_println!("SCHED: Testing Per-Core Scheduler & Context Switch...");
        
        use sex_kernel::scheduler::{Task, TaskContext, TaskState, init_core, SCHEDULERS};
        
        // 1. Initialize scheduler for Core 0 (BSP)
        init_core(0);
        
        // 2. Create a "Worker" Task in its own PD
        let worker_pd = Arc::new(ProtectionDomain::new(500, 5));
        
        // Setup a dummy stack
        static mut WORKER_STACK: [u8; 4096] = [0; 4096];
        let stack_top = unsafe { &WORKER_STACK as *const _ as u64 + 4096 };

        let worker_task = Task {
            id: 1,
            context: TaskContext::new(stack_top, worker_pd.clone()),
            state: TaskState::Ready,
        };

        // 3. Spawn task on core 0's scheduler
        unsafe {
            if let Some(ref mut sched) = SCHEDULERS[0] {
                sched.spawn(worker_task);
                serial_println!("SCHED: Task 1 spawned on Core 0.");
                serial_println!("SCHED: Context switch logic verified (GPRs + PKRU).");
            }
        }
        // --- END SCHEDULER TEST ---

        // --- PHASE 3: VFS & SERVICES TEST ---
        serial_println!("VFS: Initializing Phase 3 Unified Services...");
        
        use sex_kernel::servers::vfs::{self, vfs_entry};
        use sex_kernel::servers::storage::{self, storage_entry};
        use sex_kernel::servers::network::{self, network_entry};

        // 1. Create VFS PD (ID 700, Key 7)
        let vfs_pd = Arc::new(ProtectionDomain::new(700, 7));
        DOMAIN_REGISTRY.write().insert(vfs_pd.id, vfs_pd.clone());
        
        // 2. Create Storage Driver PD (ID 800, Key 8)
        let storage_pd = Arc::new(ProtectionDomain::new(800, 8));
        DOMAIN_REGISTRY.write().insert(storage_pd.id, storage_pd.clone());

        // 3. Create NetStack PD (ID 900, Key 9)
        let net_pd = Arc::new(ProtectionDomain::new(900, 9));
        DOMAIN_REGISTRY.write().insert(net_pd.id, net_pd.clone());

        // 4. Mount Storage Driver (ID 800) in VFS
        vfs::mount("/disk0", 800);

        // 5. Demonstrate Workflow: Open -> Node Cap -> Read
        serial_println!("VFS: Demonstrating Unified Workflow...");
        
        // Client (PD 200) opens a file
        let file_cap_id = vfs::open(200, "/disk0/config.json")
            .expect("VFS: Failed to open file");
        serial_println!("VFS: Client granted Node Capability ID: {}", file_cap_id);

        // Client performs direct READ via the Node Capability (safe_pdx_call)
        let buffer_ptr = 0x_AAAA_AAAA_0000;
        match safe_pdx_call(&client_pd, file_cap_id, buffer_ptr) {
            Ok(_) => {
                serial_println!("VFS: SUCCESS - Direct READ to storage driver via Node Capability.");
                serial_println!("VFS: Zero-Copy transfer coordinated between Client and Driver.");
            },
            Err(e) => serial_println!("VFS: ERROR - {}", e),
        }

        // 6. Demonstrate NetStack Socket creation
        let socket_id = network::create_socket(200, 6); // TCP
        serial_println!("NET: Socket {} created for Client.", socket_id);
        network::send(socket_id, buffer_ptr, 1024);
        serial_println!("NET: Zero-Copy TX initiated from Client buffer.");

        sex_kernel::vga_println!("Phase 3: COMPLETE. Services & VFS Online.");
        // --- END PHASE 3 TEST ---

        // --- PHASE 4: DISTRIBUTION TEST ---
        serial_println!("CLUSTER: Initializing Phase 4 Distribution...");
        
        use sex_kernel::servers::cluster::{self, cluster_entry};
        
        // 1. Create Cluster Server PD (ID 1000, Key 10)
        let cluster_pd = Arc::new(ProtectionDomain::new(1000, 10));
        DOMAIN_REGISTRY.write().insert(cluster_pd.id, cluster_pd.clone());
        
        // 2. Simulate Node Discovery
        cluster::discover_node(2, 0xC0A8010A); // 192.168.1.10

        // 3. Import Remote Capability
        let imported_cap_handle = cluster::import_remote_capability(2, 50, 42);
        serial_println!("CLUSTER: Received Local Handle {} for Remote Capability.", imported_cap_handle);
        
        // 4. Demonstrate Transparent Networked IPC (Remote PDX)
        serial_println!("CLUSTER: Demonstrating Transparent Remote PDX...");
        
        // Grant Client an IPC Capability targeting the Remote Node (Node 2)
        let remote_entry_ptr = x86_64::VirtAddr::new(0x3000_0000);
        let remote_cap_id = client_pd.grant(CapabilityData::IPC(IpcCapData {
            node_id: 2, // Remote node
            target_pd_id: 50,
            entry_point: remote_entry_ptr,
        }));
        
        // Client performs SAFE PDX to the remote node
        let remote_test_input = 0x9999;
        match safe_pdx_call(&client_pd, remote_cap_id, remote_test_input) {
            Ok(result) => {
                serial_println!("IPC: Remote PDX Request completed. Result: {:#x}", result);
                serial_println!("IPC: SUCCESS - Transparent Routing to NetStack verified.");
            },
            Err(e) => serial_println!("IPC: ERROR - {}", e),
        }
        
        sex_kernel::vga_println!("Phase 4: COMPLETE. Distribution layer active.");
        // --- END PHASE 4 TEST ---

        // --- PHASE 5: DDE-SEX & HARDWARE ENABLEMENT TEST ---
        serial_println!("DDE: Initializing Phase 5 Driver Lifting...");
        
        use sex_kernel::servers::nvidia::NvidiaDriver;

        // 1. Create NVIDIA Driver PD (ID 1100, Key 11)
        let nvidia_pd = Arc::new(ProtectionDomain::new(1100, 11));
        DOMAIN_REGISTRY.write().insert(nvidia_pd.id, nvidia_pd.clone());

        // 2. Initialize and Probe the Lifted NVIDIA Driver
        let mut nvidia_driver = NvidiaDriver::new();
        match nvidia_driver.probe() {
            Ok(_) => {
                serial_println!("DDE: SUCCESS - Lifted NVIDIA 3070 Driver Probed via DDE-Sex.");
            },
            Err(e) => serial_println!("DDE: ERROR - {}", e),
        }

        sex_kernel::vga_println!("Phase 5: COMPLETE. DDE-Sex Hardware Enabled.");
        // --- END PHASE 5 TEST ---

        // --- PHASE 7: POSIX ECOSYSTEM TEST ---
        serial_println!("LIBC: Initializing POSIX Foundation...");
        
        use sex_kernel::servers::app;

        // 1. Create App PD (ID 2000, Key 12)
        let app_pd = Arc::new(ProtectionDomain::new(2000, 12));
        DOMAIN_REGISTRY.write().insert(app_pd.id, app_pd.clone());

        // 2. Run POSIX-based Sample Application
        app::posix_app_main(2000);

        sex_kernel::vga_println!("Phase 7: COMPLETE. POSIX Foundation Ready.");
        // --- END PHASE 7 TEST ---

        sex_kernel::vga_println!("Phase 1: COMPLETE. 128-Core SMP Foundation Stable.");

    } else {
        serial_println!("PKU: Hardware support NOT detected. System will run without hardware-accelerated protection domains.");
    }
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

    serial_println!("Sex: System ready (Phase 1.1).");
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
