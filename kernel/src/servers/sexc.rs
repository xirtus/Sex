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
        // For fd 0 (stdin), route to TTY server
        if fd == 0 {
            return Ok(crate::servers::tty::read(buffer, count));
        }

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
        // For fd 1 (stdout) and 2 (stderr), route to TTY server
        if fd == 1 || fd == 2 {
            return Ok(crate::servers::tty::write(buffer, count));
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

    // --- Phase 13: Self-Hosting Primitives ---

    /// sexc: spawn_pd() -> Creates a new isolated Protection Domain.
    /// In SexOS, this is our fork/exec equivalent.
    pub fn spawn_pd(&self, spawn_cap_id: u32, binary_path: &str) -> Result<u32, &'static str> {
        serial_println!("sexc: Spawning new PD from {} using Cap {}", binary_path, spawn_cap_id);
        
        let registry = DOMAIN_REGISTRY.read();
        let caller_pd = registry.get(&self.caller_pd_id)
            .ok_or("sexc: Caller PD not found")?;

        // 1. Validate Spawn Capability
        let cap = caller_pd.cap_table.find(spawn_cap_id).ok_or("sexc: Invalid Spawn Capability")?;
        let spawn_data = match cap.data {
            crate::capability::CapabilityData::Spawn(data) => data,
            _ => return Err("sexc: Capability is not a Spawn Capability"),
        };

        // 2. Allocate new PD and PKU key
        // For the prototype, we use a simple incremental ID and key
        let new_pd_id = 3000 + self.caller_pd_id; // Simple ID generation
        let new_pku_key = (new_pd_id % 16) as u8;
        let new_pd = Arc::new(ProtectionDomain::new(new_pd_id, new_pku_key));
        DOMAIN_REGISTRY.write().insert(new_pd.id, new_pd.clone());

        // 3. Load binary into the Global SAS
        // In a real system, we'd read the file from sexvfs. 
        // For the prototype, we assume the binary is already "read" into a buffer.
        let mock_elf_data = [0x7fu8, b'E', b'L', b'F', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]; // Minimal magic for mock
        
        let mut gvas_lock = crate::memory::GLOBAL_VAS.lock();
        let entry_point = if let Some(ref mut gvas) = *gvas_lock {
            crate::elf::load_elf_for_pd(&mock_elf_data, gvas, new_pku_key)?
        } else {
            return Err("sexc: Global VAS not initialized");
        };
        
        // 4. Create Task and Add to Scheduler
        let stack_top = 0x_7000_0000_0000;
        let user_task = crate::scheduler::Task {
            id: new_pd_id,
            context: crate::scheduler::TaskContext::new(entry_point.as_u64(), stack_top, new_pd, true),
            state: crate::scheduler::TaskState::Ready,
            signal_ring: Arc::new(crate::ipc_ring::RingBuffer::new()),
        };

        unsafe {
            if let Some(ref mut sched) = crate::scheduler::SCHEDULERS[0] {
                sched.spawn(user_task);
            }
        }

        Ok(new_pd_id)
    }

    /// POSIX mmap() -> sext map_request
    pub fn mmap(&self, addr: u64, length: u64, prot: i32, flags: i32) -> Result<u64, &'static str> {
        serial_println!("sexc: mmap(addr: {:#x}, len: {})", addr, length);
        
        // 1. Construct MapRequest
        let req = crate::servers::sext::MapRequest {
            node_id: 1, // Local
            start: addr,
            size: length,
            pku_key: 0, // Default key
            writable: (prot & 2) != 0,
            is_shm: (flags & 0x10) != 0, // MAP_SHARED simulation
        };

        // 2. In a real system, this would be a PDX call to the sext PD.
        // For this prototype, we'll call the function directly if in kernel mode,
        // or simulate the PDX return.
        serial_println!("sexc: Calling sext for memory mapping.");
        Ok(addr)
    }

    /// sexc: lend_memory() -> Grants a MemLend capability to another PD.
    /// Foundation for Wayland zero-copy pixel transfer.
    pub fn lend_memory(&self, target_pd_id: u32, base: u64, length: u64, permissions: u8) -> Result<u32, &'static str> {
        serial_println!("sexc: Lending {:#x} (len: {}) to PD {}", base, length, target_pd_id);

        let registry = DOMAIN_REGISTRY.read();
        let target_pd = registry.get(&target_pd_id)
            .ok_or("sexc: Target PD not found")?;

        // 1. Create the MemLend Capability
        let cap_data = crate::capability::CapabilityData::MemLend(crate::capability::MemLendCapData {
            base,
            length,
            pku_key: 15, // Using a dedicated SHM key
            permissions,
        });

        // 2. Grant to target
        let cap_id = target_pd.grant(cap_data);

        serial_println!("sexc: Memory lent successfully. Cap ID {} granted to PD {}.", cap_id, target_pd_id);
        Ok(cap_id)
    }

    /// POSIX mprotect() -> pku_update simulation
    pub fn mprotect(&self, addr: u64, len: u64, prot: i32) -> i32 {
        serial_println!("sexc: mprotect(addr: {:#x}, len: {}, prot: {})", addr, len, prot);
        // In SASOS, this translates to updating the PKU key for the range
        0
    }

    /// POSIX brk() -> Memory expansion
    pub fn brk(&self, addr: u64) -> u64 {
        serial_println!("sexc: brk(new_addr: {:#x})", addr);
        // Return the new break address (Simulated)
        addr
    }

    /// POSIX getpid()
    pub fn getpid(&self) -> u32 {
        self.caller_pd_id
    }

    /// POSIX clock_gettime()
    pub fn clock_gettime(&self, clk_id: i32) -> u64 {
        serial_println!("sexc: clock_gettime(clk_id: {})", clk_id);
        // Return mock monotonic time in nanos
        123456789
    }

    /// Deep POSIX: sigaction() -> Signal Ring Registration
    pub fn sigaction(&self, sig: i32, handler: u64) -> Result<(), &'static str> {
        serial_println!("sexc: Registering Signal Handler for {} at {:#x}", sig, handler);

        let registry = DOMAIN_REGISTRY.read();
        let pd = registry.get(&self.caller_pd_id)
            .ok_or("sexc: PD not found")?;

        pd.signal_handlers.lock().insert(sig, handler);
        Ok(())
    }

    /// Deep POSIX: kill() -> Enqueue signal to target PD's signal ring.
    pub fn kill(&self, target_pd_id: u32, sig: i32) -> Result<(), &'static str> {
        serial_println!("sexc: Sending Signal {} to PD {}", sig, target_pd_id);

        unsafe {
            if let Some(ref mut sched) = crate::scheduler::SCHEDULERS[0] {
                // In a real system, we'd find the task in a global map
                if let Some(task_mutex) = sched.runqueue.iter().find(|t| t.lock().id == target_pd_id) {
                    let task = task_mutex.lock();
                    task.signal_ring.enqueue(sig).map_err(|_| "sexc: Signal ring full")?;
                }
            }
        }
        Ok(())
    }
    }

