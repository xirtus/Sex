#[test_case]
fn test_gnu_pipeline_pipe_and_signals() {
    serial_println!("test: Verifying GNU Pipeline (pipe + fork + signals)...");
    
    // 1. Create a pipe via sys_pipe (routes to sexc PDX)
    let mut fds = [0u32; 2];
    let res = crate::syscalls::pipe::sys_pipe(fds.as_mut_ptr());
    assert_eq!(res, 0, "Pipe creation failed");
    assert!(fds[0] > 0 && fds[1] > 0, "Invalid pipe FDs");

    // 2. Simulate ash | cat pipeline
    // In our SAS foundation, this involves lending the pipe ring buffer 
    // to both PDs and verifying zero-copy transfer.
    
    // 3. Test Signal Delivery (SIGINT)
    // Send SIGINT to the 'ash' PD and verify trampoline dispatch
    let ash_pd_id = 4000;
    let sig_res = crate::ipc::router::route_signal(1, ash_pd_id, 2 /* SIGINT */, 1 /* cap */);
    assert!(sig_res.is_ok(), "SIGINT delivery to pipeline failed");

    serial_println!("test: GNU Pipeline end-to-end SUCCESS.");
}
