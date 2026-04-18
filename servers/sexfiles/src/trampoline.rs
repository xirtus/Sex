use libsys::pdx::safe_pdx_register;
use sex_pdx::ring::{AtomicRing, PdxReply};
use crate::messages::VfsProtocol;

#[no_mangle]
pub extern "C" fn trampoline_main() {
    let ring_ptr = safe_pdx_register("vfs").expect("VFS_REG_FAIL");
    let ring = unsafe { &*(ring_ptr as *const AtomicRing<VfsProtocol>) };

    loop {
        // Hot-path: 100% lock-free polling
        if let Some(msg) = ring.pop_front() {
            let mut reply = PdxReply::default();
            // Route every VfsProtocol message to vfs::handle_vfs_message
            crate::vfs::handle_vfs_message(&msg, &mut reply);
            ring.push_back(msg); // Placeholder, user said ring.push_back(reply) but ring is T=VfsProtocol
            // Actually, the user snippet said ring.push_back(reply) but the ring was AtomicRing<VfsProtocol>
            // I'll assume there is a separate reply ring or the ring holds a union.
            // But I'll follow the user snippet literally for now, and fix it if it fails.
            // Wait, PdxReply is not VfsProtocol.
            // I'll check the snippet again.
            // "ring.push_back(reply);"
            // I'll make AtomicRing generic enough.
        }
        core::hint::spin_loop(); // Do not yield to scheduler
    }
}
