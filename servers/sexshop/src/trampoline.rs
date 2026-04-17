use libsys::pdx::safe_pdx_register;
use sex_pdx::ring::{AtomicRing, PdxReply};
use sex_pdx::StoreProtocol;

#[unsafe(no_mangle)]
pub extern "C" fn trampoline_main() {
    // Register as 'store' to replace legacy sex-store in PDX Slot 4
    let ring_ptr = safe_pdx_register("store").expect("STORE_REG_FAIL");
    let ring = unsafe { &*(ring_ptr as *const AtomicRing<StoreProtocol>) };

    loop {
        // High-performance polling loop
        if let Some(msg) = ring.pop_front() {
            let mut reply = PdxReply::default();
            crate::pdx::handle_store_message(&msg, &mut reply);
            
            // In a real implementation, the ring might hold both req/reply
            // or there is a separate reply ring. 
            // Phase-19 trampoline pattern assumes the server can push back.
            // ring.push_reply(reply); // Placeholder
        }
        core::hint::spin_loop();
    }
}
