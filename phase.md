# SexOS Phase 18.md — Pure PDX Zero-Copy SexCompositor Build Plan
## Architecture Lock (MANDATORY — DO NOT DEVIATE)
- Legacy is dead: NO std, NO alloc, NO relibc, NO Wayland, NO Smithay, NO Cosmic, NO Orbital, NO protocol shims.
- SexCompositor is the final native object. It lives 100% inside servers/sexdisplay.
- It is a no_std bare-metal PDX-native compositor.
- Frame commit uses direct pdx_call(0, 0xDD, PFN-list) → kernel MMIO/scanout with ZERO copies.

## Current Status (as of this session)
- sexdisplay gradients are visible in QEMU → SexCompositor is already rendering via pure PDX path.
- Kernel panic: "sext lost: loader: stack OOM" at kernel/src/init.rs:27 (Segment 0x40000000 = 4096 bytes).
- Sext loader stack must be increased to 65536 bytes (64KiB) — this is the ONLY blocker.
- SexCompositor stub already exists in servers/sexdisplay/src/lib.rs (use the exact code below).

## Exact SexCompositor Code (lib.rs — use this verbatim)
```rust
#![no_std]
#![no_main]
#![feature(asm_const)]

pub const SEX_COMPOSITOR_MAGIC: u32 = 0x53455843; // 'SEXC'
pub const PDX_COMPOSITOR_COMMIT: u64 = 0xDD;

#[repr(C)]
pub struct SexCompositor {
    magic: u32,
    fb_width: u32,
    fb_height: u32,
    fb_stride: u32,
    current_scanout_pfn_base: u64,
}

extern "C" {
    fn pdx_call(cid: u64, func: u64, args: *const u64) -> i64;
}

impl SexCompositor {
    pub const fn new(width: u32, height: u32, stride: u32) -> Self {
        Self {
            magic: SEX_COMPOSITOR_MAGIC,
            fb_width: width,
            fb_height: height,
            fb_stride: stride,
            current_scanout_pfn_base: 0,
        }
    }

    pub fn commit_frame(&mut self, pfn_list: &[u64]) -> Result<(), ()> {
        let args = [
            pfn_list.as_ptr() as u64,
            pfn_list.len() as u64,
            self.fb_width as u64,
            self.fb_height as u64,
            self.fb_stride as u64,
        ];
        let ret = unsafe { pdx_call(0, PDX_COMPOSITOR_COMMIT, args.as_ptr()) };
        if ret == 0 {
            self.current_scanout_pfn_base = pfn_list[0];
            Ok(())
        } else {
            Err(())
        }
    }
}
```

## servers/sexdisplay/src/main.rs (minimal entry point — use this verbatim)
```rust
#![no_std]
#![no_main]
use sexdisplay::SexCompositor;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut compositor = SexCompositor::new(1920, 1080, 7680);
    let test_pfn_list: [u64; 4] = [0x10000, 0x11000, 0x12000, 0x13000];
    let _ = compositor.commit_frame(&test_pfn_list);
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
```

## Cargo.toml for sexdisplay (exact)
```toml
[package]
name = "sexdisplay"
version = "0.1.0"
edition = "2021"

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
lto = true
codegen-units = 1
opt-level = "s"
```

## Step-by-Step Build & Verification Plan for THIS Phase
1. Fix sext stack OOM (increase to 64KiB in kernel/src/init.rs).
2. Rebuild sexdisplay as pure no_std.
3. cargo check -p sexdisplay --target x86_64-unknown-none
4. cargo build -p sexdisplay --target x86_64-unknown-none --release
5. Mint Limine ISO and boot QEMU with -cpu max,+pku.
6. Verify zero-copy pdx_call(0, 0xDD, ...) and no panic.
7. Mainline push.

## Next Session Instructions
When a new Gemini CLI session starts, open phase.md and execute the plan from top to bottom. Do not ask for clarification — everything is here.

Output ONLY the full content of phase.md (nothing else) so I can copy-paste it directly into the file.