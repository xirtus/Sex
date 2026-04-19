#!/bin/bash
set -e

echo "--- 1. Overwriting main.rs (Linker Section Syntax Fix) ---"
cat > kernel/src/main.rs << 'RS_EOF'
#![no_std]
#![no_main]

use limine::request::{FramebufferRequest, HhdmRequest, MemmapRequest};
use sex_kernel;

// Modern Rust Syntax: #[link_section = ".name"]
#[used]
#[link_section = ".limine_reqs"]
static FB_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[link_section = ".limine_reqs"]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[link_section = ".limine_reqs"]
static MEM_REQUEST: MemmapRequest = MemmapRequest::new();

#[no_mangle]
pub extern "C" fn _start() -> ! {
    sex_kernel::serial_println!("[SexOS] Phase 18: Hardware Bridge Initialized.");

    // Resolve responses with the new .response() API
    let hhdm = HHDM_REQUEST.response().expect("hhdm failed");
    let fb_res = FB_REQUEST.response().expect("fb failed");
    let fb = fb_res.framebuffers().iter().next().expect("no framebuffer");
    
    // Correctly apply HHDM offset to the physical address for Higher-Half access
    let fb_ptr = (fb.address() as u64 + hhdm.offset) as *mut u32;

    sex_kernel::serial_println!("Sex: FB at Phys {:?}, Width {}, Height {}", 
        fb.address(), fb.width, fb.height);

    // Draw the Blue/Cyan test pattern
    for y in 0..fb.height {
        for x in 0..fb.width {
            let color = (x as u32 % 255) | ((y as u32 % 255) << 8) | (0xFF << 16);
            let index = (y * (fb.pitch / 4) + x) as usize;
            unsafe {
                fb_ptr.add(index).write_volatile(color);
            }
        }
    }

    sex_kernel::serial_println!("[SexOS] SUCCESS: Framebuffer filled with test pattern.");

    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    sex_kernel::serial_println!("KERNEL PANIC: {}", info);
    loop {
        x86_64::instructions::hlt();
    }
}
RS_EOF

echo "--- 2. Compiling Kernel (Docker + compiler-builtins-mem) ---"
docker run --platform linux/amd64 --rm -v $(pwd):/sex -w /sex \
-e CARGO_UNSTABLE_JSON_TARGET_SPEC=true \
-e CARGO_UNSTABLE_BUILD_STD=core,alloc \
-e CARGO_UNSTABLE_BUILD_STD_FEATURES=compiler-builtins-mem \
--entrypoint /bin/bash sexos-builder:v28 -c "
    rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu && \
    cargo +nightly build --package sex-kernel --target x86_64-sex.json --release \
    --config \"target.x86_64-sex.rustflags=['-C', 'linker=rust-lld', '-C', 'target-cpu=skylake', '-C', 'link-arg=--script=kernel/linker.ld', '-C', 'code-model=kernel', '-C', 'relocation-model=static']\"
"

echo "--- 3. Launching System ---"
# Renaming for limine.cfg compatibility
mkdir -p iso_root
cp target/x86_64-sex/release/sex-kernel iso_root/sexos-kernel
cp limine.cfg limine-bios.sys limine-bios-cd.bin limine-uefi-cd.bin iso_root/ 2>/dev/null || true

xorriso -as mkisofs -b limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --efi-boot limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        iso_root -o sexos-v1.0.0.iso

qemu-system-x86_64 -cdrom sexos-v1.0.0.iso -serial stdio -m 512M -vga std -cpu max,+pku
