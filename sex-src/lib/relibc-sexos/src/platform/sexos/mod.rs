//! SexOS Platform Backend for Relibc
//! Maps standard POSIX calls to Sex Microkernel PDX/Syscalls.

use crate::platform::types::*;
use core::arch::asm;

pub const SYS_READ: usize = 0;
pub const SYS_WRITE: usize = 1;
pub const SYS_OPEN: usize = 2;
pub const SYS_CLOSE: usize = 3;
pub const SYS_STAT: usize = 4;
pub const SYS_FSTAT: usize = 5;
pub const SYS_POLL: usize = 7;
pub const SYS_LSEEK: usize = 8;
pub const SYS_MMAP: usize = 9;
pub const SYS_MPROTECT: usize = 10;
pub const SYS_BRK: usize = 12;
pub const SYS_SIGACTION: usize = 13;
pub const SYS_IOCTL: usize = 16;
pub const SYS_YIELD: usize = 24;
pub const SYS_GETPID: usize = 39;
pub const SYS_EXIT: usize = 60;
pub const SYS_KILL: usize = 62;
pub const SYS_RENAME: usize = 82;
pub const SYS_UNLINK: usize = 87;
pub const SYS_CLOCK_GETTIME: usize = 228;

pub fn close(fd: c_int) -> c_int {
    unsafe { syscall4(SYS_CLOSE, fd as usize, 0, 0) as c_int }
}

pub fn mmap(addr: *mut c_void, len: size_t, prot: c_int, flags: c_int, fd: c_int, offset: off_t) -> *mut c_void {
    unsafe {
        // x86_64 mmap uses 6 arguments, let's add syscall6
        syscall6(SYS_MMAP, addr as usize, len as usize, prot as usize, flags as usize, fd as usize, offset as usize) as *mut c_void
    }
}

pub fn fstat(fd: c_int, buf: *mut stat) -> c_int {
    unsafe { syscall4(SYS_FSTAT, fd as usize, buf as usize, 0) as c_int }
}

pub fn poll(fds: *mut pollfd, nfds: nfds_t, timeout: c_int) -> c_int {
    unsafe { syscall4(SYS_POLL, fds as usize, nfds as usize, timeout as usize) as c_int }
}

pub fn clock_gettime(clk_id: clockid_t, tp: *mut timespec) -> c_int {
    unsafe { syscall4(SYS_CLOCK_GETTIME, clk_id as usize, tp as usize, 0) as c_int }
}

#[inline(always)]
pub unsafe fn syscall6(num: usize, arg0: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> usize {
    let ret: usize;
    asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg0,
        in("rsi") arg1,
        in("rdx") arg2,
        in("r10") arg3,
        in("r8") arg4,
        in("r9") arg5,
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
