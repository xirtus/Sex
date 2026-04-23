#![no_std]
#![no_main]

use core::panic::PanicInfo;
use silk_shell::{ShellState, Canvas, PdxCompositorClient};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // sexdisplay already handed off framebuffer via PDX (PKEY 1 locked)
    let mut state = ShellState::default();
    let compositor = PdxCompositorClient;

    // Create silkbar panel (top bar) + background surface
    state.panel_window_id = compositor.create_window(0, 0, 1280, silk_shell::PANEL_HEIGHT);
    compositor.set_bg(0xFF1E1E2E);  // SexOS signature dark

    // Launcher surface (initially hidden)
    state.launcher_window_id = compositor.create_window(0, silk_shell::PANEL_HEIGHT as i32,
                                                       silk_shell::LAUNCHER_WIDTH, 400);

    // Initial render
    compositor.render_bar(state.panel_window_id);

    loop {
        // Block on PDX ring (slot 6 = self, but we listen for HID from sexinput via sexdisplay)
        let event = sex_rt::pdx_listen();
        // Simple input stub (mouse/keyboard forwarded from sexdisplay)
        match event.1 {
            sex_rt::MessageType::HIDEvent { code, value, .. } => {
                if code == 1 && value == 1 { // left-click example
                    state.is_launcher_open = !state.is_launcher_open;
                }
            }
            _ => {}
        }

        // Safe drawing via Canvas (no raw pointer math)
        // In real sexdisplay handoff, we'd receive framebuffer pointer via MessageType::DisplayPrimaryFramebuffer
        // For Phase 25 stub we simulate with a placeholder address (will be replaced by real IPC)
        let fb_placeholder = 0xFFFF_8000_0000_0000 as *mut u32; // HHDM-mapped FB (PKEY 1)
        let mut canvas = Canvas::new(fb_placeholder, 1280, 720);
        canvas.draw_panel(&state);

        // Commit to compositor (typed PDX call under PKU lock)
        compositor.render_bar(state.panel_window_id);
    }
}
