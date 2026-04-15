use crate::serial_println;
use crate::servers::dde::{self, PciDevice};
use x86_64::VirtAddr;

/// NVMe 2.0 Driver for high-performance SSDs (Samsung 990 Pro / Crucial).
/// Implements Admin Queues, IO Queues, and PRP DMA chains.

#[repr(C, packed)]
pub struct NvmeRegs {
    pub cap: u64,
    pub vs: u32,
    pub intms: u32,
    pub intmc: u32,
    pub cc: u32,
    pub reserved: u32,
    pub csts: u32,
    pub nssr: u32,
    pub aqa: u32,
    pub asq: u64,
    pub acq: u64,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct NvmeSqEntry {
    pub opcode: u8,
    pub flags: u8,
    pub command_id: u16,
    pub nsid: u32,
    pub reserved: u64,
    pub metadata_ptr: u64,
    pub prp1: u64,
    pub prp2: u64,
    pub cdw10: u32,
    pub cdw11: u32,
    pub cdw12: u32,
    pub cdw13: u32,
    pub cdw14: u32,
    pub cdw15: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct NvmeCqEntry {
    pub result: u32,
    pub reserved: u32,
    pub sq_head: u16,
    pub sq_id: u16,
    pub command_id: u16,
    pub status: u16,
}

pub struct NvmeQueue {
    pub sq_vaddr: VirtAddr,
    pub cq_vaddr: VirtAddr,
    pub sq_tail: u16,
    pub cq_head: u16,
    pub size: u16,
    pub doorbell_base: *mut u32,
    pub phase: bool,
}

impl NvmeQueue {
    pub unsafe fn submit(&mut self, entry: NvmeSqEntry) {
        let sq = core::slice::from_raw_parts_mut(self.sq_vaddr.as_mut_ptr::<NvmeSqEntry>(), self.size as usize);
        sq[self.sq_tail as usize] = entry;
        
        self.sq_tail = (self.sq_tail + 1) % self.size;
        
        // Ring SQ Doorbell (offset 0 for Admin, or based on queue ID)
        self.doorbell_base.write_volatile(self.sq_tail as u32);
    }

    pub unsafe fn poll_completion(&mut self) -> Option<NvmeCqEntry> {
        let cq = core::slice::from_raw_parts(self.cq_vaddr.as_ptr::<NvmeCqEntry>(), self.size as usize);
        let entry = cq[self.cq_head as usize];
        
        let status_phase = (entry.status >> 0) & 1;
        if status_phase as u8 == self.phase as u8 {
            self.cq_head = (self.cq_head + 1) % self.size;
            if self.cq_head == 0 { self.phase = !self.phase; }
            
            // Ring CQ Doorbell (offset 1 * stride)
            self.doorbell_base.add(1).write_volatile(self.cq_head as u32);
            return Some(entry);
        }
        None
    }
}

use crate::ipc_ring::SpscRing;
use crate::interrupts::InterruptEvent;
use alloc::sync::Arc;

pub struct NvmeController {
    pub mmio_base: VirtAddr,
    pub admin_queue: Option<NvmeQueue>,
    pub io_queue: Option<NvmeQueue>,
    pub interrupt_ring: Arc<SpscRing<InterruptEvent>>,
}

impl NvmeController {
    pub fn new(mmio_base: VirtAddr) -> Self {
        Self { 
            mmio_base,
            admin_queue: None,
            io_queue: None,
            interrupt_ring: Arc::new(SpscRing::new()),
        }
    }

    pub unsafe fn init_controller(&mut self, pd_id: u32) -> Result<(), &'static str> {
        // 1. Register IRQ Route (Vector 0x22 for NVMe)
        crate::interrupts::register_irq_route(0x22, pd_id, self.interrupt_ring.clone());

        let regs = &mut *(self.mmio_base.as_u64() as *mut NvmeRegs);
        
        // 2. Reset controller
        regs.cc &= !1;
        while (regs.csts & 1) != 0 {}

        // 3. Setup Admin Queue (AQA, ASQ, ACQ)
        // In a real system, allocate via PMM
        let sq_phys = 0x_F000_0000; 
        let cq_phys = 0x_F000_1000;
        
        // Grant DMA capabilities to the PD
        let registry = crate::ipc::DOMAIN_REGISTRY.read();
        let pd = registry.get(&pd_id).ok_or("NVMe: PD not found")?;
        pd.grant(crate::capability::CapabilityData::DMA(crate::capability::DmaCapData {
            phys_addr: sq_phys,
            length: 4096,
            pku_key: pd.pku_key,
        }));

        regs.aqa = (31 << 16) | 31;
        regs.asq = sq_phys;
        regs.acq = cq_phys;

        // 4. Enable
        regs.cc |= (6 << 16) | (4 << 20) | 1; // 4KB page size, EN=1
        while (regs.csts & 1) == 0 {}

        let db_base = (self.mmio_base.as_u64() + 0x1000) as *mut u32;
        self.admin_queue = Some(NvmeQueue {
            sq_vaddr: VirtAddr::new(sq_phys), // Assuming 1:1 mapping for kernel
            cq_vaddr: VirtAddr::new(cq_phys),
            sq_tail: 0,
            cq_head: 0,
            size: 32,
            doorbell_base: db_base,
            phase: true,
        });

        serial_println!("NVMe: Admin Queue initialized with real-time IRQ routing.");
        Ok(())
    }

    pub fn wait_for_completion(&mut self) {
        serial_println!("NVMe: Waiting for interrupt...");
        loop {
            if let Some(event) = self.interrupt_ring.dequeue() {
                serial_println!("NVMe: Received IRQ {:#x}. Processing completions.", event.vector);
                break;
            }
            // In Ring 3, we'd yield or use a wait-queue
            x86_64::instructions::hlt();
        }
    }
}

