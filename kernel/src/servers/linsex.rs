use crate::serial_println;
use crate::servers::sexc::sexc;

/// Lin-Sex: Linux Binary Compatibility Layer.
/// Intercepts Linux syscalls and maps them to Sex PDX / sexc calls.

pub struct LinSexLoader {
    pub target_pd_id: u32,
    pub libc: sexc,
}

impl LinSexLoader {
    pub fn new(pd_id: u32) -> Self {
        Self {
            target_pd_id: pd_id,
            libc: sexc::new(pd_id),
        }
    }

    /// Simulates loading a Linux ELF binary.
    pub fn load_elf(&self, path: &str) -> Result<(), &'static str> {
        serial_println!("LIN-SEX: Loading Linux ELF from {}...", path);
        // 1. Read ELF headers via sexvfs
        // 2. Map segments into Global SAS via sext
        // 3. Set up Linux-specific stack (auxiliary vector, env, args)
        Ok(())
    }

    /// The Linux Syscall Entry Point (e.g., int 0x80 or syscall instruction).
    pub fn handle_linux_syscall(&self, num: u64, arg0: u64, arg1: u64, arg2: u64) -> u64 {
        match num {
            0 => self.sys_read(arg0 as u32, arg1 as *mut u8, arg2 as usize),
            1 => self.sys_write(arg0 as u32, arg1 as *const u8, arg2 as usize),
            2 => self.sys_open(arg0 as *const u8, arg1 as i32),
            3 => self.sys_close(arg0 as u32),
            _ => {
                serial_println!("LIN-SEX: Unhandled Linux syscall {}", num);
                u64::MAX
            }
        }
    }

    fn sys_read(&self, fd: u32, buf: *mut u8, count: usize) -> u64 {
        match self.libc.read(fd, buf, count) {
            Ok(res) => res as u64,
            Err(_) => u64::MAX,
        }
    }

    fn sys_write(&self, fd: u32, buf: *const u8, count: usize) -> u64 {
        match self.libc.write(fd, buf, count) {
            Ok(res) => res as u64,
            Err(_) => u64::MAX,
        }
    }

    fn sys_open(&self, path_ptr: *const u8, flags: i32) -> u64 {
        // In a real system, we'd copy the string from the PD's memory
        let path = "/disk0/linux_app"; 
        match self.libc.open(path, flags) {
            Ok(fd) => fd as u64,
            Err(_) => u64::MAX,
        }
    }

    fn sys_close(&self, fd: u32) -> u64 {
        self.libc.close(fd) as u64
    }
}
