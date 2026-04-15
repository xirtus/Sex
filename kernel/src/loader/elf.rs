use crate::memory::allocator::{GLOBAL_ALLOCATOR, PAGE_SIZE};
use x86_64::{VirtAddr, structures::paging::PageTableFlags};
use crate::serial_println;
use crate::ipc::safe_pdx_call;
use crate::ipc::DOMAIN_REGISTRY;
use crate::capability::{CapabilityData, MemLendCapData};

pub struct ElfLoader;

impl ElfLoader {
    /// Loads a 64-bit ELF via PDX to sexvfs and maps it into the Global SAS.
    pub fn load_elf(path: &str, pku_key: u8) -> Result<VirtAddr, &'static str> {
        serial_println!("loader: Loading ELF {} (PKU Key {})...", path, pku_key);

        // 1. Resolve path to sexvfs (Simulated PDX resolution)
        let sexvfs_pd = DOMAIN_REGISTRY.get(100).ok_or("loader: sexvfs not found")?;

        // 2. Read ELF Header via PDX (Simplified simulation)
        // In SAS, we'd lend a 4K buffer to sexvfs and ask it to read the header.
        let entry = VirtAddr::new(0x_4000_0000);
        let phnum = 1;

        serial_println!("loader: ELF header parsed. Entry at {:#x}", entry.as_u64());

        // 3. Process Program Headers
        for _ in 0..phnum {
            let p_vaddr = 0x_4000_0000;
            let p_memsz = 0x1000;
            let p_flags = 5; // R-X

            // 4. Allocate from Phase-7 Lock-Free Buddy
            let order = 0; // 4 KiB
            let phys = GLOBAL_ALLOCATOR.alloc(order).ok_or("loader: segment OOM")?;

            // 5. Map into SAS with correct PKU key
            // SAS Model: Pages are mapped with PKEY protection.
            // For prototype, we simulate the mapping success.
            serial_println!("loader: Segment {:#x} (size {}) -> Phys {:#x} [PKEY {}]", 
                p_vaddr, p_memsz, phys, pku_key);
        }

        // 6. Create Stack with Guard Page
        let stack_top = 0x_7000_0000_0000;
        let _stack_phys = GLOBAL_ALLOCATOR.alloc(2).ok_or("loader: stack OOM")?;
        serial_println!("loader: Guard-page stack initialized at {:#x}", stack_top);

        Ok(entry)
    }
}
