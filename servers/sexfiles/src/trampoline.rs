use crate::pdx::{pdx_listen_raw, pdx_reply, serial_println};
use crate::vfs;

#[no_mangle]
pub extern "C" fn trampoline_main() {
    // Optional RamFS proof run (compile-time flag).
    // Matches the silk-shell SCENE_SETTINGS_PROTOCOL_PROOF pattern.
    const RAMFS_PROOF_ENABLED: bool =
        option_env!("SEXFILES_RAMFS_PROOF").is_some();
    if RAMFS_PROOF_ENABLED {
        crate::proof::run_all_proofs();
    }

    serial_println!("[sexfiles.ready]");

    loop {
        // Listen on slot 0 (self message ring — all servers use this pattern).
        let msg = pdx_listen_raw(0);
        let caller = msg.caller_pd;

        // Route message type_id to VFS handler, passing caller PD for namespace/cap check
        let reply = vfs::handle_vfs_message(msg.type_id, msg.arg0, msg.arg1, msg.arg2, caller);

        // If type_id was 0 (empty/spurious), skip reply
        if msg.type_id != 0 {
            pdx_reply(caller, reply);
        }
    }
}
