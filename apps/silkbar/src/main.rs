// Stub for cosmic-panel template
#![no_std]
#![no_main]
extern crate alloc;
use silkclient::{app_main, SexApp};
use sex_pdx::Pdx;
struct silkbar;
impl SexApp for silkbar {
    fn new(pdx: Pdx) -> Self { Self }
    fn run(&mut self, pdx: Pdx) -> bool { true }
}
app_main!(silkbar);
