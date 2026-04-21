// Stub for cosmic-settings template
#![no_std]
#![no_main]
extern crate alloc;
use silkclient::{app_main, SexApp};
use sex_pdx::Pdx;
struct {{app_name}};
impl SexApp for {{app_name}} {
    fn new(pdx: Pdx) -> Self { Self }
    fn run(&mut self, pdx: Pdx) -> bool { true }
}
app_main!({{app_name}});
