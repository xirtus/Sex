use crate::serial_println;
use crate::ipc::safe_pdx_call;
use crate::ipc::DOMAIN_REGISTRY;
use crate::capability::ProtectionDomain;
use crate::servers::vfs;
use alloc::sync::Arc;

/// Sex-Libc: POSIX Emulation Layer for the Sex Microkernel.
/// Maps standard C/POSIX calls to high-performance PDX operations.

pub struct SexLibc {
    pub caller_pd_id: u32,
}

impl SexLibc {
    pub fn new(pd_id: u32) -> Self {
        Self { caller_pd_id: pd_id }
    }

    /// POSIX open() -> VFS open()
    pub fn open(&self, path: &str, _flags: i32) -> Result<u32, &'static str> {
        serial_println!("LIBC: open(\"{}\")", path);
        // Map to the VFS server's open operation
        vfs::open(self.caller_pd_id, path)
    }

    /// POSIX read() -> Direct Driver IPC via Node Capability
    pub fn read(&self, fd: u32, buffer: *mut u8, count: usize) -> Result<usize, &'static str> {
        serial_println!("LIBC: read(fd: {}, count: {})", fd, count);
        
        let registry = DOMAIN_REGISTRY.read();
        let pd = registry.get(&self.caller_pd_id)
            .ok_or("LIBC: PD not found")?;

        // Perform safe_pdx_call using the file descriptor (which is a Capability ID)
        match safe_pdx_call(pd, fd, buffer as u64) {
            Ok(res) => Ok(res as usize),
            Err(e) => {
                serial_println!("LIBC: read error: {}", e);
                Err(e)
            }
        }
    }

    /// POSIX write() -> Direct Driver IPC or Serial/VGA PDX
    pub fn write(&self, fd: u32, buffer: *const u8, count: usize) -> Result<usize, &'static str> {
        // For fd 1 (stdout), we default to the Serial Server for this prototype
        if fd == 1 {
            serial_println!("LIBC: STDOUT write: {}", unsafe { 
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(buffer, count)) 
            });
            return Ok(count);
        }

        let registry = DOMAIN_REGISTRY.read();
        let pd = registry.get(&self.caller_pd_id)
            .ok_or("LIBC: PD not found")?;

        match safe_pdx_call(pd, fd, buffer as u64) {
            Ok(res) => Ok(res as usize),
            Err(e) => Err(e)
        }
    }

    /// POSIX close()
    pub fn close(&self, _fd: u32) -> i32 {
        serial_println!("LIBC: close(fd: {})", _fd);
        0
    }
}

/// The standard "syscall" entry point for C applications.
/// In a SASOS, this is just a function call (or a PDX jump to the Libc PD).
#[no_mangle]
pub extern "C" fn sex_syscall(num: u64, arg0: u64, arg1: u64, arg2: u64) -> u64 {
    // Basic syscall table mapping
    match num {
        1 => 0, // sys_exit
        2 => 0, // sys_open
        3 => 0, // sys_read
        4 => 0, // sys_write
        _ => u64::MAX,
    }
}
