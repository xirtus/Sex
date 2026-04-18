
#[no_mangle]
pub extern "C" fn pdx_listen(_port: u32) -> crate::pdx::PdxRequest {
    let mut req = PdxRequest { caller_pd: 0, num: 0, arg0: 0, arg1: 0, arg2: 0 };
    unsafe {
        core::arch::asm!("syscall", 
            in("rax") 25, // pdx_listen syscall
            inout("rdi") req.caller_pd,
            inout("rsi") req.num,
            inout("rdx") req.arg0,
            inout("rcx") req.arg1,
            inout("r8") req.arg2,
        );
    }
    req
}

#[no_mangle]
pub extern "C" fn pdx_reply(caller_pd: u32, result: u64) {
    unsafe {
        core::arch::asm!("syscall", 
            in("rax") 26, // pdx_reply syscall
            in("rdi") caller_pd,
            in("rsi") result,
        );
    }
}

#[no_mangle]
pub extern "C" fn pdx_call(target_pd: u32, num: u64, arg0: u64, arg1: u64) -> u64 {
    let res: u64;
    unsafe {
        core::arch::asm!("syscall",
            in("rax") 27, // pdx_call syscall
            in("rdi") target_pd,
            in("rsi") num,
            in("rdx") arg0,
            in("rcx") arg1,
            lateout("rax") res,
        );
    }
    res
}

#[derive(Debug, Clone, Copy)]
pub enum SysError {
    VfsRegFail = 1,
    Unknown = 255,
}

pub fn safe_pdx_register(service_name: &str) -> Result<*mut u8, SysError> {
    let res: u64;
    unsafe {
        core::arch::asm!("syscall",
            in("rax") 0x10A, // SYS_PDX_REG
            in("rdi") service_name.as_ptr(),
            in("rsi") service_name.len(),
            lateout("rax") res,
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
