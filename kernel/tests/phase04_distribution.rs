#[test_case]
fn test_remote_pdx_routing() {
    serial_println!("test: Verifying Distribution (Remote PDX Proxy)...");
    
    // 1. Create a socket via PDX to standalone sexnet
    let socket_cap = crate::syscalls::net::sys_socket(2 /* AF_INET */, 1 /* SOCK_STREAM */, 0);
    assert!(socket_cap > 0, "Socket creation failed");
    
    // 2. Allocate packet buffer (lent-memory source)
    let buffer = crate::memory::allocator::alloc_frame().expect("Test: buffer OOM");
    
    // 3. Send 1KiB packet via lent-memory PDX path
    // This routes to standalone sexnet which acts as the distribution proxy
    let send_res = crate::syscalls::net::sys_send(socket_cap as u32, buffer, 1024);
    assert_eq!(send_res, 1024, "Send result size mismatch");
    
    serial_println!("test: Distribution Remote PDX SUCCESS.");
}
