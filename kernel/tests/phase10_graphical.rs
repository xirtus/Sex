#[test_case]
fn test_graphical_glyph_render() {
    serial_println!("test: Verifying Phase 10 Graphical Plumbing (Glyph Render)...");
    
    // 1. Allocate lent buffer for glyph data
    let buffer = crate::memory::allocator::alloc_frame().expect("Test: OOM");
    
    // 2. Perform GpuCall via PDX to standalone sexdisplay
    // Simulation: PD 500 is sexdisplay
    let msg = crate::ipc::messages::MessageType::GpuCall {
        command: 1, // RENDER_GLYPH
        buffer_cap: 1, // Simulated cap to the allocated frame
        width: 8,
        height: 16,
    };
    
    // Simulation: Instead of direct DOMAIN_REGISTRY lookup, a real app uses its capability table.
    // For test purposes, assume capability slot 5 points to sexdisplay (granted at boot).
    let res_ptr = crate::ipc::safe_pdx_call(5, &msg as *const _ as u64).unwrap();
    
    let reply = unsafe { *(res_ptr as *const crate::ipc::messages::MessageType) };
    match reply {
        crate::ipc::messages::MessageType::GpuReply { status } => {
            assert_eq!(status, 0, "Glyph render failed");
        },
        _ => panic!("Expected GpuReply"),
    }
    
    serial_println!("test: Graphical Glyph Render SUCCESS.");
}
