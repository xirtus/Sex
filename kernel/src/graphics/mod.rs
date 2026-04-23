pub mod handoff;

use limine::framebuffer::Framebuffer;
use crate::memory::manager::update_page_pkey;
use x86_64::VirtAddr;

/// Ensures the Limine Framebuffer is correctly tagged with Intel MPK PKEY 1
/// before passing it to userland.
pub fn tag_framebuffer_pkey(fb: &Framebuffer) {
    let virt_ptr = fb.address() as u64;
    let start_addr = VirtAddr::new(virt_ptr);
    let end_addr = VirtAddr::new(virt_ptr + (fb.height * fb.pitch) - 1);
    
    let mut curr = start_addr;
    while curr <= end_addr {
        update_page_pkey(curr, 1); // Tag with Intel MPK PKEY 1
        curr += 4096u64;
    }
    crate::serial_println!("SexOS: Limine Framebuffer correctly tagged with Intel MPK PKEY 1");
}
