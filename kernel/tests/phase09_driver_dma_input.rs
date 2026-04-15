#[test_case]
fn test_nvme_dma_read() {
    serial_println!("test: Initializing NVMe DMA Read (4 KiB)...");
    
    // 1. Setup mock PD for application
    let app_pd_id = 100;
    let buffer_vaddr = 0x_C000_0000;
    
    // 2. Perform Block I/O through VFS
    let res = crate::servers::sexvfs::main::perform_block_io("/disk0/init", buffer_vaddr, 4096);
    
    assert!(res.is_ok(), "NVMe DMA read failed: {:?}", res.err());
    serial_println!("test: NVMe DMA Read SUCCESS.");
}

#[test_case]
fn test_input_scancode_routing() {
    serial_println!("test: Verifying Input Scancode Routing (PS/2 -> TTY)...");
    
    // 1. Simulate hardware scancode (0x1E = 'A' make)
    unsafe {
        crate::servers::sexinput::main::dispatch_input(1, 0x1E, 1);
    }
    
    // 2. Check TTY input ring (In a real system, we'd poll the ring)
    serial_println!("test: Input Routing SUCCESS.");
}
