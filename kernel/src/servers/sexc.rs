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

#[repr(C)]
pub struct SexSysInfo {
    pub uptime: u64,
    pub total_ram: u64,
    pub used_ram: u64,
    pub pd_count: u32,
    pub cpu_count: u32,
}

impl sexc {
    /// POSIX sysinfo() equivalent for SexOS
    pub fn sysinfo(&self, buf_ptr: u64) -> i32 {
        let mut info = SexSysInfo {
            uptime: 12345, // Mock uptime
            total_ram: 2048 * 1024 * 1024,
            used_ram: 512 * 1024 * 1024,
            pd_count: crate::ipc::DOMAIN_REGISTRY.read().len() as u32,
            cpu_count: 4,
        };

        unsafe {
            let buf = buf_ptr as *mut SexSysInfo;
            core::ptr::write(buf, info);
        }
        0
    }
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

    /// POSIX socket(domain, type, proto)
    pub fn socket(&self, domain: i32, _type: i32, _proto: i32) -> Result<u32, &'static str> {
        serial_println!("sexc: socket(domain: {}, type: {})", domain, _type);
        
        // 1. Create a Socket Capability
        let cap_data = crate::capability::CapabilityData::Socket(crate::capability::SocketCapData {
            protocol: if domain == 1 { 0 } else { 6 }, // 0 = AF_UNIX, 6 = TCP
            local_port: 0,
            remote_addr: [0, 0, 0, 0],
            remote_port: 0,
        });

        let registry = DOMAIN_REGISTRY.read();
        let pd = registry.get(&self.caller_pd_id).ok_or("sexc: PD not found")?;
        
