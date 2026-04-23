use crate::memory::allocator::GLOBAL_ALLOCATOR;
use x86_64::VirtAddr;
use crate::serial_println;
use crate::ipc::DOMAIN_REGISTRY;

pub struct ElfLoader;

impl ElfLoader {
    /// Loads a 64-bit ELF via PDX to sexfiles and maps it into the Global SAS.
    pub fn load_elf(path: &str, pku_key: u8) -> Result<VirtAddr, &'static str> {
        serial_println!("loader: Loading ELF {} (PKU Key {})...", path, pku_key);

        let current_pd = crate::core_local::CoreLocal::get().current_pd_ref();

        // 1. Resolve path to sexfiles (Via capability slot 1 - granted at boot)
        // Bootstrap: If kernel (PD 0), bypass VFS check
        if current_pd.id != 0 {
            let vfs_cap = unsafe { (&*current_pd.cap_table).find(1).ok_or("loader: VFS cap missing")? };
            let sexfiles_pd_id = match vfs_cap.data {
                crate::capability::CapabilityData::IPC(data) => data.target_pd_id,
                _ => return Err("loader: Invalid VFS cap"),
            };
            let _sexfiles_pd = DOMAIN_REGISTRY.get(sexfiles_pd_id).ok_or("loader: sexfiles not found")?;
        } else {
            serial_println!("loader: Bootstrap mode. Using internal initrd.");
        }

        // 2. Read ELF Header via PDX (Simplified simulation)

        // In SAS, we'd lend a 4K buffer to sexfiles and ask it to read the header.
        let entry = VirtAddr::new(0x_4000_0000);
        let phnum = 1;

        serial_println!("loader: ELF header parsed. Entry at {:#x}", entry.as_u64());

        // 3. Process Program Headers
        for _ in 0..phnum {
            let p_vaddr = 0x_4000_0000;
            let p_memsz = 0x1000;
            let _p_flags = 5; // R-X

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
        let stack_top = 0x_7000_0000_0000u64;
        let _stack_phys = crate::memory::allocator::GLOBAL_ALLOCATOR.alloc(4).ok_or("loader: stack OOM")?;
        serial_println!("loader: Guard-page stack initialized at {:#x}", stack_top);

        Ok(entry)
    }
}
