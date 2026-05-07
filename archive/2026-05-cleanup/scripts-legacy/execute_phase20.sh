#!/usr/bin/env bash
set -e

echo "[*] INITIATING PHASE 20 AUTOMATION: ZERO-COPY HANDOVER"
echo "[*] Target: Single Address Space (SASOS) Intel PKU Isolation"

# 1. Create necessary directories
echo ">>> Creating directory structures..."
mkdir -p kernel/src/memory
mkdir -p kernel/src/pdx
mkdir -p servers/sexdisplay/src

# 2. Scaffold Intel PKU Substrate
echo ">>> Generating kernel/src/memory/pku.rs..."
cat << 'EOF' > kernel/src/memory/pku.rs
//! Intel Protection Keys for Userspace (PKU) Manager
//! Hardened for SASOS Hardware Isolation.

pub const PKEY_FB: u32 = 1;

/// Enable PKU in CR4 and clear the PKRU register to allow all access initially.
pub fn init_pku() {
    unsafe {
        // Enable Protection Keys (Bit 22 of CR4)
        core::arch::asm!(
            "mov rax, cr4",
            "or rax, 0x400000", 
            "mov cr4, rax",
            out("rax") _
        );
        // Initialize PKRU to 0 (Allow all access to all keys)
        wrpkru(0);
    }
}

/// Write to the PKRU register. 
/// Each PKEY has 2 bits: [Bit 2n: Access Disable, Bit 2n+1: Write Disable]
pub unsafe fn wrpkru(pkru: u32) {
    let edx = 0u32;
    let ecx = 0u32;
    core::arch::asm!(
        "wrpkru",
        in("eax") pkru,
        in("ecx") ecx,
        in("edx") edx,
    );
}

/// Helper to set bits 62:59 of a Page Table Entry.
/// TODO: Integrate with your specific page table walker.
pub unsafe fn set_page_pkey(addr: usize, pkey: u32) {
    // Scaffold: Implement leaf page table entry modification here.
    // Example mask: entry = (entry & !(0xF << 59)) | ((pkey as u64) << 59);
}

/// Tag a 4K-aligned memory range with a specific PKEY.
pub fn tag_region(start: usize, size: usize, pkey: u32) {
    let pages = (size + 4095) / 4096;
    for i in 0..pages {
        let addr = start + (i * 4096);
        unsafe { set_page_pkey(addr, pkey); }
    }
}
EOF

# 3. Scaffold PDX (Protection Domain Exchange) Definitions
echo ">>> Generating kernel/src/pdx.rs..."
cat << 'EOF' > kernel/src/pdx.rs
//! Protection Domain Exchange (PDX) Message Passing

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PdxFramebufferHandover {
    pub phys_addr: u64,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub pkey: u32,
}

/// Enqueue a message to a domain's ring buffer.
pub fn send_to_domain<T>(domain: &str, payload: T) {
    // Scaffold: Atomic push to the SAS domain ringbuffer.
    // serial_println!("PDX: Handover payload sent to {} domain", domain);
}

/// Receive a message from the current domain's ring buffer.
pub fn receive<T>() -> T {
    // Scaffold: Atomic pop/spin-wait from the domain ringbuffer.
    // Returning a dummy payload to satisfy the compiler until ringbuffer is wired.
    unsafe { core::mem::zeroed() }
}
EOF

# 4. Scaffold sexdisplay Crate
echo ">>> Generating servers/sexdisplay/Cargo.toml..."
cat << 'EOF' > servers/sexdisplay/Cargo.toml
[package]
name = "sexdisplay"
version = "0.1.0"
edition = "2021"

[dependencies]
# Add core/alloc dependencies or your standard SASOS lib here if necessary.
EOF

echo ">>> Generating servers/sexdisplay/src/main.rs..."
cat << 'EOF' > servers/sexdisplay/src/main.rs
#![no_std]
#![no_main]

// Note: Ensure your kernel exports the PDX structures or duplicate them safely here.
#[repr(C)]
pub struct PdxFramebufferHandover {
    pub phys_addr: u64,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub pkey: u32,
}

// Scaffold for serial printing in userland
macro_rules! serial_println {
    ($($arg:tt)*) => { /* Wire to userland UART syscall/PDX log */ };
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Await PDX Handover
    // let msg = pdx::receive::<PdxFramebufferHandover>();
    
    // Hardcoded dummy values for compilation. Replace with PDX receive.
    let msg = PdxFramebufferHandover {
        phys_addr: 0x0, // Replaced via PDX
        width: 1024,
        height: 768,
        pitch: 1024 * 4,
        pkey: 1,
    };
    
    let fb = msg.phys_addr as *mut u32;
    let size = (msg.width * msg.height) as usize;

    // 2. Draw Test Pattern (Requires PKEY 1 Write Access)
    if !fb.is_null() {
        unsafe {
            for i in 0..size {
                // Animated gradient: Zero-Copy Confirmation
                *fb.add(i) = 0xFF00FF00 | (i as u32 % 255);
            }
        }
        serial_println!("sexdisplay: Test pattern rendered — zero-copy confirmed");
    }

    // 3. Event Loop
    loop { 
        core::hint::spin_loop(); 
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
EOF

# 5. Generate Kernel Init Drop-in Snippet
echo ">>> Generating kernel_init_snippet.rs..."
cat << 'EOF' > kernel_init_snippet.rs
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
EOF

echo "[*] PHASE 20 AUTOMATION COMPLETE."
echo "[!] NEXT STEPS:"
echo "    1. Insert the code from 'kernel_init_snippet.rs' into your kernel initialization sequence."
echo "    2. Implement the page table walker inside 'set_page_pkey' in memory/pku.rs."
echo "    3. Add 'servers/sexdisplay' to your Limine configuration and final ISO payload."
