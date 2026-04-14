use crate::serial_println;

/// The "Hello World" User-Space Shell.
/// This function is intended to be the entry point of a true Ring 3 application.
/// It uses the 'syscall' instruction to interact with the Sex Microkernel.

#[no_mangle]
pub extern "C" fn user_shell_entry() -> ! {
    // 1. Get current PID
    let pid = unsafe { syscall(39, 0, 0, 0) };
    
    // 2. Print "Hello from Userland!" via sys_write (num 4, fd 1)
    let msg = "--------------------------------------------------\n";
    unsafe { syscall(4, 1, msg.as_ptr() as u64, msg.len() as u64); }
    
    let msg2 = "Hello from Userland! I am an isolated Ring 3 PD.\n";
    unsafe { syscall(4, 1, msg2.as_ptr() as u64, msg2.len() as u64); }
    
    let msg3 = "--------------------------------------------------\n";
    unsafe { syscall(4, 1, msg3.as_ptr() as u64, msg3.len() as u64); }

    // 3. Loop forever (or until sys_exit)
    loop {
        // Yield to the kernel to prevent 100% CPU usage
        unsafe { syscall(6, 0, 0, 0); }
    }
}

/// A minimal syscall wrapper for x86_64.
/// This matches the 'sexc_syscall' table in sexc.rs.
#[inline(always)]
pub unsafe fn syscall(num: u64, arg0: u64, arg1: u64, arg2: u64) -> u64 {
    let mut ret: u64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg0,
        in("rsi") arg1,
        in("rdx") arg2,
        out("rcx") _, // rcx is clobbered by syscall
        out("r11") _, // r11 is clobbered by syscall
        lateout("rax") ret,
    );
    ret
}
