use crate::serial_println;
use crate::ipc_ring::SpscRing;

/// A descriptor for a storage operation (Inspired by seL4 sDDF).
#[repr(C)]
pub struct IoDescriptor {
    pub lba: u64,
    pub count: u32,
    pub buffer_phys: u64,
    pub op: u8, // 0 = Read, 1 = Write
}

use crate::servers::nvme::NvmeController;

pub struct Storagesexdrive {
    pub name: &'static str,
    pub nvme: Option<NvmeController>,
}

impl Storagesexdrive {
    pub fn new(name: &'static str) -> Self {
        Self { name, nvme: None }
    }

    /// Processes a single descriptor using the high-performance NVMe driver.
    pub fn process_desc(&mut self, desc: IoDescriptor) -> Result<(), &'static str> {
        serial_println!("STORAGE [{}]: Processing NVMe Descriptor: LBA={}, Phys={:#x}", 
            self.name, desc.lba, desc.buffer_phys);
        
        if let Some(ref mut nvme) = self.nvme {
            if desc.op == 0 {
                nvme.read_blocks(desc.lba, desc.count as u16, desc.buffer_phys);
            }
            return Ok(());
        }
        
        // Fallback to legacy IDE for older hardware
        let drive = IdeDrive::new(0x1F0, false);
        unsafe {
            if desc.op == 0 {
                drive.read_sector(desc.lba as u32, desc.buffer_phys as *mut u16);
            }
        }
        Ok(())
    }
}

/// The sexdrive's PDX entry point.
pub extern "C" fn storage_entry(arg: u64) -> u64 {
    let args = unsafe { &*(arg as *const crate::servers::sexc::FsArgs) };
    
    match args.command {
        crate::servers::sexc::FS_READ => {
            serial_println!("STORAGE: PDX Read: LBA={}, Count={}, Buf={:#x}", 
                args.offset, args.size, args.buffer);
            
            // In a real system, we'd use the Storagesexdrive instance
            // For the prototype, we use handle_read which talks to IDE
            handle_read(1, args.offset, args.size, args.buffer)
        },
        crate::servers::sexc::FS_GETATTR => {
            serial_println!("STORAGE: PDX GetAttr: Buf={:#x}", args.buffer);
            // Mock stat response for now
            unsafe {
                let buf = args.buffer as *mut u64;
                for i in 0..18 { *buf.add(i) = 0; }
                *buf.add(8) = 1024 * 1024; // 1MB size
            }
            0
        }
        _ => {
            serial_println!("STORAGE: Unknown PDX Command {}", args.command);
            u64::MAX
        }
    }
}

/// Block Cache PD (Phase 3 Step 2.2)
/// A dedicated domain for caching disk blocks.
pub struct BlockCache {
    // Shared via "Domain Fusion" with the Storage sexdrive and sexvfs.
}

// --- IDE/ATA Support (Real Implementation) ---

pub struct IdeDrive {
    pub io_base: u16,
    pub is_slave: bool,
}

impl IdeDrive {
    pub fn new(io_base: u16, is_slave: bool) -> Self {
        Self { io_base, is_slave }
    }

    /// Reads 256 words (512 bytes) from the disk into the buffer.
    pub unsafe fn read_sector(&self, lba: u32, buffer: *mut u16) {
        let drive_sel = if self.is_slave { 0xF0 } else { 0xE0 };
        
        // 1. Select Drive and send LBA
        x86_64::instructions::port::Port::new(self.io_base + 6).write((drive_sel | ((lba >> 24) & 0x0F) as u8) as u8);
        x86_64::instructions::port::Port::new(self.io_base + 2).write(1u8); // Sector count
        x86_64::instructions::port::Port::new(self.io_base + 3).write(lba as u8);
        x86_64::instructions::port::Port::new(self.io_base + 4).write((lba >> 8) as u8);
        x86_64::instructions::port::Port::new(self.io_base + 5).write((lba >> 16) as u8);
        
        // 2. Send Read Command (0x20)
        x86_64::instructions::port::Port::new(self.io_base + 7).write(0x20u8);

        // 3. Wait for Ready
        while (x86_64::instructions::port::Port::<u8>::new(self.io_base + 7).read() & 0x08) == 0 {}

        // 4. Read Data
        let mut data_port = x86_64::instructions::port::Port::<u16>::new(self.io_base);
        for i in 0..256 {
            *buffer.add(i) = data_port.read();
        }
    }
}

pub fn handle_read(node_id: u64, offset: u64, size: u64, buffer: u64) -> u64 {
    serial_println!("STORAGE: Reading LBA {} into {:#x}", offset, buffer);
    
    // In this prototype, we'll assume a standard primary IDE controller
    let drive = IdeDrive::new(0x1F0, false);
    unsafe {
        drive.read_sector(offset as u32, buffer as *mut u16);
    }
    
    0
}
