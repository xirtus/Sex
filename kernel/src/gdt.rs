use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;
use lazy_static::lazy_static;
use core::arch::asm;
use crate::serial_println;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            let stack_end = stack_start + (STACK_SIZE as u64);
            stack_end
        };
        
        tss.privilege_stack_table[0] = {
            const KERNEL_STACK_SIZE: usize = 4096 * 5;
            static mut KERNEL_STACK: [u8; KERNEL_STACK_SIZE] = [0; KERNEL_STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(&raw const KERNEL_STACK);
            let stack_end = stack_start + (KERNEL_STACK_SIZE as u64);
            stack_end
        };
        
        tss
    };
}

#[derive(Debug, Clone, Copy)]
pub struct Selectors {
    pub code_selector: SegmentSelector,
    pub data_selector: SegmentSelector,
    pub kernel_data_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
    pub user_compat_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
}

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        // GDT layout (8-entry max including null at index 0):
        //   0x00 null (auto)
        //   0x08 kernel_code
        //   0x10 kernel_data  ← SS after SYSCALL, also STAR kernel base
        //   0x18 user_compat  ← STAR[63:48] base for SYSRET
        //   0x20 user_data    ← SS after SYSRET  (STAR base + 8)
        //   0x28 user_code    ← CS after SYSRET  (STAR base + 16)
        //   0x30+0x38 TSS     ← 16-byte system descriptor (2 slots)
        let code_selector       = gdt.append(Descriptor::kernel_code_segment());
        let data_selector       = gdt.append(Descriptor::kernel_data_segment());
        // kernel_data_selector aliases data_selector — no second append needed
        let user_compat_selector = gdt.append(Descriptor::user_code_segment());
        let user_data_selector  = gdt.append(Descriptor::user_data_segment());
        let user_code_selector  = gdt.append(Descriptor::user_code_segment());
        let tss_selector        = gdt.append(Descriptor::tss_segment(&TSS));

        (
            gdt,
            Selectors {
                code_selector,
                data_selector,
                kernel_data_selector: data_selector,
                tss_selector,
                user_compat_selector,
                user_data_selector,
                user_code_selector,
            },
        )
    };
}

pub fn init() {
    serial_println!("X86Hal: Initializing GDT/IDT...");

    // 1. Interrupts STRICTLY disabled for the entire phase
    x86_64::instructions::interrupts::disable();
    serial_println!("   → cli held — interrupts disabled");

    let hhdm_offset = crate::HHDM_REQUEST.response().map(|r| r.offset).unwrap_or(0);
    serial_println!("   → HHDM offset = {:#x}", hhdm_offset);

    // 2. GDT — Ensure higher-half virtual address
    let gdt_ptr = &GDT.0 as *const _ as u64;
    serial_println!("   → GDT virtual base = {:#x}", gdt_ptr);

    unsafe {
        serial_println!("   → Loading GDTR...");
        GDT.0.load();
        serial_println!("   → GDTR loaded successfully");

        // 3. Mandatory far jump to reload CS in higher-half
        serial_println!("   → Performing far jump to reload CS (higher-half)...");
        let cs_selector = GDT.1.code_selector.0 as u64;
        asm!(
            "push {0}",
            "lea rax, [rip + 2f]",
            "push rax",
            "retfq",
            "2:",
            in(reg) cs_selector,
        );
        serial_println!("   → CS segment reloaded — now executing in higher-half");

        // 4. TSS load (safe after CS reload)
        x86_64::instructions::tables::load_tss(GDT.1.tss_selector);
    }
    serial_println!("   → TSS loaded");
}

pub fn get_selectors() -> Selectors {
    GDT.1
}
