#[no_mangle]
pub extern "C" fn pdx_listen(_port: u32) -> crate::pdx::PdxRequest {
    let mut req = PdxRequest { caller_pd: 0, num: 0, arg0: 0, arg1: 0, arg2: 0 };
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("syscall", 
            in("rax") 25, // pdx_listen syscall
            inout("rdi") req.caller_pd,
            inout("rsi") req.num,
            inout("rdx") req.arg0,
            inout("r10") req.arg1, // ABI: 4th arg is r10
            inout("r8") req.arg2,
            lateout("rcx") _, lateout("r11") _, // Clobbers
        );
    }
    req
}

#[no_mangle]
pub extern "C" fn pdx_reply(caller_pd: u32, result: u64) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("syscall", 
            in("rax") 26, // pdx_reply syscall
            in("rdi") caller_pd,
            in("rsi") result,
            lateout("rcx") _, lateout("r11") _, // Clobbers
        );
    }
}

#[no_mangle]
pub extern "C" fn pdx_call(target_pd: u32, num: u64, arg0: u64, arg1: u64) -> u64 {
    let mut res: u64 = 0;
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("syscall",
            in("rax") 27, // pdx_call syscall
            in("rdi") target_pd,
            in("rsi") num,
            in("rdx") arg0,
            in("r10") arg1, // ABI: 4th arg is r10
            lateout("rax") res,
            lateout("rcx") _, lateout("r11") _, // Clobbers
        );
    }
    res
}

#[derive(Debug, Clone, Copy)]
pub enum SysError {
    VfsRegFail = 1,
    Unknown = 255,
}

/// Register a PDX service with the system
/// 
/// # Arguments
/// * `service_name` - The name of the service to register
/// 
/// # Returns
/// * `Ok(*mut u8)` - Pointer to the registered service
/// * `Err(SysError)` - Error if registration failed
pub fn safe_pdx_register(service_name: &str) -> Result<*mut u8, SysError> {
    let mut res: u64 = 0;
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("syscall",
            in("rax") 0x10A, // SYS_PDX_REG
            in("rdi") service_name.as_ptr(),
            in("rsi") service_name.len(),
            lateout("rax") res,
            lateout("rcx") _, lateout("r11") _, // Clobbers
        );
    }
    if res == 0 {
        Err(SysError::VfsRegFail)
    } else {
        Ok(res as *mut u8)
    }
}

#[repr(C)]
pub struct PdxRequest {
    pub caller_pd: u32,
    pub num: u64,
    pub arg0: u64,
    pub arg1: u64,
    pub arg2: u64,
}

#[cfg(not(target_arch = "x86_64"))]
#[allow(dead_code)] 
unsafe fn syscall_fallback() { 
    loop {} 
}
