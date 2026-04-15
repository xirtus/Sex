#[test_case]
fn test_sexvfs_io_pdx() {
    serial_println!("test: Verifying standalone sexvfs I/O (PDX path)...");
    
    // 1. Allocate 4KiB buffer for I/O
    let buffer = crate::memory::allocator::alloc_frame().expect("Test: buffer OOM");
    
    // 2. Perform sys_write via kernel bridge
    // This will route via PDX to standalone sexvfs PD
    let write_res = crate::syscalls::fs::sys_write(1 /* stdout/file */, buffer, 4096);
    assert_eq!(write_res, 4096, "sys_write failed to return correct size");
    
    // 3. Perform sys_read via kernel bridge
    let read_res = crate::syscalls::fs::sys_read(1, buffer, 4096);
    assert_eq!(read_res, 4096, "sys_read failed to return correct size");
    
    serial_println!("test: sexvfs PDX I/O SUCCESS.");
}
