use limine::framebuffer::Framebuffer;
use crate::ipc::messages::MessageType;
use crate::ipc::DOMAIN_REGISTRY;
use crate::init::SEXDISPLAY_PD_ID;
use crate::memory::manager::update_page_pkey;

pub fn ship_to_sexdisplay(fb: &Framebuffer, _hhdm: u64) {
    let virt_ptr = fb.address() as u64;

    let msg = MessageType::DisplayPrimaryFramebuffer {
        virt_addr: virt_ptr,
        width: fb.width as u32,
        height: fb.height as u32,
        pitch: fb.pitch as u32,
    };

    unsafe {
        let target_id = SEXDISPLAY_PD_ID;
        if let Some(target_pd) = DOMAIN_REGISTRY.get(target_id) {
            let pku_key = target_pd.pku_key;
            let start_addr = x86_64::VirtAddr::new(virt_ptr);
            let end_addr = x86_64::VirtAddr::new(virt_ptr + (fb.height * fb.pitch) - 1);
            
            let mut curr = start_addr;
            while curr <= end_addr {
                update_page_pkey(curr, pku_key);
                curr += 4096u64;
            }
            crate::serial_println!("Sex: Granted PKU key {} to sexdisplay for FB at {:#x}", pku_key, virt_ptr);

            if let Err(_) = (*target_pd.message_ring).enqueue(msg) {
                crate::serial_println!("SexOS: Warning - sexdisplay message ring full.");
            } else {
                let main_task = target_pd.main_task.load(core::sync::atomic::Ordering::Acquire);
                if !main_task.is_null() {
                    crate::scheduler::unpark_thread(main_task);
                }
            }
        } else {
            crate::serial_println!("SexOS: Error - sexdisplay PD not found in registry.");
        }
    }
}