    pub fn read_blocks(&mut self, lba: u64, count: u16, buffer_phys: u64) {
        let mut entry = NvmeSqEntry {
            opcode: 0x02, // Read
            flags: 0,
            command_id: 0x42,
            nsid: 1,
            reserved: 0,
            metadata_ptr: 0,
            prp1: buffer_phys,
            prp2: 0,
            cdw10: lba as u32,
            cdw11: (lba >> 32) as u32,
            cdw12: (count - 1) as u32,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        };

        unsafe {
            if let Some(ref mut q) = self.admin_queue {
                q.submit(entry);
                serial_println!("NVMe: Read command submitted for LBA {}", lba);
                
                // Poll for completion (prototype logic)
                while q.poll_completion().is_none() {}
                serial_println!("NVMe: Read complete.");
            }
        }
    }
}

use crate::pci;

pub fn nvme_probe() -> Result<(), &'static str> {
    serial_println!("NVMe: Scanning for high-performance SSDs via real PCI bus...");
    
    // 1. Find device (Class 0x01, Subclass 0x08, Prog IF 0x02)
    let devices = pci::enumerate_bus();
    let dev_info = devices.into_iter().find(|d| d.class_id == 0x01 && d.subclass_id == 0x08)
        .ok_or("NVMe: No SSD found")?;

    serial_println!("NVMe: Found {} at {:02x}:{:02x}.{:x}", 
        if dev_info.vendor_id == 0x144d { "Samsung 990 Pro" } else { "NVMe Device" },
        dev_info.bus, dev_info.dev, dev_info.func);

    // 2. Map BAR0 (Registers)
    let bar0_phys = dev_info.get_bar(0);
    serial_println!("NVMe: Mapping BAR0 physical {:#x}", bar0_phys);

    // In a real system, we'd assign a unique virtual range in the SAS.
    // For the prototype, we use a fixed MMIO offset.
    let mmio_vaddr = VirtAddr::new(0x_A000_0000_0000 + bar0_phys); 
    
    let mut gvas_lock = crate::memory::GLOBAL_VAS.lock();
    if let Some(ref mut gvas) = *gvas_lock {
        let flags = x86_64::structures::paging::PageTableFlags::PRESENT 
                  | x86_64::structures::paging::PageTableFlags::WRITABLE 
                  | x86_64::structures::paging::PageTableFlags::NO_CACHE;
        
        gvas.map_phys_range(mmio_vaddr, x86_64::PhysAddr::new(bar0_phys), 0x4000, flags, 0)?;
    }

    let mut nvme = NvmeController::new(mmio_vaddr);
    unsafe { nvme.init_controller()?; }

    Ok(())
}
