#![no_std]
#![no_main]

extern crate alloc;

use silkclient::{app_main, SexApp, Window};
use sex_pdx::Pdx;

// This is a stub for a full port of COSMIC Files.
// A real port would involve significant work to adapt the original codebase.

struct {{app_name}};

impl SexApp for {{app_name}} {
    fn new(pdx: Pdx) -> Self {
        let mut window = Window::new(pdx, 800, 600, "COSMIC Files Port").unwrap();
        // In a real port, you would initialize the COSMIC Files UI here,
        // passing it the window buffer.
        window.present();
        Self
    }

    fn run(&mut self, pdx: Pdx) -> bool {
        // Handle events and update the UI
        true
    }
}

app_main!({{app_name}});
