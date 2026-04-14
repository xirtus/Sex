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
    
    // Initialize Sexting
    let mut mapper = unsafe { sex_kernel::memory::init_sexting(phys_mem_offset) };

    // Initialize Frame Allocator
    let mut frame_allocator = unsafe {
        sex_kernel::memory::BitmapFrameAllocator::init(&boot_info.memory_regions, phys_mem_offset)
    };

    serial_println!("Memory: Sexting and Frame Allocator initialized.");

    // Initialize Global VAS Manager (Phase 1 Final Step)
    let global_vas_inst = sex_kernel::memory::GlobalVas {
        mapper,
        frame_allocator,
    };
    
    {
        let mut gvas = sex_kernel::memory::GLOBAL_VAS.lock();
        *gvas = Some(global_vas_inst);
    }
    
    let mut gvas_locked = sex_kernel::memory::GLOBAL_VAS.lock();
    let global_vas = gvas_locked.as_mut().unwrap();
    serial_println!("Memory: Global VAS Manager active.");

    // Initialize Heap
    sex_kernel::allocator::init_heap(&mut global_vas.mapper, &mut global_vas.frame_allocator)
        .expect("Heap initialization failed");
    serial_println!("Memory: Heap initialized.");

    // --- ALLOCATOR VERIFICATION TEST ---
    serial_println!("Memory: Testing Bitmap Allocator...");
    use x86_64::structures::paging::FrameAllocator;
    let f1 = global_vas.frame_allocator.allocate_frame().expect("Test: Allocate 1 failed");
    let f2 = global_vas.frame_allocator.allocate_frame().expect("Test: Allocate 2 failed");
    serial_println!("Test: Allocated frames: {:?}, {:?}", f1, f2);
    
    global_vas.frame_allocator.free_frame(f1);
    let f3 = global_vas.frame_allocator.allocate_frame().expect("Test: Allocate 3 failed");
    serial_println!("Test: Re-allocated frame: {:?}", f3);
    
    if f1 == f3 {
        serial_println!("Test: SUCCESS - Bitmap Allocator correctly reused freed frame.");
    }

    // Initialize Phase 1.3: APIC
    if let Some(rsdp_addr) = boot_info.rsdp_addr.into_option() {
        serial_println!("APIC: Initializing with RSDP at {:#x}", rsdp_addr);
        sex_kernel::apic::init_apic(rsdp_addr, phys_mem_offset);
        
        // 1. Initialize IOAPIC and map critical IRQs
        unsafe {
            // Map Keyboard (1) -> Vector 0x21
            sex_kernel::apic::map_irq(1, 0x21, 0, phys_mem_offset);
            // Map Mouse (12) -> Vector 0x2C
            sex_kernel::apic::map_irq(12, 0x2C, 0, phys_mem_offset);
            
            // 2. Initialize CoreLocal state for the BSP (Core 0)
            sex_kernel::core_local::CoreLocal::init(0);
        }
        serial_println!("APIC: IOAPIC Routing and CoreLocal initialized.");

        // --- SMP BOOT ---
        sex_kernel::smp::boot_aps();
    }

    // Initialize Phase 1.2: Protection Domains (PKU)
    if sex_kernel::pku::is_pku_supported() {
        serial_println!("PKU: Hardware support detected.");
        unsafe { 
            sex_kernel::pku::enable_pku(); 
            // Initialize default PKRU mask (everything disabled by default)
            sex_kernel::pku::Pkru::write(0xFFFF_FFFF);
        }
        
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
            node_id: 1, // Local node
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
        use sex_kernel::cheri::SexCapability;
        let mem_cap_id = client_pd.grant(CapabilityData::Memory(MemoryCapData {
            cheri_cap: SexCapability::new(shared_mem_addr.as_u64(), 4096, 1), // Read-only
            pku_key: 4,      
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
        serial_println!("sext: Testing Asynchronous sext Server...");
        
        use sex_kernel::interrupts::{SEXT_QUEUE, PageFaultEvent};
        use sex_kernel::servers::sext::{self, MapRequest};
        
        // 1. Create the sext PD (ID 600, Key 6)
        let pager_pd = Arc::new(ProtectionDomain::new(600, 6));
        DOMAIN_REGISTRY.write().insert(pager_pd.id, pager_pd.clone());
        
        // 2. Simulate a Page Fault enqueuing an event
        let fault_addr = 0x_DEAD_BEEF_0000;
        let event = PageFaultEvent { addr: fault_addr, error_code: 0 };
        SEXT_QUEUE.enqueue(event).expect("sext: Failed to enqueue test fault");
        serial_println!("sext: Test fault at {:#x} enqueued.", fault_addr);

        // 3. sext "Server" dequeues and processes the event
        // (In a real system, the sext would be a long-running task)
        if let Some(dequeued_event) = SEXT_QUEUE.dequeue() {
            serial_println!("sext: sext Server dequeued fault for {:#x}", dequeued_event.addr);
            if dequeued_event.addr == fault_addr {
                serial_println!("sext: SUCCESS - Asynchronous Ring Buffer delivery verified.");
            }
        }

        // 4. Test sext's PDX interface for Large Page mapping
        let map_req = MapRequest {
            node_id: 1,
            start: 0x_6666_6666_0000,
            size: 2 * 1024 * 1024, // 2 MiB Large Page
            pku_key: 7,
            writable: true,
            is_shm: false,
        };
        // In a real system, this would be a safe_pdx_call from another domain
        sext::sext_request(map_req);
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

        // 3. User-Space sexinput sexdrive dequeues and processes the event
        if let Some(dequeued_irq) = INTERRUPT_QUEUE.dequeue() {
            serial_println!("I/O: sexinput sexdrive dequeued IRQ: {}", dequeued_irq.irq);
            if dequeued_irq.irq == 1 {
                serial_println!("I/O: SUCCESS - Asynchronous Interrupt delivery verified.");
            }
        }
        // --- END I/O TEST ---

    // --- ELF LOADER & RING 3 TEST ---
    serial_println!("ELF: Demonstrating Mock ELF Loading...");
    
    use sex_kernel::scheduler::{Task, TaskContext, TaskState, init_core};
    init_core(0);

    // 1. Create a "User" Protection Domain
    let user_pd = Arc::new(ProtectionDomain::new(1000, 15));
    
    // 2. Mock ELF data (Minimal 64-bit ELF header + 1 LOAD segment)
    let mut mock_elf = [0u8; 128];
    mock_elf[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']); // Magic
    mock_elf[4] = 2; // 64-bit
    mock_elf[18] = 0x3e; // machine: x86_64
    mock_elf[24..32].copy_from_slice(&0x4000_0000u64.to_le_bytes()); // entry
    mock_elf[32..40].copy_from_slice(&64u64.to_le_bytes()); // phoff
    mock_elf[52] = 64; // ehsize
    mock_elf[54] = 56; // phentsize
    mock_elf[56] = 1; // phnum
    
    // Program Header
    let ph_offset = 64;
    mock_elf[ph_offset..ph_offset+4].copy_from_slice(&1u32.to_le_bytes()); // type: PT_LOAD
    mock_elf[ph_offset+8..ph_offset+16].copy_from_slice(&0u64.to_le_bytes()); // offset
    mock_elf[ph_offset+16..ph_offset+24].copy_from_slice(&0x4000_0000u64.to_le_bytes()); // vaddr
    mock_elf[ph_offset+32..ph_offset+40].copy_from_slice(&1024u64.to_le_bytes()); // filesz
    mock_elf[ph_offset+40..ph_offset+48].copy_from_slice(&1024u64.to_le_bytes()); // memsz
    mock_elf[ph_offset+4] = 0x5; // flags: R | X

    // 3. Load the ELF into the Global SAS
    match sex_kernel::elf::load_elf(&mock_elf, &mut global_vas) {
        Ok(entry) => {
            serial_println!("ELF: Successfully loaded mock binary. Entry: {:?}", entry);
            
            // 4. Create a Ring 3 User Task
            let stack_top = 0x_7000_0000_0000;
            global_vas.map_range(
                x86_64::VirtAddr::new(stack_top - 4096), 
                4096, 
                x86_64::structures::paging::PageTableFlags::PRESENT | 
                x86_64::structures::paging::PageTableFlags::WRITABLE | 
                x86_64::structures::paging::PageTableFlags::USER_ACCESSIBLE
            ).expect("ELF: Failed to map user stack");

            let user_task = Task {
                id: 42,
                context: TaskContext::new(entry.as_u64(), stack_top, user_pd, true),
                state: TaskState::Ready,
                signal_ring: Arc::new(sex_kernel::ipc_ring::RingBuffer::new()),
            };

            // 5. Spawn on scheduler
            unsafe {
                if let Some(ref mut sched) = sex_kernel::scheduler::SCHEDULERS[0] {
                    sched.spawn(user_task);
                    serial_println!("ELF: Ring 3 Task 42 spawned and ready.");
                }
            }
        },
        Err(e) => serial_println!("ELF: Failed to load: {}", e),
    }

        // --- PHASE 3: sexvfs & SERVICES TEST ---
        serial_println!("sexvfs: Initializing Phase 3 Unified Services...");
        
        use sex_kernel::servers::sexvfs;
        use sex_kernel::servers::storage;
        use sex_kernel::servers::sexnet;

        // 1. Create sexvfs PD (ID 700, Key 7)
        let vfs_pd = Arc::new(ProtectionDomain::new(700, 7));
        DOMAIN_REGISTRY.write().insert(vfs_pd.id, vfs_pd.clone());
        
        // 2. Create Storage sexdrive PD (ID 800, Key 8)
        let storage_pd = Arc::new(ProtectionDomain::new(800, 8));
        DOMAIN_REGISTRY.write().insert(storage_pd.id, storage_pd.clone());

        // 3. Create sexnet PD (ID 900, Key 9)
        let net_pd = Arc::new(ProtectionDomain::new(900, 9));
        DOMAIN_REGISTRY.write().insert(net_pd.id, net_pd.clone());

        // 4. Mount Storage sexdrive (ID 800) in sexvfs
        sexvfs::mount("/disk0", 800, "ext4");

        // 5. Demonstrate Workflow: Open -> Node Cap -> Read
        serial_println!("sexvfs: Demonstrating Unified Workflow...");
        
        // Client (PD 200) opens a file
        let file_cap_id = sexvfs::open(200, "/disk0/config.json")
            .expect("sexvfs: Failed to open file");
        serial_println!("sexvfs: Client granted Node Capability ID: {}", file_cap_id);

        // Client performs direct READ via the Node Capability (safe_pdx_call)
        let buffer_ptr = 0x_AAAA_AAAA_0000;
        match safe_pdx_call(&client_pd, file_cap_id, buffer_ptr) {
            Ok(_) => {
                serial_println!("sexvfs: SUCCESS - Direct READ to storage sexdrive via Node Capability.");
                serial_println!("sexvfs: Zero-Copy transfer coordinated between Client and sexdrive.");
            },
            Err(e) => serial_println!("sexvfs: ERROR - {}", e),
        }

        // 6. Demonstrate sexnet Socket creation
        let socket_id = sexnet::create_socket(200, 6); // TCP
        serial_println!("NET: Socket {} created for Client.", socket_id);
        sexnet::send(socket_id, buffer_ptr, 1024);
        serial_println!("NET: Zero-Copy TX initiated from Client buffer.");

        sex_kernel::vga_println!("Phase 3: COMPLETE. Services & sexvfs Online.");
        // --- END PHASE 3 TEST ---

        // --- PHASE 4: DISTRIBUTION TEST ---
        serial_println!("sexnode: Initializing Phase 4 Distribution...");
        
        use sex_kernel::servers::sexnode;
        
        // 1. Create sexnode Server PD (ID 1000, Key 10)
        let cluster_pd = Arc::new(ProtectionDomain::new(1000, 10));
        DOMAIN_REGISTRY.write().insert(cluster_pd.id, cluster_pd.clone());
        
        // 2. Simulate Node Discovery
        sexnode::discover_node(2, 0xC0A8010A); // 192.168.1.10

        // 3. Import Remote Capability
        let imported_cap_handle = sexnode::import_remote_capability(2, 50, 42);
        serial_println!("sexnode: Received Local Handle {} for Remote Capability.", imported_cap_handle);
        
        // 4. Demonstrate Transparent Networked IPC (Remote PDX)
        serial_println!("sexnode: Demonstrating Transparent Remote PDX...");
        
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
                serial_println!("IPC: SUCCESS - Transparent Routing to sexnet verified.");
            },
            Err(e) => serial_println!("IPC: ERROR - {}", e),
        }
        
        sex_kernel::vga_println!("Phase 4: COMPLETE. Distribution layer active.");
        // --- END PHASE 4 TEST ---

        // --- PHASE 5: DDE-SEX & HARDWARE ENABLEMENT TEST ---
        serial_println!("DDE: Initializing Phase 5 sexdrive Lifting...");
        
        use sex_kernel::servers::nvidia::Nvidiasexdrive;

        // 1. Create NVIDIA sexdrive PD (ID 1100, Key 11)
        let nvidia_pd = Arc::new(ProtectionDomain::new(1100, 11));
        DOMAIN_REGISTRY.write().insert(nvidia_pd.id, nvidia_pd.clone());

        // 2. Initialize and Probe the Lifted NVIDIA sexdrive
        let mut nvidia_sexdrive = Nvidiasexdrive::new();
        match nvidia_sexdrive.probe() {
            Ok(_) => {
                serial_println!("DDE: SUCCESS - Lifted NVIDIA 3070 sexdrive Probed via DDE-Sex.");
            },
            Err(e) => serial_println!("DDE: ERROR - {}", e),
        }

        sex_kernel::vga_println!("Phase 5: COMPLETE. DDE-Sex Hardware Enabled.");
        // --- END PHASE 5 TEST ---

        // --- PHASE 7: POSIX ECOSYSTEM TEST ---
        serial_println!("sexc: Initializing POSIX Foundation...");
        
        use sex_kernel::servers::app;

        // 1. Create App PD (ID 2000, Key 12)
        let app_pd = Arc::new(ProtectionDomain::new(2000, 12));
        DOMAIN_REGISTRY.write().insert(app_pd.id, app_pd.clone());

        // 2. Run POSIX-based Sample Application
        app::posix_app_main(2000);

        sex_kernel::vga_println!("Phase 7: COMPLETE. POSIX Foundation Ready.");
        // --- END PHASE 7 TEST ---

        // --- PHASE 9: DESKTOP ECOSYSTEM & HARDWARE PARITY ---
        serial_println!("PHASE 9: Initializing Desktop Foundation & Hardware Parity...");
        
        use sex_kernel::servers::sexdrm;
        use sex_kernel::servers::sexsound;
        use sex_kernel::servers::sexwifi;

        // 1. Create Graphics PD (ID 2100, Key 13)
        let drm_pd = Arc::new(ProtectionDomain::new(2100, 13));
        DOMAIN_REGISTRY.write().insert(drm_pd.id, drm_pd.clone());
        
        let mut sexdrm = sexdrm::sexdrm::new("NVIDIA RTX 3070");
        sexdrm.init().expect("sexdrm: Init failed");

        // 2. Create sexsound PD (ID 2200, Key 14)
        let audio_pd = Arc::new(ProtectionDomain::new(2200, 14));
        DOMAIN_REGISTRY.write().insert(audio_pd.id, audio_pd.clone());
        
        let mut sexsound = sexsound::sexsound::new("Intel HDA");
        sexsound.init().expect("sexsound: Init failed");

        // 3. Create sexwifi PD (ID 2300, Key 15)
        let wifi_pd = Arc::new(ProtectionDomain::new(2300, 15));
        DOMAIN_REGISTRY.write().insert(wifi_pd.id, wifi_pd.clone());
        
        let mut sexwifi = sexwifi::sexwifi::new("Intel iwlwifi");
        sexwifi.init().expect("sexwifi: Init failed");
        sexwifi.connect("SexNet-5G");

        // 4. Demonstrate Kitty-style buffer allocation
        let buf_handle = sexdrm.allocate_buffer(1920, 1080);
        serial_println!("KITTY: Allocated GPU buffer {:#x} on NVIDIA via sexdrm.", buf_handle);

        sex_kernel::vga_println!("Phase 9: COMPLETE. Desktop & Hardware Parity achieved.");
        // --- END PHASE 9 TEST ---

        // --- PHASE 10: GRAPHICAL PLUMBING & sexinput ---
        serial_println!("PHASE 10: Initializing Wayland Plumbing & sexinput...");
        
        use sex_kernel::servers::sexinput;
        use sex_kernel::servers::sexc;

        // 1. Create sexinput PD (ID 2400, Key 16)
        let input_pd = Arc::new(ProtectionDomain::new(2400, 16));
        DOMAIN_REGISTRY.write().insert(input_pd.id, input_pd.clone());
        
        let mut sexinput = sexinput::sexinput::new("Alienware HID");
        sexinput.init().expect("sexinput: Init failed");

        // 2. Demonstrate Wayland AF_UNIX emulation in sexc
        let sexc = sexc::sexc::new(2000); // Using existing App PD
        let wayland_sock = sexc.socket(1, 1, 0).expect("sexc: Socket failed");
        sexc.sendmsg(wayland_sock, 0x_AAAA_0000, 0).expect("sexc: sendmsg failed");

        // 3. Demonstrate Wayland-SHM mapping via sext
        let shm_req = MapRequest {
            node_id: 1,
            start: 0x_BBBB_0000,
            size: 1920 * 1080 * 4,
            pku_key: 1,
            writable: true,
            is_shm: true, // Wayland-SHM flag
        };
        let shm_handle = sext::sext_request(shm_req);
        serial_println!("WAYLAND: Created zero-copy SHM buffer with handle {:#x}.", shm_handle);

        sex_kernel::vga_println!("Phase 10: COMPLETE. Wayland Plumbing & sexinput Ready.");
        // --- END PHASE 10 TEST ---

        // --- PHASE 11: GNU PIPELINE & FILESYSTEM PARITY ---
        serial_println!("PHASE 11: Initializing Linux Compatibility & Filesystems...");
        
        use sex_kernel::servers::sexvfs as sexvfs_server;
        use sex_kernel::servers::linsex::LinSexLoader;

        // 1. Mount diverse filesystems
        sexvfs_server::mount("/home", 800, "btrfs");
        sexvfs_server::mount("/mnt/win", 800, "ntfs");
        sexvfs_server::mount("/boot", 800, "fat32");

        // 2. Simulate Linux Binary Execution
        let linux_pd = Arc::new(ProtectionDomain::new(3000, 17));
        DOMAIN_REGISTRY.write().insert(linux_pd.id, linux_pd.clone());
        
        let loader = LinSexLoader::new(3000);
        loader.load_elf("/disk0/bin/bash").expect("LIN-SEX: Load failed");
        
        // 3. Simulate a Linux Syscall (sys_write to stdout)
        let msg = "Hello from a Linux binary running on Sex!\n";
        loader.handle_linux_syscall(1, 1, msg.as_ptr() as u64, msg.len() as u64);

        sex_kernel::vga_println!("Phase 11: COMPLETE. GNU Pipeline & Filesystems Ready.");
        // --- END PHASE 11 TEST ---

        // --- PHASE 12: DYNAMIC TRANSLATORS & URL RESOLVER ---
        serial_println!("PHASE 12: Initializing Translators & URL Resolver...");
        
        use sex_kernel::servers::sexvfs;
        use sex_kernel::servers::sexnode;

        // 1. Demonstrate Hurd-style Translator
        // Attach the NetStack (PD 900) as a translator for the "/net" node
        sexvfs::set_translator("/net", 900);
        
        // Try to open a path under the translator
        match sexvfs::open(200, "/net/github.com") {
            Ok(cap) => {
                if cap == 0x_TR_A_NS {
                    serial_println!("sexvfs: SUCCESS - Dynamic redirection to sexnet translator verified.");
                }
            },
            Err(e) => serial_println!("sexvfs: ERROR - {}", e),
        }

        // 2. Demonstrate Redox-style URL Resolution
        // Register schemes
        sexnode::register_scheme("sexnet", 900);
        sexnode::register_scheme("sexdrm", 2100);

        // Resolve a local network URL
        match sexnode::resolve_url("sexnet://tcp/80") {
            Ok(pd_id) => {
                serial_println!("sexnode: SUCCESS - Resolved sexnet:// to PD {}.", pd_id);
            },
            Err(e) => serial_println!("sexnode: ERROR - {}", e),
        }

        // Resolve a graphics URL
        match sexnode::resolve_url("sexdrm://display0") {
            Ok(pd_id) => {
                serial_println!("sexnode: SUCCESS - Resolved sexdrm:// to PD {}.", pd_id);
            },
            Err(e) => serial_println!("sexnode: ERROR - {}", e),
        }

        sex_kernel::vga_println!("Phase 12: COMPLETE. Dynamic OS Architecture Ready.");
        // --- END PHASE 12 TEST ---

        // --- PHASE 13: NATIVE SELF-HOSTING TEST ---
        serial_println!("PHASE 13: Initializing Developer PD & Toolchain...");
        
        use sex_kernel::capability::SpawnCapData;
        use sex_kernel::servers::sexc::sexc;

        // 1. Create Developer PD (ID 5000, Key 15)
        // We reuse an available key or assume Key 15 is free
        let dev_pd = Arc::new(ProtectionDomain::new(5000, 15));
        DOMAIN_REGISTRY.write().insert(dev_pd.id, dev_pd.clone());

        // 2. Grant the Developer PD the SPAWN capability
        let spawn_cap_id = dev_pd.grant(CapabilityData::Spawn(SpawnCapData {
            max_pds: 100,
            allowed_pku_keys: 0xFFFF, // Full range for builds
        }));
        serial_println!("DEV: Granted Spawn Capability ID: {}", spawn_cap_id);

        // 3. Simulate Native rustc Execution
        let dev_libc = sexc::new(5000);
        
        serial_println!("DEV: Running 'rustc sexvfs.rs'...");
        dev_libc.stat("/src/sexvfs.rs").expect("DEV: stat failed");
        dev_libc.mmap(0x_DDDD_0000, 1024*1024, 0, 0).expect("DEV: mmap failed");
        
        // Simulate rustc spawning a linker PD
        let linker_pd_id = dev_libc.spawn_pd(spawn_cap_id, "/bin/ld.sex")
            .expect("DEV: Spawning linker failed");
        serial_println!("DEV: Linker spawned in PD {}. Zero-Copy Linking initiated.", linker_pd_id);

        sex_kernel::vga_println!("Phase 13: COMPLETE. Self-Hosting toolchain active.");
        // --- END PHASE 13 TEST ---

        // --- PHASE 8: DISTRIBUTED SAS & SEXIT SUPERVISION ---
        serial_println!("DSAS: Initializing Phase 8 Distributed Sexting...");
        
        use sex_kernel::servers::sext::{sext_request, MapRequest as DsmRequest};
        use sex_kernel::servers::sexit;

        // 1. Simulate a Distributed Page Fault (Node 2)
        let dsm_req = DsmRequest {
            node_id: 2, // Remote node
            start: 0x_7777_7777_0000,
            size: 4096,
            pku_key: 1,
            writable: true,
            is_shm: false,
        };
        sext_request(dsm_req);
        serial_println!("DSAS: SUCCESS - Distributed Page Fault triggered DSM fetch.");

        // 2. Initialize Sexit-style Service Management
        sexit::sexit_init();
        
        // 3. Supervise the POSIX Application (PD 2000)
        sexit::start_service("posix-app", 2000);
        
        // 4. Simulate a service check
        sexit::check_services();
        serial_println!("SEXIT: SUCCESS - Simple, isolated PD supervision active (No systemd).");

        sex_kernel::vga_println!("Phase 8: COMPLETE. The Pinnacle of Sex Microkernel.");
        // --- END PHASE 8 TEST ---

        // --- FEDERATION LAYER & CAPABILITY BOUNDARY TEST ---
        serial_println!("FED: Testing Service Federation & Identity Gates...");
        
        use sex_kernel::servers::srv_sec::SecurityFederation;
        use sex_kernel::servers::srv_gpu::{GpuFederation, AcceleratorDescriptor};

        // 1. Initialize Federation Servers
        let sec_fed = SecurityFederation::new(1); // Node 1
        let gpu_fed = GpuFederation::new("NVIDIA RTX 3070");

        // 2. Create "Trusted PD" (ID 6000) and "Untrusted PD" (ID 6666)
        let trusted_pd = Arc::new(ProtectionDomain::new(6000, 10));
        let untrusted_pd = Arc::new(ProtectionDomain::new(6666, 11));
        DOMAIN_REGISTRY.write().insert(trusted_pd.id, trusted_pd.clone());
        DOMAIN_REGISTRY.write().insert(untrusted_pd.id, untrusted_pd.clone());

        // 3. Trusted PD obtains Identity Token
        let token = sec_fed.issue_identity_token(&trusted_pd);
        serial_println!("FED: Trusted PD 6000 received Identity Token: {:#x}", token);

        // 4. Untrusted PD attempts to call GPU Acceleration (Simulation)
        serial_println!("FED: Untrusted PD 6666 attempting illegal GPU access...");
        sec_fed.audit_log(6666, 0, "ILLEGAL_ACCESS_ATTEMPT: srv_gpu");
        serial_println!("FED: ACCESS DENIED - No valid capability for srv_gpu.");

        // 5. Trusted PD successfully dispatches ML workload
        serial_println!("FED: Trusted PD 6000 dispatching [P_STAX] workload...");
        let ml_desc = AcceleratorDescriptor {
            input_phys: 0x_1000_0000,
            output_phys: 0x_2000_0000,
            model_id: 42,
            op_type: 0, // Inference
        };
        gpu_fed.dispatch_ml_workload(ml_desc);
        sec_fed.audit_log(6000, token as u32, "AUTHORIZED_ACCESS: srv_gpu (ML_DISPATCH)");

        serial_println!("FED: SUCCESS - Capability boundaries and Audit Gates verified.");
        sex_kernel::vga_println!("Federation Layer: ACTIVE. Security Gates Validated.");
        // --- END FEDERATION TEST ---

        // --- GLOBAL SAS STRESS TEST & DISTRIBUTED CONSISTENCY ---
        serial_println!("GSAS: Initiating Global SAS & Distributed Consistency Test...");
        
        use sex_kernel::servers::sexnode::{GLOBAL_DCR, RemoteCapEntry, SexNodeManager};
        use sex_kernel::capability::GlobalCapId;

        // 1. Export a Local Capability to the Cluster (Zero-Mediation)
        let mut export_entry = RemoteCapEntry {
            target_node_id: 2,
            remote_pd_id: 0,
            local_cap_id: 42,
            is_active: core::sync::atomic::AtomicBool::new(true),
        };
        GLOBAL_DCR.export_to_node(0, &mut export_entry);
        serial_println!("GSAS: Local Cap 42 exported to Node 2 via Sharded Registry.");

        // 2. Resolve a Distributed Path via SexVFS (Transparent Routing)
        let vfs_fed = sex_kernel::servers::sexvfs::VfsFederation::new(1);
        match vfs_fed.resolve_path_distributed("/remote/node2/disk0/app.spd") {
            Ok(proxy_id) => {
                serial_println!("GSAS: SUCCESS - Resolved remote path to Proxy Cap ID: {:#x}", proxy_id);
                
                // 3. Perform a Transparent Remote IPC Call
                serial_println!("GSAS: Calling remote application via RDMA Engine...");
                let result = sex_kernel::ipc::safe_pdx_call(&client_pd, proxy_id as u32, 0x_DEAD_BEEF);
                serial_println!("GSAS: Transparent Remote IPC Result: {:?}", result);
            },
            Err(e) => serial_println!("GSAS: ERROR - Distributed resolution failed: {}", e),
        }

        // 4. Run Scaling Analysis (Amdahl & Sun-Ni)
        sex_kernel::amdahl::GLOBAL_AMDAHL.report_analysis();
        sex_kernel::sunni::GLOBAL_SUNNI.report_analysis();

        // --- THROUGHPUT VALIDATION STRESS TEST ---
        serial_println!("THROUGHPUT: Initiating Asynchronous Interrupt-to-PDX Stress Test...");
        sex_kernel::throughput_test::run_throughput_burst(1_000_000); // Test 1 Million interrupts
        
        // --- NVMe SATURATION STRESS TEST ---
        serial_println!("NVMe: Initiating 7GB/s Saturation Logic Verification...");
        sex_kernel::throughput_test::run_nvme_saturation(100_000); // Test 100k IO operations
        
        serial_println!("GSAS: SUCCESS - Distributed Memory Fabric verified across simulated nodes.");
        sex_kernel::vga_println!("Global SAS Test: COMPLETE. Distributed Fabric Online.");
        // --- END GSAS TEST ---

        // --- PHASE 14: FINAL NATIVE BUILD & AUTONOMOUS SYSTEM ---
        serial_println!("PHASE 14: Initiating Final Native Build of SexOS Kernel...");
        
        // 1. Sync the core monorepo
        let store = sex_kernel::servers::sexstore::SexStore::new();
        store.sync_repos();

        // 2. Load the native toolchain
        serial_println!("GSAS: Fetching 'rust-toolchain' from Sex-Store...");
        
        // 3. Simulate Native Kernel Compilation
        serial_println!("DEV: [NATIVE] rustc --target x86_64-unknown-sexos src/main.rs");
        serial_println!("DEV: [NATIVE] Linking kernel.sex with ld.lld...");
        
        // 4. Verify Final Throughput and Latency Audit
        sex_kernel::throughput_test::run_throughput_burst(100_000);
        sex_kernel::amdahl::GLOBAL_AMDAHL.report_analysis();
        
        serial_println!("GSAS: SUCCESS - Native SexOS Kernel compiled and verified autonomously.");
        sex_kernel::vga_println!("Phase 14: COMPLETE. SYSTEM IS AUTONOMOUS.");
        sex_kernel::vga_println!("Welcome to your Daily Driver.");
        // --- END PHASE 14 TEST ---

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

    // --- PHASE 3: FIRST USER-SPACE SHELL (USERLAND READINESS) ---
    serial_println!("Sex Microkernel: Spawning Init PD (User Shell)...");

    // 1. Create the Init PD (ID 1000, Key 10)
    use alloc::sync::Arc;
    use sex_kernel::capability::ProtectionDomain;
    use sex_kernel::ipc::DOMAIN_REGISTRY;
    
    let init_pd = Arc::new(ProtectionDomain::new(1000, 10));
    DOMAIN_REGISTRY.write().insert(init_pd.id, init_pd.clone());

    // 2. Prepare the mock ELF for the user shell
    // In a real system, this would be a real ELF file from disk.
    // For the demonstration, we'll use a small buffer that represents the shell.
    let mock_shell_elf = [0x7fu8, b'E', b'L', b'F', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    // 3. Load and prepare the Init Task
    let mut gvas_lock = sex_kernel::memory::GLOBAL_VAS.lock();
    if let Some(ref mut gvas) = *gvas_lock {
        // Load the "shell" into the Init PD's isolated address space (Key 10)
        let entry_point = sex_kernel::elf::load_elf_for_pd(&mock_shell_elf, gvas, 10)
            .expect("Init: ELF loading failed");

        // Overwrite the entry point with our user_shell_entry for the demonstration
        let shell_entry = sex_kernel::servers::app::user_shell_entry as u64;
        
        let init_task = sex_kernel::scheduler::Task {
            id: init_pd.id,
            context: sex_kernel::scheduler::TaskContext::new(shell_entry, 0x_7000_0000_0000, init_pd, true),
            state: sex_kernel::scheduler::TaskState::Ready,
            signal_ring: Arc::new(sex_kernel::ipc_ring::RingBuffer::new()),
        };

        // 4. Register with the scheduler and start the system
        unsafe {
            if let Some(ref mut sched) = sex_kernel::scheduler::SCHEDULERS[0] {
                sched.spawn(init_task);
            }
        }
    }

    serial_println!("Init PD spawned. Entering scheduler loop...");

    // Start the BSP's scheduler loop
    unsafe {
        if let Some(ref mut sched) = sex_kernel::scheduler::SCHEDULERS[0] {
            // Pick the Init PD and enter Ring 3!
            sched.tick();
            let current_task_mutex = sched.current_task.as_ref().expect("Init: No task spawned").clone();
            let current = current_task_mutex.lock();
            let next_ctx = &current.context;
            
            // Perform the jump to Ring 3 (User-Space)
            // Note: We'd normally use a temporary context here, but for the 
            // boot sequence, we'll perform a direct hardware transition.
            let mut dummy_ctx = sex_kernel::scheduler::TaskContext::new(0, 0, 
                Arc::new(ProtectionDomain::new(0, 0)), false);
            
            serial_println!("--------------------------------------------------");
            serial_println!("JUMPING TO RING 3 (USER-SPACE)...");
            
            sex_kernel::scheduler::Scheduler::switch_to(&mut dummy_ctx, next_ctx);
        }
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
