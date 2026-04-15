use crate::pd::create::create_protection_domain;
use crate::serial_println;

/// sys_spawn_pd: The PDX/Syscall entry for spawning new domains.
pub fn sys_spawn_pd(path_ptr: *const u8) -> i64 {
    // 1. Convert pointer to string (In a real system, validate SAS bounds)
    let path = unsafe {
        let len = (0..4096).find(|&i| *path_ptr.add(i) == 0).unwrap_or(0);
        core::str::from_utf8_unchecked(core::slice::from_raw_parts(path_ptr, len))
    };

    serial_println!("syscall: sys_spawn_pd(\"{}\")", path);

    // 2. Route to PD lifecycle manager
    match create_protection_domain(path) {
        Ok(pd_id) => pd_id as i64,
        Err(e) => {
            serial_println!("syscall: spawn failed: {}", e);
            -1
        }
    }
}
