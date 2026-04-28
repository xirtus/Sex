// IPC state removed: UCGM is now the sole source of system truth.
// Minimal stubs to satisfy syscalls/mod.rs references
pub const SVC_STATE_LISTENING: u8 = 0;

pub fn set_service_listening(_pd_id: u32) {
    // no-op: UCGM handles state now
}
