use crate::pd::create::create_protection_domain;
use crate::serial_println;

/// sys_spawn_pd: The PDX/Syscall entry for spawning new domains.
/// IPCtax: Routes via create_protection_domain which uses pure PDX for ELF loading.
pub fn sys_spawn_pd(path_ptr: *const u8) -> i64 {
    // 1. Convert pointer to string (In SAS, address is global)
    let path = unsafe {
        let len = (0..4096).find(|&i| *path_ptr.add(i) == 0).unwrap_or(0);
        core::str::from_utf8_unchecked(core::slice::from_raw_parts(path_ptr, len))
    };

    serial_println!("syscall: sys_spawn_pd(\"{}\")", path);

    // 2. Execute Spawn (Lock-Free)
    match create_protection_domain(path, None) {
        Ok(pd_id) => pd_id as i64,
        Err(_) => -1,
    }
}
