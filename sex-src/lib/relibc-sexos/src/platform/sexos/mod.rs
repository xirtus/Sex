//! SexOS Platform Backend for Relibc
//! Maps standard POSIX calls to Sex Microkernel PDX/Syscalls.

use crate::platform::types::*;
use core::arch::asm;

pub const SYS_EXIT: usize = 1;
pub const SYS_OPEN: usize = 2;
pub const SYS_READ: usize = 3;
pub const SYS_WRITE: usize = 4;
pub const SYS_YIELD: usize = 6;
pub const SYS_BRK: usize = 12;
pub const SYS_IOCTL: usize = 16;
pub const SYS_GETPID: usize = 39;

#[inline(always)]
pub unsafe fn syscall4(num: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let ret: usize;
    asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg0,
        in("rsi") arg1,
        in("rdx") arg2,
        out("rcx") _,
        out("r11") _,
        lateout("rax") ret,
    );
    ret
}

pub fn open(path: *const c_char, flags: c_int, mode: mode_t) -> c_int {
    unsafe { syscall4(SYS_OPEN, path as usize, flags as usize, mode as usize) as c_int }
}

pub fn read(fd: c_int, buf: *mut c_void, count: size_t) -> ssize_t {
    unsafe { syscall4(SYS_READ, fd as usize, buf as usize, count as usize) as ssize_t }
}

pub fn write(fd: c_int, buf: *const c_void, count: size_t) -> ssize_t {
    unsafe { syscall4(SYS_WRITE, fd as usize, buf as usize, count as usize) as ssize_t }
}

pub fn exit(status: c_int) -> ! {
    unsafe { syscall4(SYS_EXIT, status as usize, 0, 0); }
    loop {}
}

pub fn brk(addr: *mut c_void) -> *mut c_void {
    unsafe { syscall4(SYS_BRK, addr as usize, 0, 0) as *mut c_void }
}

pub fn getpid() -> pid_t {
    unsafe { syscall4(SYS_GETPID, 0, 0, 0) as pid_t }
}

pub fn ioctl(fd: c_int, request: c_ulong, arg: *mut c_void) -> c_int {
    unsafe { syscall4(SYS_IOCTL, fd as usize, request as usize, arg as usize) as c_int }
}