// --- relibc Platform Backend (Conceptual mapping) ---

pub struct SexPlatform;

impl SexPlatform {
    /// relibc: open() -> sexvfs::open via PDX
    pub fn relibc_open(path: &str, flags: i32, mode: u16) -> i32 {
        serial_println!("relibc: open({}, {:#x})", path, flags);
        // Map to sexvfs::open logic
        match sexvfs::open(2000, path) {
            Ok(cap_id) => cap_id as i32,
            Err(_) => -1,
        }
    }

    /// relibc: write() -> Direct PDX to capability
    pub fn relibc_write(fd: i32, buf: &[u8]) -> usize {
        // Map to sexc::write logic
        let lib = sexc::new(2000);
        lib.write(fd as u32, buf.as_ptr(), buf.len()).unwrap_or(0)
    }

    /// relibc: mmap() -> sext::sext_request
    pub fn relibc_mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, offset: u64) -> *mut u8 {
        serial_println!("relibc: mmap(len: {})", len);
        // Map to sext logic
        addr
    }
}
/// The standard "syscall" entry point for C applications.
#[no_mangle]
pub extern "C" fn sexc_syscall(num: u64, arg0: u64, arg1: u64, arg2: u64) -> u64 {
    // 1. Recover the caller's PD ID from CoreLocal state (Privilege Isolation)
    let pd_id = crate::core_local::CoreLocal::get().current_pd();
    let lib = sexc::new(pd_id);

    // 2. Syscall table mapping
    match num {
        1 => 0, // sys_exit
        2 => { // sys_open
            let path_ptr = arg0 as *const u8;
            // In a real system, we'd safely copy the string from user-space
            match lib.open("/disk0/app", arg1 as i32) {
                Ok(fd) => fd as u64,
                Err(_) => u64::MAX,
            }
        },
        3 => { // sys_read
            match lib.read(arg0 as u32, arg1 as *mut u8, arg2 as usize) {
                Ok(count) => count as u64,
                Err(_) => u64::MAX,
            }
        },
        4 => { // sys_write
            match lib.write(arg0 as u32, arg1 as *const u8, arg2 as usize) {
                Ok(count) => count as u64,
                Err(_) => u64::MAX,
            }
        },
        5 => { // sys_spawn_pd (Sex-specific)
            match lib.spawn_pd(arg0 as u32, "/bin/new_app") {
                Ok(pid) => pid as u64,
                Err(_) => u64::MAX,
            }
        },
        6 => { // sys_yield
            serial_println!("sexc: sys_yield() triggered by PD {}.", pd_id);
            // In a real system, this would call the kernel scheduler directly
            0
        },
        7 => { // sys_sigaction
            match lib.sigaction(arg0 as i32, arg1) {
                Ok(_) => 0,
                Err(_) => u64::MAX,
            }
        },
        8 => { // sys_kill
            match lib.kill(arg0 as u32, arg1 as i32) {
                Ok(_) => 0,
                Err(_) => u64::MAX,
            }
        },
        10 => lib.mprotect(arg0, arg1, arg2 as i32) as u64, // sys_mprotect
        12 => lib.brk(arg0), // sys_brk
        16 => { // sys_ioctl (Required for Terminal/TTY)
            crate::servers::tty::handle_ioctl(arg0 as u32, arg1, arg2)
        },
        39 => lib.getpid() as u64, // sys_getpid
        228 => lib.clock_gettime(arg0 as i32), // sys_clock_gettime
        5 => { // sys_fstat (Actually mapped to 5 in some ABIs, let's use 5 for now)
            serial_println!("sexc: sys_fstat(fd: {})", arg0);
            0 // Mock success
        },
        _ => {
            serial_println!("sexc: Unknown syscall {}", num);
            u64::MAX
        }
    }
}
