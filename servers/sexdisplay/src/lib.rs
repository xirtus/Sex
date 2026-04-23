#![no_std]
extern crate alloc;

use sex_pdx::Rect;

pub struct Compositor {
    pub surface: Rect,
}

impl Compositor {
    pub fn new() -> Self {
        Self {
            surface: Rect { x: 0, y: 0, width: 1280, height: 720 },
        }
    }
}