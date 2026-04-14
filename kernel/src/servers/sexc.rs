use crate::serial_println;
use crate::ipc::safe_pdx_call;
use crate::ipc::DOMAIN_REGISTRY;
use crate::capability::ProtectionDomain;
use crate::servers::sexvfs;
use alloc::sync::Arc;

/// sexc: POSIX Emulation Layer for the Sex Microkernel.
/// Maps standard C/POSIX calls to high-performance PDX operations.

pub struct sexc {
    pub caller_pd_id: u32,
}

impl sexc {
    pub fn new(pd_id: u32) -> Self {
        Self { caller_pd_id: pd_id }
    }

    /// POSIX open() -> sexvfs open()
    pub fn open(&self, path: &str, _flags: i32) -> Result<u32, &'static str> {
        serial_println!("sexc: open(\"{}\")", path);
        // Map to the sexvfs server's open operation
        sexvfs::open(self.caller_pd_id, path)
    }

    /// POSIX read() -> Direct sexdrive IPC via Node Capability
    pub fn read(&self, fd: u32, buffer: *mut u8, count: usize) -> Result<usize, &'static str> {
        serial_println!("sexc: read(fd: {}, count: {})", fd, count);
        
        let registry = DOMAIN_REGISTRY.read();
        let pd = registry.get(&self.caller_pd_id)
            .ok_or("sexc: PD not found")?;

        // Perform safe_pdx_call using the file descriptor (which is a Capability ID)
        match safe_pdx_call(pd, fd, buffer as u64) {
            Ok(res) => Ok(res as usize),
            Err(e) => {
                serial_println!("sexc: read error: {}", e);
                Err(e)
            }
        }
    }

    /// POSIX write() -> Direct sexdrive IPC or Serial/VGA PDX
    pub fn write(&self, fd: u32, buffer: *const u8, count: usize) -> Result<usize, &'static str> {
        // For fd 1 (stdout), we default to the Serial Server for this prototype
        if fd == 1 {
            serial_println!("sexc: STDOUT write: {}", unsafe { 
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(buffer, count)) 
            });
            return Ok(count);
        }

        let registry = DOMAIN_REGISTRY.read();
        let pd = registry.get(&self.caller_pd_id)
            .ok_or("sexc: PD not found")?;

        match safe_pdx_call(pd, fd, buffer as u64) {
            Ok(res) => Ok(res as usize),
            Err(e) => Err(e)
        }
    }

    /// POSIX close()
    pub fn close(&self, _fd: u32) -> i32 {
        serial_println!("sexc: close(fd: {})", _fd);
        0
    }

    // --- Wayland AF_UNIX Emulation ---

    /// POSIX socket(AF_UNIX, ...)
    pub fn socket(&self, domain: i32, _type: i32, _proto: i32) -> Result<u32, &'static str> {
        if domain == 1 { // AF_UNIX
            serial_println!("sexc: socket(AF_UNIX)");
            // In a SASOS, a socket is just a PDX capability to a local port
            return Ok(100); // Simulated socket FD
        }
        Err("sexc: Unsupported socket domain")
    }

    /// POSIX sendmsg() with Capability (FD) passing
    pub fn sendmsg(&self, fd: u32, msg: u64, _flags: i32) -> Result<usize, &'static str> {
        serial_println!("sexc: sendmsg(fd: {}) - Transferring Capabilities", fd);
        // Map to a PDX call that includes capability transfer in the message header
        Ok(0)
    }
}

/// The standard "syscall" entry point for C applications.
/// In a SASOS, this is just a function call (or a PDX jump to the sexc PD).
#[no_mangle]
pub extern "C" fn sexc_syscall(num: u64, arg0: u64, arg1: u64, arg2: u64) -> u64 {
    // Basic syscall table mapping
    match num {
        1 => 0, // sys_exit
        2 => 0, // sys_open
        3 => 0, // sys_read
        4 => 0, // sys_write
        _ => u64::MAX,
    }
}
