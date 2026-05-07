use crate::pdx::{pdx_listen_raw, pdx_reply, serial_println};
use crate::messages;
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
    const DISKFS_OBJECT_TABLE_PROOF_ENABLED: bool =
        option_env!("SEXOS_DISKFS_OBJECT_TABLE_PROOF").is_some();
    if DISKFS_OBJECT_TABLE_PROOF_ENABLED {
        crate::proof::run_diskfs_object_table_proofs();
    }
    const SEXFILES_JOURNAL_PROOF_ENABLED: bool =
        option_env!("SEXOS_SEXFILES_JOURNAL_PROOF").is_some();
    if SEXFILES_JOURNAL_PROOF_ENABLED {
        crate::proof::run_sexfiles_journal_proofs();
    }
    const SEXFILES_REPLAY_PROOF_ENABLED: bool =
        option_env!("SEXOS_SEXFILES_REPLAY_PROOF").is_some();
    if SEXFILES_REPLAY_PROOF_ENABLED {
        crate::proof::run_sexfiles_replay_proofs();
    }
    const SEXFILES_CAP_RECORD_PROOF_ENABLED: bool =
        option_env!("SEXOS_SEXFILES_CAP_RECORD_PROOF").is_some();
    if SEXFILES_CAP_RECORD_PROOF_ENABLED {
        crate::proof::run_sexfiles_cap_record_proofs();
    }
    const LINEN_SEXFILES_METADATA_PROOF_ENABLED: bool =
        option_env!("SEXOS_LINEN_SEXFILES_METADATA_PROOF").is_some();
    if LINEN_SEXFILES_METADATA_PROOF_ENABLED {
        crate::proof::run_linen_sexfiles_metadata_proofs();
    }
    const SEXFILES_FAULT_INJECTION_PROOF_ENABLED: bool =
        option_env!("SEXOS_SEXFILES_FAULT_INJECTION_PROOF").is_some();
    if SEXFILES_FAULT_INJECTION_PROOF_ENABLED {
        crate::proof::run_sexfiles_fault_injection_proofs();
    }
    const SEXFILES_REAL_BLOCK_PROOF_ENABLED: bool =
        option_env!("SEXOS_SEXFILES_REAL_BLOCK_PROOF").is_some();
    if SEXFILES_REAL_BLOCK_PROOF_ENABLED {
        crate::proof::run_sexfiles_real_block_proofs();
    }
    const SEXFILES_REBOOT_PROOF_ENABLED: bool =
        option_env!("SEXOS_SEXFILES_REBOOT_PROOF").is_some();
    if SEXFILES_REBOOT_PROOF_ENABLED {
        crate::proof::run_sexfiles_reboot_proofs();
    }
    const SEXFILES_EXTENT_PROOF_ENABLED: bool =
        option_env!("SEXOS_SEXFILES_EXTENT_PROOF").is_some();
    if SEXFILES_EXTENT_PROOF_ENABLED {
        crate::proof::run_sexfiles_extent_proofs();
    }
    const SEXFILES_CHECKPOINT_PROOF_ENABLED: bool =
        option_env!("SEXOS_SEXFILES_CHECKPOINT_PROOF").is_some();
    if SEXFILES_CHECKPOINT_PROOF_ENABLED {
        crate::proof::run_sexfiles_checkpoint_proofs();
    }
    const DISKFS_MULTI_OBJECT_PROOF_ENABLED: bool = cfg!(sexfiles_diskfs_multi_object_proof);
    const SEXFILES_ROUTE_AUDIT_ONLY: bool = option_env!("SEXFILES_ROUTE_AUDIT_ONLY").is_some();
    if DISKFS_MULTI_OBJECT_PROOF_ENABLED {
        if SEXFILES_ROUTE_AUDIT_ONLY {
            serial_println!("[sexfiles.disk.multi.skip] reason=route_audit");
        } else {
            crate::proof::run_diskfs_multi_object_proofs();
        }
    }

    const SEXOBJECT_VIEW_PROOF_ENABLED: bool =
        option_env!("SEXOS_SEXOBJECT_VIEW_PROOF").is_some();
    if SEXOBJECT_VIEW_PROOF_ENABLED {
        crate::proof::run_sexobject_view_proof();
    }

    const LINEN_DISK_OBJECT_PROOF_ENABLED: bool =
        option_env!("SEXOS_LINEN_DISK_OBJECT_PROOF").is_some();
    if LINEN_DISK_OBJECT_PROOF_ENABLED {
        crate::proof::run_linen_disk_object_proof();
    }

    serial_println!("[sexfiles.ready]");

    loop {
        static mut SEXFILES_TRAMP_LISTEN_BUDGET: u32 = 8;
        unsafe {
            if SEXFILES_TRAMP_LISTEN_BUDGET > 0 {
                SEXFILES_TRAMP_LISTEN_BUDGET -= 1;
                serial_println!("[sexfiles.trampoline.listen.enter]");
            }
        }
        // Listen on slot 0 (self message ring — all servers use this pattern).
        let msg = pdx_listen_raw(0);
        let caller = msg.caller_pd;
        serial_println!(
            "[sexfiles.trampoline.after_listen] type={:#x} caller={} a0={:#x} a1={:#x}",
            msg.type_id, caller, msg.arg0, msg.arg1
        );
        serial_println!(
            "[sexfiles.route.recv] type={:#x} caller={} a0={:#x} a1={:#x}",
            msg.type_id, caller, msg.arg0, msg.arg1
        );

        // Route message type_id to VFS handler, passing caller PD for namespace/cap check
        let reply = vfs::handle_vfs_message(msg.type_id, msg.arg0, msg.arg1, msg.arg2, caller);

        // If type_id was 0 (empty/spurious), skip reply
        if msg.type_id != 0 {
            if msg.type_id == messages::OP_DISKFS_READ {
                serial_println!(
                    "[sexfiles.bridge.diskfs.read.reply.sent] caller={} value={:#x}",
                    caller, reply
                );
            }
            pdx_reply(caller, reply);
        }
    }
}
