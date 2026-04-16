#![no_std]
#![no_main]

use sex_pdx::{pdx_listen, pdx_reply, pdx_call, MessageType, DisplayProtocol, StoreProtocol, NodeProtocol, PageHandover};

// Minimal 8x16 font
static FONT: [[u8; 16]; 128] = include!("../font.txt");

/// sexstore-gui: Phase 20 Graphical Package Browser
/// Minimal, no_std, zero-copy GUI with package loading and caching.

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut gui = Gui::new();
    gui.run();
}

pub fn sys_park() {
    unsafe {
        core::arch::asm!("syscall", in("rax") 24);
    }
}

struct Gui {
    width: u32,
    height: u32,
    framebuffer: PageHandover,
}

impl Gui {
    fn new() -> Self {
        // Request a framebuffer from sexdisplay (Slot 3)
        let fb = pdx_call(3, 0, &MessageType::Display(DisplayProtocol::DisplayBufferAlloc { width: 1024, height: 768, format: 0 }) as *const _ as u64, 0);

        let mut gui = Self {
            width: 1024,
            height: 768,
            framebuffer: PageHandover { pfn: fb, pku_key: 5 }, // Assume key 5 from sexdisplay
        };
        
        gui.precache_packages();
        gui
    }

    fn run(&mut self) -> ! {
        self.draw_widgets();
        loop {
            sys_park();
            let req = pdx_listen(0);
            let msg = unsafe { *(req.arg0 as *const MessageType) };

            match msg {
                MessageType::HardwareInterrupt { vector, data: _ } => {
                    if vector == 0x80 { // Keyboard Interrupt
                        let scancode: u8 = unsafe { x86_64::instructions::port::Port::new(0x60).read() };
                        if scancode == 2 { // '1' key
                            self.load_package("bash");
                        }
                    }
                    pdx_reply(req.caller_pd, 0);
                },
                _ => {
                    pdx_reply(req.caller_pd, u64::MAX);
                }
            }
        }
    }
    
    fn precache_packages(&mut self) {
        let popular_packages = ["bash", "coreutils", "gcc"];
        for &pkg_name in popular_packages.iter() {
            let mut name_bytes = [0u8; 256];
            name_bytes[..pkg_name.len()].copy_from_slice(pkg_name.as_bytes());

            // Fetch the package to get a PageHandover
            let page_handover_ptr = pdx_call(4, 0, &MessageType::Store(StoreProtocol::FetchPackage { name: name_bytes }) as *const _ as u64, 0);
            let image = unsafe { *(page_handover_ptr as *const PageHandover) };

            // Send to cache
            pdx_call(4, 0, &MessageType::Store(StoreProtocol::CacheBinary { name: name_bytes, image }) as *const _ as u64, 0);
        }
    }

    fn load_package(&mut self, name: &str) {
        let mut name_bytes = [0u8; 256];
        name_bytes[..name.len()].copy_from_slice(name.as_bytes());

        // 1. Fetch package from sexstore (Slot 4)
        let page_handover_ptr = pdx_call(4, 0, &MessageType::Store(StoreProtocol::FetchPackage { name: name_bytes }) as *const _ as u64, 0);
        let image = unsafe { *(page_handover_ptr as *const PageHandover) };

        // 2. Load package into sexnode (Slot 5)
        pdx_call(5, 0, &MessageType::Node(NodeProtocol::LoadDriver { image }) as *const _ as u64, 0);
    }

    fn draw_widgets(&mut self) {
        self.fill_rect(0, 0, self.width, self.height, 0x1E1E2E); // Background
        self.draw_text(20, 20, "Available Packages:", 0xFFFFFF);
        
        self.draw_text(30, 50, "1. bash", 0x89B4FA);
        self.draw_text(30, 70, "2. coreutils", 0x89B4FA);
        self.draw_text(30, 90, "3. gcc", 0x89B4FA);
        self.draw_text(20, 120, "Press '1' to load bash", 0xCDD6F4);


        // Commit the buffer
        pdx_call(3, 0, &MessageType::Display(DisplayProtocol::DisplayBufferCommit { page: self.framebuffer }) as *const _ as u64, 0);
    }

    fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: u32) {
        let fb_ptr = self.framebuffer.pfn as *mut u32;
        for j in y..(y + h) {
            for i in x..(x + w) {
                unsafe {
                    fb_ptr.add((j * self.width + i) as usize).write_volatile(color);
                }
            }
        }
    }

    fn draw_text(&mut self, x: u32, y: u32, text: &str, color: u32) {
        let fb_ptr = self.framebuffer.pfn as *mut u32;
        let mut current_x = x;
        for c in text.chars() {
            if c as usize >= FONT.len() { continue; }
            let glyph = FONT[c as usize];
            for (row_idx, row) in glyph.iter().enumerate() {
                for col_idx in 0..8 {
                    if (row >> (7 - col_idx)) & 1 == 1 {
                        let px = current_x + col_idx;
                        let py = y + row_idx as u32;
                        if px < self.width && py < self.height {
                            unsafe {
                                fb_ptr.add((py * self.width + px) as usize).write_volatile(color);
                            }
                        }
                    }
                }
            }
            current_x += 8;
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { sys_park() }
}
