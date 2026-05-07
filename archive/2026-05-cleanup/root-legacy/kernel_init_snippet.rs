// DROP THIS INTO kernel/src/lib.rs OR main.rs AFTER FRAMEBUFFER ACQUISITION

/*
// 1. Initialize hardware PKU
crate::memory::pku::init_pku();

// 2. Tag Framebuffer with PKEY 1
let fb_ptr = framebuffer.address();
let fb_size = (framebuffer.width() * framebuffer.height() * 4) as usize;
crate::memory::pku::tag_region(fb_ptr as usize, fb_size, crate::memory::pku::PKEY_FB);
serial_println!("PKU: Framebuffer tagged PKEY {}", crate::memory::pku::PKEY_FB);

// 3. Construct PDX Payload
let handover = crate::pdx::PdxFramebufferHandover {
    phys_addr: fb_ptr as u64,
    width: framebuffer.width() as u32,
    height: framebuffer.height() as u32,
    pitch: framebuffer.pitch() as u32,
    pkey: crate::memory::pku::PKEY_FB,
};

// 4. REVOKE KERNEL WRITE ACCESS
// Bit 3 (0b1000) disables write for Key 1. Access (Bit 2) remains 0 (allowed).
unsafe { crate::memory::pku::wrpkru(0b1000); }
serial_println!("PKU: Kernel write bit cleared via wrpkru");

// 5. Send Handover and Spawn
crate::pdx::send_to_domain("sexdisplay", handover);
serial_println!("PDX: Handover payload sent to sexdisplay domain");

// DO NOT ATTEMPT TO WRITE TO THE FRAMEBUFFER FROM THE KERNEL AFTER THIS LINE.
*/