        Ok(pd.grant(cap_data))
    }

    /// POSIX bind(fd, addr, addrlen)
    pub fn bind(&self, fd: u32, _addr: u64, _len: u32) -> i32 {
        serial_println!("sexc: bind(fd: {})", fd);
        0 // Mock success
    }

    /// POSIX connect(fd, addr, addrlen)
    pub fn connect(&self, fd: u32, _addr: u64, _len: u32) -> i32 {
        serial_println!("sexc: connect(fd: {})", fd);
        0 // Mock success
    }

    /// POSIX send(fd, buf, len, flags)
    pub fn send(&self, fd: u32, buffer: *const u8, count: usize) -> Result<usize, &'static str> {
        // Route to sexnet via safe_pdx_call
        let registry = DOMAIN_REGISTRY.read();
        let pd = registry.get(&self.caller_pd_id).ok_or("sexc: PD not found")?;

        match safe_pdx_call(pd, fd, buffer as u64) {
            Ok(res) => Ok(res as usize),
            Err(e) => Err(e)
        }
    }

    /// POSIX recv(fd, buf, len, flags)
    pub fn recv(&self, fd: u32, buffer: *mut u8, count: usize) -> Result<usize, &'static str> {
        // Route to sexnet via safe_pdx_call
        let registry = DOMAIN_REGISTRY.read();
        let pd = registry.get(&self.caller_pd_id).ok_or("sexc: PD not found")?;

        match safe_pdx_call(pd, fd, buffer as u64) {
            Ok(res) => Ok(res as usize),
            Err(e) => Err(e)
        }
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
        match cap.data {
            crate::capability::CapabilityData::Spawn(_) => {},
            _ => return Err("sexc: Capability is not a Spawn Capability"),
        };

        // 2. Open binary via sexvfs
        let fd = self.open(binary_path, 0)?;
        
        // 3. Allocate new PD and PKU key
        let new_pd_id = 3000 + (x86_64::instructions::random::rdseed().unwrap_or(0) as u32 % 1000);
        let new_pku_key = (new_pd_id % 15) as u8 + 1; // Avoid key 0
        let new_pd = Arc::new(ProtectionDomain::new(new_pd_id, new_pku_key));
        DOMAIN_REGISTRY.write().insert(new_pd.id, new_pd.clone());

        // 4. Load binary into the Global SAS
        // For the bootstrap, we read the first 64KB (simplified)
        let mut buffer = [0u8; 65536];
        let bytes_read = self.read(fd, buffer.as_mut_ptr(), buffer.len())?;
        
        let mut gvas_lock = crate::memory::GLOBAL_VAS.lock();
        let entry_point = if let Some(ref mut gvas) = *gvas_lock {
            crate::elf::load_elf_for_pd(&buffer[..bytes_read], gvas, new_pku_key)?
        } else {
            return Err("sexc: Global VAS not initialized");
        };
        
        // 5. Create Task and Add to Scheduler via Load Balancer
        let stack_top = 0x_7000_0000_0000 + (new_pd_id as u64 * 0x1000_000);
        let user_task = crate::scheduler::Task {
            id: new_pd_id,
            context: crate::scheduler::TaskContext::new(entry_point.as_u64(), stack_top, new_pd, true),
            state: crate::scheduler::TaskState::Ready,
            signal_ring: Arc::new(crate::ipc_ring::RingBuffer::new()),
        };

        crate::scheduler::balanced_spawn(user_task);

        Ok(new_pd_id)
    }

    /// POSIX mmap() -> sext map_request
    pub fn mmap(&self, addr: u64, length: u64, prot: i32, flags: i32) -> Result<u64, &'static str> {
        serial_println!("sexc: mmap(addr: {:#x}, len: {}, prot: {}, flags: {})", addr, length, prot, flags);
        
        // 1. Construct MapRequest
        let req = crate::servers::sext::MapRequest {
            node_id: 1, // Local
            start: addr,
            size: length,
            pku_key: (self.caller_pd_id % 16) as u8, // Assign PD's default key
            writable: (prot & 0x2) != 0, // PROT_WRITE
            is_shm: (flags & 0x10) != 0, // MAP_SHARED
        };

        // 2. Interface with the Global VAS and PD Capability Table
        let cheri_cap = crate::cheri::SexCapability {
            base: addr,
            length,
            permissions: prot as u8,
            object_id: 0xDEAD_BEEF, // Simulated object ID
        };

        let registry = DOMAIN_REGISTRY.read();
        let pd = registry.get(&self.caller_pd_id).ok_or("sexc: PD not found")?;
        
        // Grant the memory capability to the PD so sext can validate it on fault
        pd.grant(crate::capability::CapabilityData::Memory(crate::capability::MemoryCapData {
            cheri_cap,
            pku_key: req.pku_key,
        }));

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

    /// POSIX lseek()
    pub fn lseek(&self, fd: u32, offset: i64, whence: i32) -> Result<i64, &'static str> {
        serial_println!("sexc: lseek(fd: {}, offset: {}, whence: {})", fd, offset, whence);
        Ok(offset) // Mock successful seek
    }

    /// POSIX fstat()
    pub fn fstat(&self, fd: u32, statbuf: u64) -> i32 {
        // serial_println!("sexc: fstat(fd: {})", fd);
        // Linux x86_64 struct stat layout (approximate offsets in 8-byte units)
        unsafe {
            let buf = statbuf as *mut u64;
            // Clear structure (144 bytes for standard Linux x86_64 stat)
            for i in 0..18 { *buf.add(i) = 0; }
            
            *buf.add(0) = 1; // st_dev
            *buf.add(1) = 2; // st_ino
            *buf.add(3) = 0o100644; // st_mode (Regular file, 644)
            *buf.add(4) = 1; // st_nlink
            *buf.add(5) = 1000; // st_uid
            *buf.add(6) = 1000; // st_gid
            *buf.add(8) = 40960; // st_size (Simulated size)
            *buf.add(9) = 512; // st_blksize
            *buf.add(11) = 80; // st_blocks
            
            // Timestamps (atime, mtime, ctime) - Mocking with current boot time
            *buf.add(12) = 123456789; // st_atime
            *buf.add(14) = 123456789; // st_mtime
            *buf.add(16) = 123456789; // st_ctime
        }
        0
    }

    /// POSIX stat()
    pub fn stat(&self, path: &str, statbuf: u64) -> i32 {
        serial_println!("sexc: stat(path: {})", path);
        self.fstat(0, statbuf)
    }

    /// POSIX poll()
    pub fn poll(&self, fds: u64, nfds: u64, timeout: i32) -> i32 {
        // serial_println!("sexc: poll(nfds: {}, timeout: {})", nfds, timeout);
        // Simple implementation: Always report ready for stdout/stderr, 
        // and check input buffer for stdin.
        let mut ready = 0;
        for i in 0..nfds {
            unsafe {
                let fd_struct = (fds + i * 8) as *mut i32; // Assuming struct pollfd { int fd; short events; short revents; }
                let fd = *fd_struct;
                let events = *(fds + i * 8 + 4) as *mut i16;
                let revents = (fds + i * 8 + 6) as *mut i16;

                if fd == 0 { // stdin
                    if crate::servers::tty::DEFAULT_TTY.lock().input_buffer.len() > 0 {
                        *revents = 1; // POLLIN
                        ready += 1;
                    } else {
                        *revents = 0;
                    }
                } else if fd == 1 || fd == 2 { // stdout/stderr
                    *revents = 4; // POLLOUT
                    ready += 1;
                } else {
                    *revents = 0;
                }
            }
        }
        ready
    }

    /// POSIX unlink()
    pub fn unlink(&self, path: &str) -> i32 {
        serial_println!("sexc: unlink(path: {})", path);
        0
    }

    /// POSIX rename()
    pub fn rename(&self, oldpath: &str, newpath: &str) -> i32 {
        serial_println!("sexc: rename(old: {}, new: {})", oldpath, newpath);
        0
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

    /// sexc: register_irq() -> Dynamically maps a hardware vector to a PDX ring.
    pub fn register_irq(&self, vector: u8, ring_cap_id: u32) -> Result<(), &'static str> {
        serial_println!("sexc: PD {} requesting IRQ registration for Vector {:#x}", self.caller_pd_id, vector);

        let registry = DOMAIN_REGISTRY.read();
        let pd = registry.get(&self.caller_pd_id).ok_or("sexc: PD not found")?;

        // 1. Verify Interrupt Capability
        let cap = pd.cap_table.caps.lock().iter().find(|c| {
            match c.data {
                crate::capability::CapabilityData::Interrupt(data) => data.irq == (vector - 0x20),
                _ => false,
            }
        }).ok_or("sexc: No valid Interrupt Capability for this vector")?.clone();

        // 2. Resolve the Ring Capability (Assuming it's an IPC cap to a ring buffer)
        // For the prototype, we'll assume the driver provides its own SpscRing structure address
        // passed via the ring_cap_id (acting as a pointer in SAS).
        let ring_ptr = ring_cap_id as *mut crate::ipc_ring::SpscRing<crate::interrupts::InterruptEvent>;
        let ring = unsafe { Arc::from_raw(ring_ptr) };

        // 3. Update VectorRoutingTable
        crate::interrupts::register_irq_route(vector, self.caller_pd_id, ring);

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

    // 2. Syscall table mapping (Standard x86_64 Linux numbers)
    match num {
        0 => { // sys_read
            match lib.read(arg0 as u32, arg1 as *mut u8, arg2 as usize) {
                Ok(count) => count as u64,
                Err(_) => u64::MAX,
            }
        },
        1 => { // sys_write
            match lib.write(arg0 as u32, arg1 as *const u8, arg2 as usize) {
                Ok(count) => count as u64,
                Err(_) => u64::MAX,
            }
        },
        2 => { // sys_open
            let path_ptr = arg0 as *const u8;
            // In a real system, we'd safely copy the string from user-space
            match lib.open("/disk0/app", arg1 as i32) {
                Ok(fd) => fd as u64,
                Err(_) => u64::MAX,
            }
        },
        3 => { // sys_close
            lib.close(arg0 as u32) as u64
        },
        4 => { // sys_stat
            lib.stat("/disk0/app", arg1) as u64
        },
        5 => { // sys_fstat
            lib.fstat(arg0 as u32, arg1) as u64
        },
        7 => { // sys_poll
            lib.poll(arg0, arg1, arg2 as i32) as u64
        },
        8 => { // sys_lseek
            match lib.lseek(arg0 as u32, arg1 as i64, arg2 as i32) {
                Ok(off) => off as u64,
                Err(_) => u64::MAX,
            }
        },
        9 => { // sys_mmap
            match lib.mmap(arg0, arg1, arg2 as i32, 0x02 | 0x20) { // PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS
                Ok(addr) => addr,
                Err(_) => u64::MAX,
            }
        },
        10 => lib.mprotect(arg0, arg1, arg2 as i32) as u64, // sys_mprotect
        12 => lib.brk(arg0), // sys_brk
        13 => { // sys_rt_sigaction
            match lib.sigaction(arg0 as i32, arg1) {
                Ok(_) => 0,
                Err(_) => u64::MAX,
            }
        },
        16 => { // sys_ioctl
            crate::servers::tty::handle_ioctl(arg0 as u32, arg1, arg2)
        },
        24 => { // sys_sched_yield
            unsafe {
                if let Some(ref mut sched) = crate::scheduler::SCHEDULERS[0] {
                    if let Some((old, next)) = sched.tick() {
                        // In a syscall, we don't have an interrupt frame, 
                        // but switch_to will save the GPRs.
                        crate::scheduler::Scheduler::switch_to(old, next);
                    }
                }
            }
            0
        },
        39 => lib.getpid() as u64, // sys_getpid
        41 => { // sys_socket
            match lib.socket(arg0 as i32, arg1 as i32, arg2 as i32) {
                Ok(fd) => fd as u64,
                Err(_) => u64::MAX,
            }
        },
        42 => { // sys_connect
            lib.connect(arg0 as u32, arg1, arg2 as u32) as u64
        },
        44 => { // sys_sendto
            match lib.send(arg0 as u32, arg1 as *const u8, arg2 as usize) {
                Ok(n) => n as u64,
                Err(_) => u64::MAX,
            }
        },
        45 => { // sys_recvfrom
            match lib.recv(arg0 as u32, arg1 as *mut u8, arg2 as usize) {
                Ok(n) => n as u64,
                Err(_) => u64::MAX,
            }
        },
        49 => { // sys_bind
            lib.bind(arg0 as u32, arg1, arg2 as u32) as u64
        },
        60 => 0, // sys_exit
        62 => { // sys_kill
            match lib.kill(arg0 as u32, arg1 as i32) {
                Ok(_) => 0,
                Err(_) => u64::MAX,
            }
        },
        82 => { // sys_rename
            lib.rename("/disk0/old", "/disk0/new") as u64
        },
        87 => { // sys_unlink
            lib.unlink("/disk0/file") as u64
        },
        228 => lib.clock_gettime(arg0 as i32), // sys_clock_gettime
        400 => { // sys_spawn_pd (Sex-specific)
            match lib.spawn_pd(arg0 as u32, "/bin/new_app") {
                Ok(pid) => pid as u64,
                Err(_) => u64::MAX,
            }
        },
        401 => { // sys_register_irq (Sex-specific)
            match lib.register_irq(arg0 as u8, arg1 as u32) {
                Ok(_) => 0,
                Err(_) => u64::MAX,
            }
        },
        402 => { // sys_sysinfo (Sex-specific)
            lib.sysinfo(arg0) as u64
        },
        _ => {
            serial_println!("sexc: Unknown syscall {}", num);
            u64::MAX
        }
    }
}
