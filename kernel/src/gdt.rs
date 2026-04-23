use crate::serial_println;
use x86_64::instructions::segmentation::{CS, Segment};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u8 = 0;

const IST_STACK_SIZE: usize = 16384;

#[allow(dead_code)]
#[repr(align(16))]
struct AlignedStack([u8; IST_STACK_SIZE]);
static mut IST_STACK: AlignedStack = AlignedStack([0; IST_STACK_SIZE]);

static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();
static mut TSS: TaskStateSegment = TaskStateSegment::new();

#[derive(Debug, Clone, Copy)]
pub struct Selectors {
    pub kernel_code: SegmentSelector, // 0x08
    pub kernel_data: SegmentSelector, // 0x10
    pub sysret_base: SegmentSelector, // 0x18
    pub user_data:   SegmentSelector, // 0x20
    pub user_code:   SegmentSelector, // 0x28
    pub tss:         SegmentSelector, // 0x30
}

static mut SELECTORS: Option<Selectors> = None;

pub unsafe fn init() {
    // IST[0] for double-fault / page-fault safety — never touched by PKU
    let stack_ptr = (&raw const IST_STACK).cast::<u8>();
    let stack_top = VirtAddr::from_ptr(stack_ptr) + IST_STACK_SIZE as u64;
    TSS.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = stack_top;

    let gdt = &mut *(&raw mut GDT);

    // Phase 25 Canonical Layout:
    // 0x00: Null
    // 0x08: Kernel Code
    // 0x10: Kernel Data
    // 0x18: SYSRET Base (User RPL3)
    // 0x20: User Data (User SS)
    // 0x28: User Code (User CS)
    // 0x30: TSS
    let kernel_code = gdt.append(Descriptor::kernel_code_segment());
    let kernel_data = gdt.append(Descriptor::kernel_data_segment());
    
    // We append three user segments to ensure the indices match Star::write expectations
    let mut sysret_base = gdt.append(Descriptor::user_code_segment());
    let mut user_data   = gdt.append(Descriptor::user_data_segment());
    let mut user_code   = gdt.append(Descriptor::user_code_segment());

    // Ensure RPL is set to 3 for user segments
    sysret_base = SegmentSelector::new(sysret_base.index(), x86_64::PrivilegeLevel::Ring3);
    user_data   = SegmentSelector::new(user_data.index(), x86_64::PrivilegeLevel::Ring3);
    user_code   = SegmentSelector::new(user_code.index(), x86_64::PrivilegeLevel::Ring3);

    let tss_ref: &'static TaskStateSegment = &*(&raw const TSS);
    let tss = gdt.append(Descriptor::tss_segment(tss_ref));

    SELECTORS = Some(Selectors {
        kernel_code,
        kernel_data,
        sysret_base,
        user_data,
        user_code,
        tss,
    });

    let gdt_ref: &'static GlobalDescriptorTable = &*(&raw const GDT);
    gdt_ref.load();
    CS::set_reg(kernel_code);

    serial_println!("[GDT] Phase 25 layout loaded — KCS=0x{:x} | SYSRET_BASE=0x{:x} | TSS=0x{:x}", 
                    kernel_code.0, sysret_base.0, tss.0);
}

pub unsafe fn init_tss_descriptor() {
    let selectors = (*(&raw const SELECTORS)).as_ref().expect("GDT selectors not initialized");
    load_tss(selectors.tss);
    serial_println!("[GDT] TSS loaded (selector 0x{:x}) — ring-3 PDX handoff now fully safe", selectors.tss.0);
}

pub fn get_selectors() -> Selectors {
    unsafe { SELECTORS.expect("GDT selectors not initialized") }
}
