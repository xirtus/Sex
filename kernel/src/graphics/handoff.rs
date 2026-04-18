use limine::framebuffer::Framebuffer;
use crate::ipc::messages::MessageType;
use crate::ipc::DOMAIN_REGISTRY;
use crate::init::SEXDISPLAY_PD_ID;

/// Ships the framebuffer metadata to the sexdisplay Protection Domain.
/// This maintains the SASOS philosophy: No copy, just a permission handoff.
pub fn ship_to_sexdisplay(fb: &Framebuffer, hhdm: u64) {
    let virt_ptr = fb.address() as u64;

    let msg = MessageType::DisplayPrimaryFramebuffer {
        virt_addr: virt_ptr,
        width: fb.width as u32,
        height: fb.height as u32,
        pitch: fb.pitch as u32,
    };

    // Lock-free dispatch to the sexdisplay server
    unsafe {
        let target_id = SEXDISPLAY_PD_ID;
        if let Some(target_pd) = DOMAIN_REGISTRY.get(target_id) {
            // Update FB page permissions to allow sexdisplay access (PKU)
            let pku_key = target_pd.pku_key;
            let start_page = x86_64::structures::paging::Page::<x86_64::structures::paging::Size4KiB>::containing_address(x86_64::VirtAddr::new(virt_ptr));
            let end_page = x86_64::structures::paging::Page::<x86_64::structures::paging::Size4KiB>::containing_address(x86_64::VirtAddr::new(virt_ptr + (fb.height * fb.pitch) - 1));
            
            for page in x86_64::structures::paging::Page::range_inclusive(start_page, end_page) {
                crate::memory::update_page_pkey(page, pku_key, x86_64::VirtAddr::new(hhdm));
            }
            crate::serial_println!("Sex: Granted PKU key {} to sexdisplay for FB at {:#x}", pku_key, virt_ptr);

            // Push to target's message ring (XIPC protocol)
            if let Err(_) = (*target_pd.message_ring).enqueue(msg) {
                crate::serial_println!("SexOS: Warning - sexdisplay message ring full.");
            } else {
                // Unpark target if it's sleeping
                let trampoline_task = target_pd.trampoline_task.load(core::sync::atomic::Ordering::Acquire);
                if !trampoline_task.is_null() {
                    crate::scheduler::unpark_thread(trampoline_task);
                }
            }
        } else {
            crate::serial_println!("SexOS: Error - sexdisplay PD not found in registry.");
        }
    }
}
