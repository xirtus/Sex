#![no_std]
#![no_main]

extern crate alloc;

use silkclient::{app_main, SexApp, SilkWindow};
use sex_pdx::*;

struct SilkBar {
    window: SilkWindow,
}

impl SexApp for SilkBar {
    fn new(_pdx: u32) -> Self {
        // Create a bar at the top: 1920x32 (standard stub size)
        let window = SilkWindow::create("silkbar", 1920, 32).expect("Failed to create silkbar window");
        Self { window }
    }

    fn run(&mut self, _pdx: u32) -> bool {
        // In a real implementation, we would listen for events or update the clock.
        // For the stub, we just paint once and yield.
        self.window.paint().unwrap();
        
        // Notify the shell slot that we are rendering the bar (if needed)
        unsafe {
            pdx_call(SLOT_SHELL as u32, OP_RENDER_BAR, self.window.id, 0, 0);
        }

        sex_pdx::sched_yield();
        true
    }
}

app_main!(SilkBar);
